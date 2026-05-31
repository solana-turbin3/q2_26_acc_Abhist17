use anchor_lang::prelude::*;
use solana_gpt_oracle::Counter;

use crate::state::GptScheduler;

const AGENT_DESC: &str = "You are a helpful Solana blockchain assistant. Answer concisely.";

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32 + 4 + 200 + 4 + 500 + 4, // Fixed size
        seeds = [b"gpt_scheduler"],
        bump
    )]
    pub gpt_scheduler: Account<'info, GptScheduler>,

    #[account(mut)]
    pub counter: Account<'info, Counter>,

    /// CHECK: Checked in oracle program
    #[account(mut)]
    pub llm_context: AccountInfo<'info>,

    /// CHECK: Checked oracle id
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn process_initialize(ctx: Context<Initialize>) -> Result<()> {
    let gpt_scheduler = &mut ctx.accounts.gpt_scheduler;
    
    gpt_scheduler.context = ctx.accounts.llm_context.key();
    gpt_scheduler.authority = ctx.accounts.payer.key();
    gpt_scheduler.query = AGENT_DESC.to_string();
    gpt_scheduler.last_response = String::new();
    gpt_scheduler.query_count = 0;

    // CPI to create context
    let cpi_program = ctx.accounts.oracle_program.to_account_info();
    let cpi_accounts = solana_gpt_oracle::cpi::accounts::CreateLlmContext {
        payer: ctx.accounts.payer.to_account_info(),
        counter: ctx.accounts.counter.to_account_info(),
        context_account: ctx.accounts.llm_context.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    
    solana_gpt_oracle::cpi::create_llm_context(cpi_ctx, AGENT_DESC.to_string())?;

    msg!("GPT Scheduler initialized");

    Ok(())
}