use anchor_lang::prelude::*;

declare_id!("8Axod2sBdMn6b7Nwicbtujwj6Gsv3wfBh6NjDtPPVKAh");

mod error;
mod instructions;

use instructions::*;

#[program]
pub mod automated_fee_collector {
    use super::*;

    pub fn init_mint(
        ctx: Context<InitMint>,
        decimals: u8,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        process_init_mint(ctx, decimals, transfer_fee_basis_points, maximum_fee)
    }

    pub fn init_treasury(ctx: Context<InitTreasury>) -> Result<()> {
        process_init_treasury(ctx)
    }

    pub fn mint_to(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        process_mint_to(ctx, amount)
    }

    pub fn transfer(ctx: Context<Transfer>, amount: u64) -> Result<()> {
        process_transfer(ctx, amount)
    }

    pub fn manual_collect<'info>(
        ctx: Context<'_, '_, 'info, 'info, ManualCollect<'info>>,
    ) -> Result<()> {
        collect(ctx)
    }

    pub fn schedule<'info>(
        ctx: Context<'_, '_, 'info, 'info, Schedule<'info>>,
        task_id: u16,
    ) -> Result<()> {
        process_schedule(ctx, task_id)
    }

    pub fn update_fee(
        ctx: Context<UpdateFee>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<()> {
        process_update_fee(ctx, transfer_fee_basis_points, maximum_fee)
    }
}