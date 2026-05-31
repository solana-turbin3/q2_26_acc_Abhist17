use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid oracle callback")]
    InvalidOracleCallback,
    #[msg("Analysis not found")]
    AnalysisNotFound,
    #[msg("Invalid user pubkey")]
    InvalidUserPubkey,
    #[msg("Oracle program not authorized")]
    UnauthorizedOracle,
}