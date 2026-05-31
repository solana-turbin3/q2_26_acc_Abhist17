use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError,
};
use pinocchio_pubkey::derive_address;

use crate::state::Escrow;

pub fn process_cancel_instruction(
    accounts: &[AccountView],
    _data: &[u8],
) -> ProgramResult {
    let [
        maker,
        mint_a,
        escrow_account,
        escrow_ata,
        maker_ata,
        token_program,
        _system_program @ ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify maker is signer
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Verify escrow account ownership
    unsafe {
        if escrow_account.owner() != &crate::ID {
            return Err(ProgramError::IllegalOwner);
        }
    }

    let bump;
    {
        let escrow_state = Escrow::from_account_info(escrow_account)?;

        if escrow_state.maker().as_ref() != maker.address().as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }

        if escrow_state.mint_a().as_ref() != mint_a.address().as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }

        bump = escrow_state.bump;
    }
    
    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    if escrow_pda != *escrow_account.address().as_array() {
        return Err(ProgramError::InvalidSeeds);
    }

    // Verify maker's ATA
    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(&maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    let vault_balance;
    {
        let escrow_ata_state =
            pinocchio_token::state::TokenAccount::from_account_view(&escrow_ata)?;
        vault_balance = escrow_ata_state.amount();
    }

    // Create signer seeds for escrow PDA
    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let signer = Signer::from(&signer_seeds);

    // Transfer tokens back from escrow vault to maker
    if vault_balance > 0 {
        pinocchio_token::instructions::Transfer {
            from: escrow_ata,
            to: maker_ata,
            authority: escrow_account,
            amount: vault_balance,
        }.invoke_signed(&[signer.clone()])?;
    }

    // Close the escrow vault token account
    pinocchio_token::instructions::CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
    }.invoke_signed(&[signer.clone()])?;

    // Close the escrow account and return lamports to maker
    maker.set_lamports(maker.lamports() + escrow_account.lamports());
    escrow_account.set_lamports(0);
    escrow_account.resize(0);

    Ok(())
}