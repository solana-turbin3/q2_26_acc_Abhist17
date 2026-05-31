use anchor_lang::prelude::*;
use solana_gpt_oracle::Identity;

use crate::state::GptScheduler;

#[derive(Accounts)]
pub struct CallbackFromGpt<'info> {
    /// CHECK: Identity from oracle - must be signer
    pub identity: Account<'info, Identity>,

    #[account(
        mut,
        seeds = [b"gpt_scheduler"],
        bump
    )]
    pub gpt_scheduler: Account<'info, GptScheduler>,
}

pub fn process_callback_from_gpt(
    ctx: Context<CallbackFromGpt>,
    response: String,
) -> Result<()> {
    // Verify callback is from oracle
    require!(
        ctx.accounts.identity.to_account_info().is_signer,
        crate::error::GptSchedulerError::UnauthorizedCallback
    );

    let gpt_scheduler = &mut ctx.accounts.gpt_scheduler;
    
    // Store response
    gpt_scheduler.last_response = response.clone();

    msg!("==============================================");
    msg!("GPT RESPONSE RECEIVED (Query #{})", gpt_scheduler.query_count);
    msg!("==============================================");
    msg!("Query: {}", gpt_scheduler.query);
    msg!("Response: {}", response);
    msg!("==============================================");

    Ok(())
}