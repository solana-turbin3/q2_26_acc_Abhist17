#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod state;
mod instructions;

use instructions::*;

declare_id!("2xJpANPcNaCnCa3irM4x8vNMxjY2S48XLRJZZndb8u4L");

#[ephemeral]
#[program]
pub mod er_state_account {

    use super::*;

    pub fn initialize(ctx: Context<InitUser>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;
        
        Ok(())
    }

    pub fn update(ctx: Context<UpdateUser>, new_data: u64) -> Result<()> {
        ctx.accounts.update(new_data)?;
        
        Ok(())
    }

    pub fn update_commit(ctx: Context<UpdateCommit>, new_data: u64) -> Result<()> {
        ctx.accounts.update_commit(new_data)?;
        
        Ok(())
    }

    pub fn delegate(ctx: Context<Delegate>) -> Result<()> {
        ctx.accounts.delegate()?;
        
        Ok(())
    }

    pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
        ctx.accounts.undelegate()?;
        
        Ok(())
    }

    pub fn close(ctx: Context<CloseUser>) -> Result<()> {
        ctx.accounts.close()?;
        
        Ok(())
    }

    pub fn randomize_user_state(ctx: Context<RandomizeUserState>, client_seed: u8) -> Result<()> {
        ctx.accounts.randomize_user_state(client_seed, instruction::CallbackRandomize::DISCRIMINATOR.as_ref())?;
        
        Ok(())
    }

    pub fn randomize_user_state_delegated(ctx: Context<RandomizeUserStateDelegated>, client_seed: u8) -> Result<()> {
        ctx.accounts.randomize_user_state_delegated(client_seed, instruction::CallbackRandomize::DISCRIMINATOR.as_ref())?;
        
        Ok(())
    }

    pub fn callback_randomize(ctx: Context<CallbackRandomizeCtx>, randomness: [u8; 32]) -> Result<()> {
        ctx.accounts.callback_randomize(randomness)?;
        
        Ok(())
    }
}

