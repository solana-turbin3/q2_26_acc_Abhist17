use anchor_lang::prelude::*;
use anchor_lang::system_program::{create_account, CreateAccount};
use anchor_spl::{
    token_2022::{
        initialize_mint2,
        spl_token_2022::{
            extension::{
                transfer_fee::TransferFeeConfig, BaseStateWithExtensions, ExtensionType,
                StateWithExtensions,
            },
            pod::PodMint,
            state::Mint as MintState,
        },
        InitializeMint2,
    },
    token_interface::{
        spl_pod::optional_keys::OptionalNonZeroPubkey, transfer_fee_initialize, Token2022,
        TransferFeeInitialize,
    },
};

#[derive(Accounts)]
pub struct InitMint<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub mint: Signer<'info>,

    /// CHECK: PDA that will be the withdraw withheld authority for automated collection
    #[account(
        seeds = [b"fee_authority"],
        bump
    )]
    pub fee_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn process_init_mint(
    ctx: Context<InitMint>,
    decimals: u8,
    transfer_fee_basis_points: u16,
    maximum_fee: u64,
) -> Result<()> {
    // Calculate space required for mint with TransferFeeConfig extension
    let mint_size =
        ExtensionType::try_calculate_account_len::<PodMint>(&[ExtensionType::TransferFeeConfig])?;

    let lamports = Rent::get()?.minimum_balance(mint_size);

    // Create the mint account
    create_account(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            CreateAccount {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.mint.to_account_info(),
            },
        ),
        lamports,
        mint_size as u64,
        &ctx.accounts.token_program.key(),
    )?;

    // Initialize transfer fee extension BEFORE initializing mint
    // IMPORTANT: withdraw_withheld_authority is set to fee_authority PDA for automated collection
    transfer_fee_initialize(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferFeeInitialize {
                token_program_id: ctx.accounts.token_program.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        Some(&ctx.accounts.authority.key()),      // transfer_fee_config_authority (can update fee)
        Some(&ctx.accounts.fee_authority.key()),  // withdraw_withheld_authority (PDA for automation!)
        transfer_fee_basis_points,
        maximum_fee,
    )?;

    // Initialize the mint
    initialize_mint2(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            InitializeMint2 {
                mint: ctx.accounts.mint.to_account_info(),
            },
        ),
        decimals,
        &ctx.accounts.authority.key(),
        Some(&ctx.accounts.authority.key()),
    )?;

    // Verify extension data was set correctly
    let mint = &ctx.accounts.mint.to_account_info();
    let mint_data = mint.data.borrow();
    let mint_with_extension = StateWithExtensions::<MintState>::unpack(&mint_data)?;
    let extension_data = mint_with_extension.get_extension::<TransferFeeConfig>()?;

    // Verify fee_authority is set as withdraw authority
    assert_eq!(
        extension_data.withdraw_withheld_authority,
        OptionalNonZeroPubkey::try_from(Some(ctx.accounts.fee_authority.key()))?
    );

    msg!("Mint initialized with transfer fee extension");
    msg!("Transfer fee: {} basis points", transfer_fee_basis_points);
    msg!("Maximum fee: {}", maximum_fee);
    msg!("Withdraw authority (PDA): {}", ctx.accounts.fee_authority.key());

    Ok(())
}