// #![allow(unexpected_cfgs)]
use pinocchio::{
    address::declare_id, error::ProgramError,
    program_entrypoint, AccountView, Address, ProgramResult,
};

use crate::instruction::FundInstruction;
mod constant;
mod error;
mod instruction;
// mod state;

mod tests;
program_entrypoint!(process_instruction);

// entrypoint!(process_instruction);

declare_id!("9piQZir4QXTh76Xt9HwVSFtisTud8paBcWWeea6qCJxS");
// pub static ID: [u8; 32] = [
//     18, 52, 92, 250, 203, 184, 161, 154, 76, 145, 117, 30, 211, 230, 201, 41, 85, 198, 73, 233,
//     165, 146, 89, 117, 17, 211, 236, 80, 246, 118, 46, 155,
// ];

#[inline(always)]
pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    // assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundInstruction::try_from(discriminator)? {
        FundInstruction::Initialize => instruction::process_initialize_instruction(accounts, data)?,
        FundInstruction::Contribute => instruction::process_contribute_instruction(accounts, data)?,
        FundInstruction::Checker => instruction::process_checker_instruction(accounts, data)?,
        FundInstruction::Refund => instruction::process_refund_instruction(accounts, data)?,
        // _=> return Err(ProgramError::InvalidInstructionData),
    }
    Ok(())
}
