use crate::{state::Agent, ANCHOR_DISCRIMINATOR};
use anchor_lang::prelude::*;
use solana_gpt_oracle::{Counter, ID};

const AGENT_DESC: &str = "You are a Solana transaction analyst. 
Given a Solana public key, analyze the user's recent transaction activity 
and provide a clear summary in plain English. Focus on:
- Recent DeFi activities (swaps, staking, lending)
- Token movements and balances
- Spending patterns
- Notable transactions

Keep response concise (2-3 sentences max).";

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        space = ANCHOR_DISCRIMINATOR + Agent::INIT_SPACE,
        seeds = [b"agent"],
        bump
    )]
    pub agent: Account<'info, Agent>,

    /// CHECK: This is the LLM context account created by the oracle program
    #[account(mut)]
    pub llm_context: AccountInfo<'info>,

    #[account(mut)]
    pub counter: Account<'info, Counter>,

    pub system_program: Program<'info, System>,

    /// CHECK: Checked oracle id
    #[account(
        address = ID
    )]
    pub oracle_program: AccountInfo<'info>,
}

impl<'info> Initialize<'info> {
    pub fn init_llm(&mut self, bumps: &InitializeBumps) -> Result<()> {
        // creating context for AI Agent
        let cpi_program = self.oracle_program.to_account_info();

        let cpi_accounts = solana_gpt_oracle::cpi::accounts::CreateLlmContext {
            payer: self.payer.to_account_info(),
            context_account: self.llm_context.to_account_info(),
            counter: self.counter.to_account_info(),
            system_program: self.system_program.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        solana_gpt_oracle::cpi::create_llm_context(cpi_ctx, AGENT_DESC.to_string())?;

        //storing context in our agent
        self.agent.set_inner(Agent {
            context: self.llm_context.key(),
            bump: bumps.agent,
        });

        Ok(())
    }
}