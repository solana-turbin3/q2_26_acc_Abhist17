use anchor_lang::prelude::*;
use anchor_lang::Discriminator;

use crate::state::GptScheduler;

#[derive(Accounts)]
pub struct QueryGpt<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"gpt_scheduler"],
        bump
    )]
    pub gpt_scheduler: Account<'info, GptScheduler>,

    /// CHECK: Interaction account for oracle
    #[account(mut)]
    pub interaction: UncheckedAccount<'info>,

    /// CHECK: Context account - just need the pubkey to match
    #[account(address = gpt_scheduler.context)]
    pub context_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: Oracle program
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: UncheckedAccount<'info>,
}

pub fn process_query_gpt(ctx: Context<QueryGpt>) -> Result<()> {
    let gpt_scheduler = &mut ctx.accounts.gpt_scheduler;
    
    gpt_scheduler.query_count += 1;

    msg!("Querying GPT (query #{})", gpt_scheduler.query_count);
    msg!("Query: {}", gpt_scheduler.query);

    let callback_disc = crate::instruction::CallbackFromGpt::DISCRIMINATOR.try_into().expect("Discriminator must be 8 bytes");

    // CPI to interact with LLM
    let cpi_program = ctx.accounts.oracle_program.to_account_info();
    let cpi_accounts = solana_gpt_oracle::cpi::accounts::InteractWithLlm {
        payer: ctx.accounts.payer.to_account_info(),
        interaction: ctx.accounts.interaction.to_account_info(),
        context_account: ctx.accounts.context_account.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    solana_gpt_oracle::cpi::interact_with_llm(
        cpi_ctx,
        gpt_scheduler.query.clone(),
        crate::ID,
        callback_disc,
        None,
    )?;

    msg!("Query sent to oracle");

    Ok(())
}