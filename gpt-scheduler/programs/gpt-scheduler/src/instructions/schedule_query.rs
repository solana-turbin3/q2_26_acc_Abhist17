use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use tuktuk_program::{
    compile_transaction,
    tuktuk::{
        cpi::{accounts::QueueTaskV0, queue_task_v0},
        program::Tuktuk,
        types::TriggerV0,
    },
    types::QueueTaskArgsV0,
    TransactionSourceV0,
};

use crate::error::GptSchedulerError;
use crate::state::GptScheduler;

#[derive(Accounts)]
pub struct ScheduleQuery<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        seeds = [b"gpt_scheduler"],
        bump
    )]
    pub gpt_scheduler: Account<'info, GptScheduler>,

    /// CHECK: Validated by tuktuk
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,

    /// CHECK: Validated by tuktuk
    pub task_queue_authority: UncheckedAccount<'info>,

    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,

    /// CHECK: PDA for signing
    #[account(
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: UncheckedAccount<'info>,

    /// CHECK: Will be passed to query_gpt
    #[account(mut)]
    pub interaction: UncheckedAccount<'info>,

    /// CHECK: GPT context account
    #[account(address = gpt_scheduler.context)]
    pub context_account: UncheckedAccount<'info>,

    /// CHECK: Oracle program
    #[account(address = solana_gpt_oracle::ID)]
    pub oracle_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    pub tuktuk_program: Program<'info, Tuktuk>,
}

pub fn process_schedule_query<'info>(
    ctx: Context<'_, '_, 'info, 'info, ScheduleQuery<'info>>,
    task_id: u16,
) -> Result<()> {
    msg!("Scheduling GPT query with task ID: {}", task_id);

    // Build the QueryGpt instruction discriminator
    let discriminator = crate::instruction::QueryGpt::DISCRIMINATOR;

    // Build account metas for QueryGpt instruction
    let account_metas = vec![
        AccountMeta::new(ctx.accounts.payer.key(), true), // payer (will be cranker when executed)
        AccountMeta::new(ctx.accounts.gpt_scheduler.key(), false),
        AccountMeta::new(ctx.accounts.interaction.key(), false),
        AccountMeta::new_readonly(ctx.accounts.context_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
        AccountMeta::new_readonly(ctx.accounts.oracle_program.key(), false),
    ];

    let query_ix = Instruction {
        program_id: crate::ID,
        accounts: account_metas,
        data: discriminator.to_vec(),
    };

    // Compile transaction
    let (compiled_tx, _) = compile_transaction(vec![query_ix], vec![])
        .map_err(|_| GptSchedulerError::CompileTransactionFailed)?;

    let bump = ctx.bumps.queue_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"queue_authority", &[bump]]];

    // Queue task with TukTuk
    queue_task_v0(
        CpiContext::new_with_signer(
            ctx.accounts.tuktuk_program.to_account_info(),
            QueueTaskV0 {
                payer: ctx.accounts.payer.to_account_info(),
                queue_authority: ctx.accounts.queue_authority.to_account_info(),
                task_queue: ctx.accounts.task_queue.to_account_info(),
                task_queue_authority: ctx.accounts.task_queue_authority.to_account_info(),
                task: ctx.accounts.task.to_account_info(),
                system_program: ctx.accounts.system_program.to_account_info(),
            },
            signer_seeds,
        ),
        QueueTaskArgsV0 {
            trigger: TriggerV0::Now,
            transaction: TransactionSourceV0::CompiledV0(compiled_tx),
            crank_reward: Some(1_000_000), // 0.001 SOL
            free_tasks: 0,
            id: task_id,
            description: "Scheduled GPT query".to_string(),
        },
    )?;

    msg!("GPT query scheduled successfully");

    Ok(())
}