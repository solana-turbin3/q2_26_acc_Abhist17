use anchor_lang::{
    prelude::*, 
    system_program
};

use crate::state::WhiteListEntry;

#[derive(Accounts)]
pub struct AddToWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: User account
    pub user: UncheckedAccount<'info>,
    
    #[account(
        init,
        payer = admin,
        space = 8 + WhiteListEntry::LEN,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist_entry: Account<'info, WhiteListEntry>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: User account
    pub user: UncheckedAccount<'info>,

    #[account (
        mut,
        close = admin,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]    
    pub whitelist_entry: Account<'info, WhiteListEntry>,
}