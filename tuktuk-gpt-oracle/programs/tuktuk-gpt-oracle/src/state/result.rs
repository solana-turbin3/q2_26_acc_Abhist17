use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AnalysisResult {
    pub user: Pubkey,
    #[max_len(500)]
    pub analysis: String,
    pub timestamp: i64,
    pub bump: u8,
}