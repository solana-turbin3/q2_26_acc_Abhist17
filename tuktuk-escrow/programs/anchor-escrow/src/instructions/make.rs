use anchor_lang::{InstructionData, prelude::*};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use solana_instruction::Instruction;
use tuktuk_program::tuktuk::program::Tuktuk;
// use solana_instruction::Instruction;
use tuktuk_program::{
    compile_transaction,
    tuktuk::cpi::{accounts::QueueTaskV0, queue_task_v0},
    types::QueueTaskArgsV0,
    TransactionSourceV0, TriggerV0,
};

use crate::state::Escrow;

#[derive(Accounts)]
#[instruction(task_id: u16, seed: u64)]
pub struct Make<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,
    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
    )]
    pub maker_ata_a: InterfaceAccount<'info, TokenAccount>,
    #[account(
        init,
        payer = maker,
        seeds = [b"escrow", maker.key().as_ref(), seed.to_le_bytes().as_ref()],
        bump,
        space = 8 + Escrow::INIT_SPACE,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
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

impl<'info> Make<'info> {
    pub fn init_escrow(&mut self, seed: u64, receive: u64, bumps: &MakeBumps) -> Result<()> {
        //to get the current tiem. in unix timestamp;
        let time = Clock::get()?.unix_timestamp;

        self.escrow.set_inner(Escrow {
            seed,
            maker: self.maker.key(),
            mint_a: self.mint_a.key(),
            mint_b: self.mint_b.key(),
            receive,
            bump: bumps.escrow,
            created_at: time,
        });

        Ok(())
    }

    pub fn deposit(&mut self, deposit: u64) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = TransferChecked {
            from: self.maker_ata_a.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
            mint: self.mint_a.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer_checked(cpi_ctx, deposit, self.mint_a.decimals)?;

        Ok(())
    }
    pub fn schedule(&mut self, task_id: u16, bump: &MakeBumps) -> Result<()> {
        let current_time = Clock::get()?.unix_timestamp;
        // let duration = 7;
        let duration =  100 * 24 * 60 * 60; //100days
        let timeleft = duration - (current_time - self.escrow.created_at);

        if timeleft > 0 {
            let (compiled_tx, _) = compile_transaction(
                vec![Instruction {
                    program_id: crate::ID,
                    accounts: crate::__cpi_client_accounts_refund::Refund {
                       
                        maker: self.maker.to_account_info(),
                        mint_a: self.mint_a.to_account_info(),
                        maker_ata_a:self.maker_ata_a.to_account_info(),
                        escrow: self.escrow.to_account_info(),
                        vault: self.vault.to_account_info(),
                        token_program: self.token_program.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                        
                    }
                    .to_account_metas(Some(true))
                    .to_vec(),
                    data: crate::instruction::Refund{}.data()
                }],
                vec![],
            )
            .unwrap();

            queue_task_v0(
                CpiContext::new_with_signer(
                    self.tuktuk_program.to_account_info(),
                    QueueTaskV0 {
                        payer: self.maker.to_account_info(),
                        queue_authority: self.queue_authority.to_account_info(),
                        task_queue: self.task_queue.to_account_info(),
                        task_queue_authority: self.task_queue_authority.to_account_info(),
                        task: self.task.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                    },
                    &[&["queue_authority".as_bytes(), &[bump.queue_authority]]],
                ),
                QueueTaskArgsV0 {
                    trigger: TriggerV0::Timestamp(timeleft + current_time),
                    transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                    crank_reward: Some(1000002),
                    free_tasks: 1,
                    id: task_id,
                    description: "test".to_string(),
                },
            )?;


        }

        Ok(())
    }
}
