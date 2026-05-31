use anchor_lang::prelude::instruction::Instruction;
use anchor_lang::{InstructionData, prelude::*};


use solana_gpt_oracle::ContextAccount;
use solana_gpt_oracle::program::SolanaGptOracle;
use tuktuk_program::tuktuk::program::Tuktuk;
// use solana_instruction::Instruction;
use tuktuk_program::{
    compile_transaction,
    tuktuk::cpi::{accounts::QueueTaskV0, queue_task_v0},
    types::QueueTaskArgsV0,
    TransactionSourceV0, TriggerV0,
};

use crate::{ANCHOR_DISCRIMINATOR, Agent, AnalysisResult, ID};


#[derive(Accounts)]
#[instruction(user_pubkey: Pubkey, user_data: String,task_id: u16, seed: u64)]
pub struct Schedule<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: Correct interaction account
    #[account(
        mut,
        seeds = [solana_gpt_oracle::Interaction::seed(), payer.key().as_ref(), context_account.key().as_ref()],
        bump,
        seeds::program = oracle_program
    )]
    pub interaction: AccountInfo<'info>,

    #[account(
        seeds = [b"agent"],
        bump = agent.bump
    )]
    pub agent: Account<'info, Agent>,

    /// CHECK: Accept any context
    pub context_account: Account<'info, ContextAccount>,

    /// CHECK: Verified oracle id
    #[account(
        address = ID
    )]
    pub oracle_program: Program<'info, SolanaGptOracle>,

    /// Analysis result account - initialized here so callback can update it
    #[account(
        init_if_needed,
        payer = payer,
        space = ANCHOR_DISCRIMINATOR + AnalysisResult::INIT_SPACE,
        seeds = [b"analysis", user_pubkey.as_ref()],
        bump
    )]
    pub analysis_result: Account<'info, AnalysisResult>,

    pub system_program: Program<'info, System>,
   
    
    
    /// CHECK: Don't need to parse this account, just using it in CPI
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,
    
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue_authority: UncheckedAccount<'info>,
    
    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: AccountInfo<'info>,
    
    
    /// CHECK: Via seeds
    #[account(
            mut,
            seeds = [b"queue_authority"],
            bump
        )]
    pub queue_authority: AccountInfo<'info>,
    
    
    pub tuktuk_program: Program<'info, Tuktuk>,
}

impl<'info> Schedule<'info> {
    pub fn schedule(&mut self, user_pubkey: Pubkey, user_data: String,task_id: u16, seed: u64, bump: &ScheduleBumps) -> Result<()> {
      
            let (compiled_tx, _) = compile_transaction(
                vec![Instruction {
                    program_id: crate::ID,
                    accounts: crate::__cpi_client_accounts_analyze_user::AnalyzeUser {
                       payer:self.payer.to_account_info(),
                       interaction:self.interaction.to_account_info(),
                       agent:self.agent.to_account_info(),
                       context_account:self.context_account.to_account_info(),
                       oracle_program:self.oracle_program.to_account_info(),
                       system_program:self.system_program.to_account_info(),
                       analysis_result:self.analysis_result.to_account_info(),
                       
                       
                       
                       
                        
                    }
                    .to_account_metas(Some(true))
                    .to_vec(),
                    data: crate::instruction::AnalyzeUser{
                        user_pubkey,
                        user_data,
                    }.data()
                }],
                vec![],
            )
            .unwrap();

            queue_task_v0(
                CpiContext::new_with_signer(
                    self.tuktuk_program.to_account_info(),
                    QueueTaskV0 {
                        payer: self.payer.to_account_info(),
                        queue_authority: self.queue_authority.to_account_info(),
                        task_queue: self.task_queue.to_account_info(),
                        task_queue_authority: self.task_queue_authority.to_account_info(),
                        task: self.task.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                    },
                    &[&["queue_authority".as_bytes(), &[bump.queue_authority]]],
                ),
                QueueTaskArgsV0 {
                    trigger: TriggerV0::Now,
                    transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                    crank_reward: Some(1000002),
                    free_tasks: 1,
                    id: task_id,
                    description: "test".to_string(),
                },
            )?;

        Ok(())
    }
}
