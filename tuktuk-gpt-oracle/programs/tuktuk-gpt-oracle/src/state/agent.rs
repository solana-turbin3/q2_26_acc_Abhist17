use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Agent {
    pub context: Pubkey,
    pub bump: u8,
}