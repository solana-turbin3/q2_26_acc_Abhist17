use crate::{error::ErrorCode, state::AnalysisResult};
use anchor_lang::prelude::*;
use solana_gpt_oracle::Identity;

#[derive(Accounts)]
pub struct CallbackFromAgent<'info> {
    #[account(
        mut,
        seeds = [b"analysis", user_pubkey.key().as_ref()],
        bump
    )]
    pub analysis_result: Account<'info, AnalysisResult>,

    /// CHECK: User pubkey we analyzed
    pub user_pubkey: AccountInfo<'info>,

    /// CHECK: Oracle program identity
    pub identity: Account<'info, Identity>,

    /// CHECK: Oracle program that will call this callback
    #[account(
        address = solana_gpt_oracle::ID
    )]
    pub oracle_program: AccountInfo<'info>,
}

impl<'info> CallbackFromAgent<'info> {
    pub fn callback_from_agent(
        &mut self,
        response: String,
        bumps: &CallbackFromAgentBumps,
    ) -> Result<()> {
        if !self.identity.to_account_info().is_signer {
            return Err(ErrorCode::InvalidOracleCallback.into());
        }

        // storing the analysis result
        self.analysis_result.set_inner(AnalysisResult {
            user: self.user_pubkey.key(),
            analysis: response,
            timestamp: Clock::get()?.unix_timestamp,
            bump: bumps.analysis_result,
        });

        Ok(())
    }
}