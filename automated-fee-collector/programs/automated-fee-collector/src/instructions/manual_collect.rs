use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    harvest_withheld_tokens_to_mint, withdraw_withheld_tokens_from_mint,
    HarvestWithheldTokensToMint, Mint, TokenAccount, TokenInterface,
    WithdrawWithheldTokensFromMint,
};

#[derive(Accounts)]
pub struct ManualCollect<'info> {
    /// CHECK: PDA that is the withdraw_withheld_authority - verified by seeds
    #[account(
        seeds = [b"fee_authority"],
        bump
    )]
    pub fee_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub mint_account: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint_account,
    )]
    pub treasury_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn collect<'info>(ctx: Context<'_, '_, 'info, 'info, ManualCollect<'info>>) -> Result<()> {
    // Filter remaining accounts to only include valid token accounts for this mint
    let sources: Vec<AccountInfo<'info>> = ctx
        .remaining_accounts
        .iter()
        .filter_map(|account| {
            InterfaceAccount::<TokenAccount>::try_from(account)
                .ok()
                .filter(|token_account| token_account.mint == ctx.accounts.mint_account.key())
                .map(|_| account.to_account_info())
        })
        .collect();

    if sources.is_empty() {
        msg!("No valid source accounts to harvest from");
        return Ok(());
    }

    msg!("Harvesting fees from {} accounts", sources.len());

    // Step 1: Harvest withheld tokens from source accounts TO the mint
    harvest_withheld_tokens_to_mint(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            HarvestWithheldTokensToMint {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
            },
        ),
        sources,
    )?;

    msg!("Harvested fees to mint");

    // Step 2: Withdraw harvested tokens from mint to treasury
    // This requires signing with the fee_authority PDA
    let bump = ctx.bumps.fee_authority;
    let signer_seeds: &[&[&[u8]]] = &[&[b"fee_authority", &[bump]]];

    withdraw_withheld_tokens_from_mint(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            WithdrawWithheldTokensFromMint {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint_account.to_account_info(),
                destination: ctx.accounts.treasury_token_account.to_account_info(),
                authority: ctx.accounts.fee_authority.to_account_info(),
            },
            signer_seeds,
        ),
    )?;

    msg!("Fees withdrawn to treasury: {}", ctx.accounts.treasury_token_account.key());

    Ok(())
}