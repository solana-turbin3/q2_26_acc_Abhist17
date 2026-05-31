
use anchor_lang::prelude::*;

#[account]
pub struct WhiteListEntry {
    pub user: Pubkey,
    pub bump: u8,
}

impl WhiteListEntry {
    pub const LEN: usize = 32 + 1;
}