use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError,
    sysvars::{Sysvar, rent::Rent},
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;
use wincode::SchemaRead;

use crate::state::EscrowV2;

#[derive(SchemaRead)]
pub struct MakeV2InstructionData {
    pub amount_to_receive: u64,
    pub amount_to_give: u64,
    pub bump: u8,
}

pub fn process_make_instruction_v2(
    accounts: &[AccountView],
    data: &[u8],
) -> ProgramResult {
    let [
        maker,
        mint_a,
        mint_b,
        escrow_account,
        maker_ata,
        escrow_ata,
        system_program,
        token_program,
        _associated_token_program @ ..
    ] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let ix_data = wincode::deserialize::<MakeV2InstructionData>(data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    let amount_to_receive = ix_data.amount_to_receive;
    let amount_to_give = ix_data.amount_to_give;
    let bump = ix_data.bump;

    // Validate maker's ATA in a scope so borrow is released
    {
        let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_view(&maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_a.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Verify PDA
    let seed = [b"escrow".as_ref(), maker.address().as_ref(), &[bump]];
    let escrow_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(escrow_account_pda, *escrow_account.address().as_array());

    let bump_bytes = [bump.to_le()];
    let signer_seeds = [
        Seed::from(b"escrow"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump_bytes),
    ];
    let seeds = Signer::from(&signer_seeds);

    unsafe {
        if escrow_account.owner() != &crate::ID {
            CreateAccount {
                from: maker,
                to: escrow_account,
                lamports: Rent::get()?.try_minimum_balance(EscrowV2::LEN)?,
                space: EscrowV2::LEN as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[seeds.clone()])?;

            // Build state on stack and write — borrow acquired and released inside write_to
            let escrow_state = EscrowV2 {
                maker: *maker.address().as_array(),
                mint_a: *mint_a.address().as_array(),
                mint_b: *mint_b.address().as_array(),
                amount_to_receive,
                amount_to_give,
                bump,
            };
            escrow_state.write_to(escrow_account)?;
        } else {
            return Err(ProgramError::IllegalOwner);
        }
    }

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: escrow_ata,
        wallet: escrow_account,
        mint: mint_a,
        token_program,
        system_program,
    }
    .invoke()?;

    pinocchio_token::instructions::Transfer {
        from: maker_ata,
        to: escrow_ata,
        authority: maker,
        amount: amount_to_give,
    }
    .invoke()?;

    Ok(())
}