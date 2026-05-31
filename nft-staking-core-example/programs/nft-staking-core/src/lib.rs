use anchor_lang::prelude::*;

mod state;
mod instructions;
mod errors;
use instructions::*;

declare_id!("E9Xs2NZciqeQySG6RGRfCBsuBuShoZss8e6KL63N2fSC");

#[program]
pub mod nft_staking_core {
    use super::*;

    pub fn create_collection(ctx: Context<CreateCollection>, name: String, uri: String) -> Result<()> {
        ctx.accounts.create_collection(name, uri, &ctx.bumps)
    }

    pub fn mint_nft(ctx: Context<Mint>, name: String, uri: String) -> Result<()> {
        ctx.accounts.mint_nft(name, uri, &ctx.bumps)
    }

    pub fn initialize_config(ctx: Context<InitConfig>, points_per_stake: u32, freeze_period: u8) -> Result<()> {
        ctx.accounts.init_config(points_per_stake, freeze_period, &ctx.bumps)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake(&ctx.bumps)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake(&ctx.bumps)
    }

    // Task 1.1: Claim rewards without unstaking
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.claim_rewards(&ctx.bumps)
    }

    // Task 1.2: Burn staked NFT for bonus rewards
    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.burn_staked_nft(&ctx.bumps)
    }

    // Task 2: Oracle - Initialize oracle account
    pub fn initialize_oracle(ctx: Context<InitializeOracle>) -> Result<()> {
        ctx.accounts.initialize_oracle(&ctx.bumps)
    }

    // Task 2: Oracle - Crank to update oracle validation based on time
    pub fn crank_oracle(ctx: Context<CrankOracle>) -> Result<()> {
        ctx.accounts.crank_oracle()
    }

    // Task 2: Oracle - Transfer NFT (respects oracle time window)
    pub fn transfer_nft<'info>(ctx: Context<'_, '_, 'info, 'info, TransferNft<'info>>) -> Result<()> {
        ctx.accounts.transfer_nft(&ctx.bumps, ctx.remaining_accounts)
    }

    // Task 2: Oracle - Fund the crank vault
    pub fn fund_crank_vault(ctx: Context<FundCrankVault>, amount: u64) -> Result<()> {
        ctx.accounts.fund_crank_vault(amount)
    }
}