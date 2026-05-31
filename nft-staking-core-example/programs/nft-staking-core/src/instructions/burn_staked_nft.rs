use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to_checked, Mint, MintToChecked, TokenAccount, TokenInterface},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::{BurnV1CpiBuilder, UpdatePluginV1CpiBuilder, UpdateCollectionPluginV1CpiBuilder},
    types::{Attribute, Attributes, FreezeDelegate, Plugin, PluginType, UpdateAuthority},
    ID as MPL_CORE_ID,
};
use crate::errors::StakingError;
use crate::state::Config;

const SECONDS_PER_DAY: i64 = 86400;
const BURN_BONUS_MULTIPLIER: u64 = 10;

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"config", collection.key().as_ref()],
        bump = config.config_bump
    )]
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [b"rewards", config.key().as_ref()],
        bump = config.rewards_bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = rewards_mint,
        associated_token::authority = user,
    )]
    pub user_rewards_ata: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: NFT account
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> BurnStakedNft<'info> {
    pub fn burn_staked_nft(&mut self, bumps: &BurnStakedNftBumps) -> Result<()> {
        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.user.key(), StakingError::InvalidOwner);
        require!(
            base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()),
            StakingError::InvalidAuthority
        );
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(
            base_collection.update_authority == self.update_authority.key(),
            StakingError::InvalidAuthority
        );

        let collection_key = self.collection.key();
        let signer_seeds = &[
            b"update_authority",
            collection_key.as_ref(),
            &[bumps.update_authority],
        ];

        let current_timestamp = Clock::get()?.unix_timestamp;

        let fetched_attribute_list = match fetch_plugin::<BaseAssetV1, Attributes>(
            &self.nft.to_account_info(),
            PluginType::Attributes,
        ) {
            Err(_) => return Err(StakingError::NotStaked.into()),
            Ok((_, attributes, _)) => attributes,
        };

        let mut staked_value: Option<String> = None;
        let mut staked_at_value: Option<String> = None;

        for attribute in &fetched_attribute_list.attribute_list {
            match attribute.key.as_str() {
                "staked" => staked_value = Some(attribute.value.clone()),
                "staked_at" => staked_at_value = Some(attribute.value.clone()),
                _ => {}
            }
        }

        require!(staked_value.as_deref() == Some("true"), StakingError::NotStaked);

        let staked_at_timestamp = staked_at_value
            .ok_or(StakingError::InvalidTimestamp)?
            .parse::<i64>()
            .map_err(|_| StakingError::InvalidTimestamp)?;

        let elapsed_seconds = current_timestamp
            .checked_sub(staked_at_timestamp)
            .ok_or(StakingError::InvalidTimestamp)?;

        let staked_time_days = elapsed_seconds
            .checked_div(SECONDS_PER_DAY)
            .ok_or(StakingError::InvalidTimestamp)?;

        let effective_days = if staked_time_days < 1 { 1 } else { staked_time_days as u64 };
        let base_amount = effective_days
            .checked_mul(self.config.points_per_stake as u64)
            .ok_or(StakingError::Overflow)?;
        let bonus_amount = base_amount
            .checked_mul(BURN_BONUS_MULTIPLIER)
            .ok_or(StakingError::Overflow)?;

        // Unfreeze before burn
        match fetch_plugin::<BaseAssetV1, FreezeDelegate>(
            &self.nft.to_account_info(),
            PluginType::FreezeDelegate,
        ) {
            Ok((_, freeze_delegate, _)) => {
                if freeze_delegate.frozen {
                    UpdatePluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                        .asset(&self.nft.to_account_info())
                        .collection(Some(&self.collection.to_account_info()))
                        .payer(&self.user.to_account_info())
                        .authority(Some(&self.update_authority.to_account_info()))
                        .system_program(&self.system_program.to_account_info())
                        .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
                        .invoke_signed(&[signer_seeds])?;
                }
            }
            Err(_) => {}
        }

        BurnV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.nft.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.user.to_account_info())
            .authority(Some(&self.user.to_account_info()))
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke()?;

        let config_seeds = &[
            b"config",
            collection_key.as_ref(),
            &[self.config.config_bump],
        ];
        let config_signer_seeds = &[&config_seeds[..]];

        let cpi_accounts = MintToChecked {
            mint: self.rewards_mint.to_account_info(),
            to: self.user_rewards_ata.to_account_info(),
            authority: self.config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            config_signer_seeds,
        );
        mint_to_checked(cpi_ctx, bonus_amount, self.rewards_mint.decimals)?;

        self.update_collection_staked_count(signer_seeds)?;

        Ok(())
    }

    fn update_collection_staked_count(&self, signer_seeds: &[&[u8]]) -> Result<()> {
        let result = fetch_plugin::<BaseCollectionV1, Attributes>(
            &self.collection.to_account_info(),
            PluginType::Attributes,
        );

        if let Ok((_, fetched_attrs, _)) = result {
            let mut attribute_list: Vec<Attribute> = Vec::new();
            for attr in fetched_attrs.attribute_list {
                if attr.key == "total_staked" {
                    let current: u64 = attr.value.parse().unwrap_or(0);
                    attribute_list.push(Attribute {
                        key: "total_staked".to_string(),
                        value: current.saturating_sub(1).to_string(),
                    });
                } else {
                    attribute_list.push(attr);
                }
            }

            UpdateCollectionPluginV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
                .collection(&self.collection.to_account_info())
                .payer(&self.user.to_account_info())
                .authority(Some(&self.update_authority.to_account_info()))
                .system_program(&self.system_program.to_account_info())
                .plugin(Plugin::Attributes(Attributes { attribute_list }))
                .invoke_signed(&[signer_seeds])?;
        }
        Ok(())
    }
}