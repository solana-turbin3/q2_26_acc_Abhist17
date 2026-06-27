pub mod checker;
pub mod contribute;
pub mod initialize;
pub mod refund;

pub use checker::*;
pub use contribute::*;
pub use initialize::*;
pub use refund::*;





use pinocchio::error::ProgramError;

pub enum FundInstruction {
    Initialize = 0,
    Contribute = 1,
    Checker = 2,
    Refund = 3,

}

impl TryFrom<&u8> for FundInstruction {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FundInstruction::Initialize),
            1 => Ok(FundInstruction::Contribute),
            2 => Ok(FundInstruction::Checker),
            3 => Ok(FundInstruction::Refund),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}