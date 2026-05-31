#![allow(unexpected_cfgs, deprecated)]
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("GayQidtYDDM43ee5d2vUo5w4LpAbCKNuuyCUrJLRBpSh");

#[program]
pub mod tuktuk_gpt_oracle {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.init_llm(&ctx.bumps)
    }

    pub fn analyze_user(
        ctx: Context<AnalyzeUser>,
        user_pubkey: Pubkey,
        user_data: String,
    ) -> Result<()> {
        ctx.accounts.analyse_user(user_pubkey, user_data, &ctx.bumps)
    }

    pub fn callback_from_agent(ctx: Context<CallbackFromAgent>, response: String) -> Result<()> {
        ctx.accounts.callback_from_agent(response, &ctx.bumps)
    }

    // this ix is to only forward the result from the llm to the frontend
    pub fn get_analysis(ctx: Context<GetAnalysis>) -> Result<String> {
        ctx.accounts.get_analysis()
    }
    
    pub fn schedule(ctx: Context<Schedule>, user_pubkey: Pubkey, user_data: String,task_id: u16, seed: u64) -> Result<()> {
        ctx.accounts.schedule(user_pubkey,user_data,task_id,seed, &ctx.bumps)
    }
}
