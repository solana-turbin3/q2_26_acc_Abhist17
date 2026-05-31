use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::token_interface::Token2022;
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

#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated by tuktuk program
    pub task_queue: UncheckedAccount<'info>,

    /// CHECK: Validated by tuktuk program
    pub task_queue_authority: UncheckedAccount<'info>,

    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,

    /// CHECK: PDA used as queue authority for tuktuk
    #[account(
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: UncheckedAccount<'info>,

    /// CHECK: PDA that is the fee withdraw authority
    #[account(
        seeds = [b"fee_authority"],
        bump
    )]
    pub fee_authority: UncheckedAccount<'info>,

    /// CHECK: Mint account for the token
    pub mint_account: UncheckedAccount<'info>,

    /// CHECK: Treasury token account where fees will be deposited
    pub treasury_token_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,

    pub tuktuk_program: Program<'info, Tuktuk>,
}

pub fn process_schedule<'info>(
    ctx: Context<'_, '_, 'info, 'info, Schedule<'info>>,
    task_id: u16,
) -> Result<()> {
    // Build account metas for ManualCollect instruction
    let mut account_metas = vec![
        AccountMeta::new_readonly(ctx.accounts.fee_authority.key(), false),
        AccountMeta::new(ctx.accounts.mint_account.key(), false),
        AccountMeta::new(ctx.accounts.treasury_token_account.key(), false),
        AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
    ];

    // Add source token accounts from remaining_accounts
    // These are the accounts from which we'll harvest fees
    for source in ctx.remaining_accounts.iter() {
        account_metas.push(AccountMeta::new(source.key(), false));
    }

    msg!("Scheduling fee collection for {} source accounts", ctx.remaining_accounts.len());

    // Compile the ManualCollect instruction
    let (compiled_tx, _) = compile_transaction(
        vec![Instruction {
            program_id: crate::ID,
            accounts: account_metas,
            data: anchor_lang::InstructionData::data(&crate::instruction::ManualCollect {}),
        }],
        vec![], // No external signers needed - PDA signs internally
    )
    .map_err(|_| error!(crate::error::FeeCollectorError::CompileTransactionFailed))?;

    let bump = ctx.bumps.queue_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"queue_authority", &[bump]]];

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
            crank_reward: Some(1_000_000), // 0.001 SOL reward for cranker
            free_tasks: 0,
            id: task_id,
            description: "Automated fee collection".to_string(),
        },
    )?;

    msg!("Fee collection task scheduled with ID: {}", task_id);

    Ok(())
}