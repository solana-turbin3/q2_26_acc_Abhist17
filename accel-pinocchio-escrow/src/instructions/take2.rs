use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError,
};
use pinocchio_pubkey::derive_address;

use crate::state::EscrowV2;

pub fn process_take_instruction_v2(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        taker,
        maker,
        mint_a,
        mint_b,
        escrow_account,
        escrow_ata,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        token_program,
        _system_program @ ..
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !taker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    unsafe {
        if escrow_account.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
    }

    // Deserialize escrow state — borrow acquired and released inside read_from
    let escrow_state = EscrowV2::read_from(escrow_account)?;

    // Verify maker
    if escrow_state.maker.as_ref() != maker.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify mints
    if escrow_state.mint_a.as_ref() != mint_a.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }
    if escrow_state.mint_b.as_ref() != mint_b.address().as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify PDA
    let bump = escrow_state.bump;
    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    if escrow_pda != *escrow_account.address().as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    let amount_to_receive = escrow_state.amount_to_receive;
    let amount_to_give = escrow_state.amount_to_give;

    // Validate taker's ATA for mint_b in a scope
    {
        let taker_ata_b_state =
            pinocchio_token::state::TokenAccount::from_account_view(&taker_ata_b)?;
        if taker_ata_b_state.owner() != taker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if taker_ata_b_state.mint() != mint_b.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Transfer mint_b from taker to maker
    pinocchio_token::instructions::Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
    }
    .invoke()?;

    // Create signer seeds for escrow PDA
    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seeds);

    // Transfer mint_a from escrow vault to taker
    pinocchio_token::instructions::Transfer {
        from: escrow_ata,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
    }
    .invoke_signed(&[signer.clone()])?;

    // Close vault token account
    pinocchio_token::instructions::CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
    }
    .invoke_signed(&[signer.clone()])?;

    // Close escrow account — return lamports to maker
    maker.set_lamports(maker.lamports() + escrow_account.lamports());
    escrow_account.set_lamports(0);
    escrow_account.resize(0);

    Ok(())
}