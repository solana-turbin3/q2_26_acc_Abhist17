use anchor_lang::prelude::*;

#[error_code]
pub enum GptSchedulerError {
    #[msg("Failed to compile transaction")]
    CompileTransactionFailed,
    #[msg("Unauthorized callback")]
    UnauthorizedCallback,
}