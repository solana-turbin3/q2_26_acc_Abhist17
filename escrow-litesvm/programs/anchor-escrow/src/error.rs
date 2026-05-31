use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Cannot take escrow before 5 days have passed")]
    TooEarlyToTake,
}