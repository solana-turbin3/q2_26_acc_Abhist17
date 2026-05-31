use anchor_lang::prelude::*;
use anchor_lang::system_program;
use mpl_core::{
    ID as MPL_CORE_ID,
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::TransferV1CpiBuilder,
    types::UpdateAuthority,
};
use crate::errors::StakingError;
use crate::state::OracleState;

const WINDOW_OPEN_HOUR: i64 = 9;
const WINDOW_CLOSE_HOUR: i64 = 17;
const SECONDS_PER_HOUR: i64 = 3600;
const SECONDS_PER_DAY: i64 = 86400;
const BOUNDARY_TOLERANCE: i64 = 300;
const CRANK_REWARD: u64 = 10_000;

#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Collection account
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + OracleState::INIT_SPACE,
        seeds = [b"oracle", collection.key().as_ref()],
        bump
    )]
    pub oracle_account: Account<'info, OracleState>,
    /// CHECK: PDA vault for crank rewards
    #[account(
        mut,
        seeds = [b"crank_vault", collection.key().as_ref()],
        bump
    )]
    pub crank_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeOracle<'info> {
    pub fn initialize_oracle(&mut self, bumps: &InitializeOracleBumps) -> Result<()> {
        let base_collection = BaseCollectionV1::try_from(&self.collection.to_account_info())?;
        require!(
            base_collection.update_authority == self.update_authority.key(),
            StakingError::InvalidAuthority
        );

        let clock = Clock::get()?;
        let current_hour = get_utc_hour(clock.unix_timestamp);
        let transfer_allowed = current_hour >= WINDOW_OPEN_HOUR && current_hour < WINDOW_CLOSE_HOUR;

        self.oracle_account.set_inner(OracleState {
            transfer_allowed,
            last_updated: clock.unix_timestamp,
            bump: bumps.oracle_account,
            vault_bump: bumps.crank_vault,
            collection: self.collection.key(),
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct CrankOracle<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    /// CHECK: Collection reference
    pub collection: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"oracle", collection.key().as_ref()],
        bump = oracle_account.bump,
        has_one = collection,
    )]
    pub oracle_account: Account<'info, OracleState>,
    /// CHECK: PDA vault for crank rewards
    #[account(
        mut,
        seeds = [b"crank_vault", collection.key().as_ref()],
        bump = oracle_account.vault_bump
    )]
    pub crank_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CrankOracle<'info> {
    pub fn crank_oracle(&mut self) -> Result<()> {
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;
        let current_hour = get_utc_hour(current_timestamp);

        let should_be_allowed = current_hour >= WINDOW_OPEN_HOUR && current_hour < WINDOW_CLOSE_HOUR;

        require!(
            self.oracle_account.transfer_allowed != should_be_allowed,
            StakingError::OracleAlreadyCorrectState
        );

        self.oracle_account.transfer_allowed = should_be_allowed;
        self.oracle_account.last_updated = current_timestamp;

        let seconds_since_midnight = current_timestamp.rem_euclid(SECONDS_PER_DAY);
        let open_boundary = WINDOW_OPEN_HOUR * SECONDS_PER_HOUR;
        let close_boundary = WINDOW_CLOSE_HOUR * SECONDS_PER_HOUR;

        let near_open = (seconds_since_midnight - open_boundary).abs() <= BOUNDARY_TOLERANCE;
        let near_close = (seconds_since_midnight - close_boundary).abs() <= BOUNDARY_TOLERANCE;

        if near_open || near_close {
            let vault_balance = self.crank_vault.lamports();
            let rent_exempt = Rent::get()?.minimum_balance(0);
            let available = vault_balance.saturating_sub(rent_exempt);

            if available >= CRANK_REWARD {
                **self.crank_vault.to_account_info().try_borrow_mut_lamports()? -= CRANK_REWARD;
                **self.cranker.to_account_info().try_borrow_mut_lamports()? += CRANK_REWARD;
            }
        }

        Ok(())
    }
}

#[derive(Accounts)]
pub struct FundCrankVault<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    /// CHECK: Collection reference
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA vault for crank rewards
    #[account(
        mut,
        seeds = [b"crank_vault", collection.key().as_ref()],
        bump
    )]
    pub crank_vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> FundCrankVault<'info> {
    pub fn fund_crank_vault(&mut self, amount: u64) -> Result<()> {
        system_program::transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                system_program::Transfer {
                    from: self.funder.to_account_info(),
                    to: self.crank_vault.to_account_info(),
                },
            ),
            amount,
        )?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: The new owner
    pub new_owner: UncheckedAccount<'info>,
    /// CHECK: NFT account
    #[account(mut)]
    pub nft: UncheckedAccount<'info>,
    /// CHECK: Collection account
    #[account(mut)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: PDA Update authority
    #[account(
        seeds = [b"update_authority", collection.key().as_ref()],
        bump
    )]
    pub update_authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"oracle", collection.key().as_ref()],
        bump = oracle_account.bump,
        has_one = collection,
    )]
    pub oracle_account: Account<'info, OracleState>,
    /// CHECK: Metaplex Core program
    #[account(address = MPL_CORE_ID)]
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> TransferNft<'info> {
    pub fn transfer_nft(
        &mut self,
        _bumps: &TransferNftBumps,
        remaining_accounts: &[AccountInfo<'info>],
    ) -> Result<()> {
        require!(
            self.oracle_account.transfer_allowed,
            StakingError::TransferNotAllowed
        );

        let clock = Clock::get()?;
        let current_hour = get_utc_hour(clock.unix_timestamp);
        require!(
            current_hour >= WINDOW_OPEN_HOUR && current_hour < WINDOW_CLOSE_HOUR,
            StakingError::TransferNotAllowed
        );

        let base_asset = BaseAssetV1::try_from(&self.nft.to_account_info())?;
        require!(base_asset.owner == self.owner.key(), StakingError::InvalidOwner);
        require!(
            base_asset.update_authority == UpdateAuthority::Collection(self.collection.key()),
            StakingError::InvalidAuthority
        );

        let mpl_core_program = self.mpl_core_program.to_account_info();
        let nft = self.nft.to_account_info();
        let collection = self.collection.to_account_info();
        let owner = self.owner.to_account_info();
        let new_owner = self.new_owner.to_account_info();
        let system_program = self.system_program.to_account_info();

        let mut builder = TransferV1CpiBuilder::new(&mpl_core_program);
        builder
            .asset(&nft)
            .collection(Some(&collection))
            .payer(&owner)
            .authority(Some(&owner))
            .new_owner(&new_owner)
            .system_program(Some(&system_program));

        for account in remaining_accounts {
            builder.add_remaining_account(account, false, false);
        }

        builder.invoke()?;

        Ok(())
    }
}

fn get_utc_hour(timestamp: i64) -> i64 {
    let seconds_since_midnight = timestamp.rem_euclid(SECONDS_PER_DAY);
    seconds_since_midnight / SECONDS_PER_HOUR
}