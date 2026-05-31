use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub points_per_stake: u32,
    pub freeze_period: u8,
    pub rewards_bump: u8,
    pub config_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct OracleState {
    pub transfer_allowed: bool,    // true = approved, false = rejected
    pub last_updated: i64,         // last time the oracle was updated
    pub bump: u8,
    pub vault_bump: u8,
    pub collection: Pubkey,        // the collection this oracle is for
}