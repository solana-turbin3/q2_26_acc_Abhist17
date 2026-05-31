#![allow(unexpected_cfgs)]
use pinocchio::{AccountView, entrypoint, Address, ProgramResult, address::declare_id, error::ProgramError};

use crate::instructions::EscrowInstructions;

mod tests;
mod state;
mod instructions;

entrypoint!(process_instruction);

declare_id!("4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT");

pub fn process_instruction(
    program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data.split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match EscrowInstructions::try_from(discriminator)? {
        EscrowInstructions::Make => instructions::process_make_instruction(accounts, data)?,
        EscrowInstructions::Take => instructions::process_take_instruction(accounts, data)?,
        EscrowInstructions::Cancel => instructions::process_cancel_instruction(accounts, data)?,
        EscrowInstructions::MakeV2 => instructions::process_make_instruction_v2(accounts, data)?,
        EscrowInstructions::TakeV2 => instructions::process_take_instruction_v2(accounts, data)?,
        EscrowInstructions::CancelV2 => instructions::process_cancel_instruction_v2(accounts, data)?,
    }
    Ok(())
}