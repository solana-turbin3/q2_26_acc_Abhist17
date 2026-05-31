use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use spl_tlv_account_resolution::{
    state::ExtraAccountMetaList,
    account::ExtraAccountMeta,
    seeds::Seed,
};
use spl_transfer_hook_interface::instruction::ExecuteInstruction;

#[derive(Accounts)]
pub struct TokenFactory<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        mint::decimals = 9,
        mint::authority = user,
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: PDA storing extra-account resolution rules for the transfer hook
    #[account(
        init,
        payer = user,
        space = ExtraAccountMetaList::size_of(1).unwrap(),
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> TokenFactory<'info> {
    pub fn init_mint(&mut self) -> Result<()> {
        // Require exactly ONE extra account:
        // PDA("whitelist", owner)
        let metas = vec![
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal { bytes: b"whitelist".to_vec() },
                    // OWNER of the source token account
                    Seed::AccountKey { index: 3 }, // index 3 is the owner
                ],
                false, // not signer
                false, // not writable
            ).unwrap(),
        ];

        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut self.extra_account_meta_list.try_borrow_mut_data()?,
            &metas,
        ).unwrap();

        Ok(())
    }
}
