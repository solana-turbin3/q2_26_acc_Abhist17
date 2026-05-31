use anchor_lang::prelude::*;

#[error_code]
pub enum FeeCollectorError {
    #[msg("Failed to compile transaction")]
    CompileTransactionFailed,
    #[msg("Invalid source account")]
    InvalidSourceAccount,
    #[msg("No fees to collect")]
    NoFeesToCollect,
}