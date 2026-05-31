use crate::state::AnalysisResult;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct GetAnalysis<'info> {
    #[account(
        seeds = [b"analysis", user_pubkey.key().as_ref()],
        bump
    )]
    pub analysis_result: Account<'info, AnalysisResult>,

    /// CHECK: This is the user pubkey we want to analyze
    pub user_pubkey: AccountInfo<'info>,
}

impl<'info> GetAnalysis<'info> {
    pub fn get_analysis(&mut self) -> Result<String> {
        Ok(self.analysis_result.analysis.clone())
    }
}