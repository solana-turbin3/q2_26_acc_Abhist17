use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_token::{instructions::Transfer, state::Mint};

use crate::{
    constant::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS},
    ID,
};

pub fn process_contribute_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    //amount u64 duration u8 -> u8-> 9
    let [contributor, mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, _system_program, _token_program] =
        accounts
    else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    //list of account contraints.
    //
    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !(Mint::LEN == mint_to_raise.data_len()) {
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Optimise — skip flag check

    let (
        fundraiser_mint,
        fundraiser_duration,
        fundraiser_time_started,
        fundraiser_amount_to_raise,
        fundraiser_current_amaount,
    ) = {
        let fundraiser_data = unsafe { fundraiser.borrow_unchecked() };

        (
            unsafe { &*(fundraiser.data_ptr().add(32) as *const [u8; 32]) },
            unsafe { fundraiser.data_ptr().add(88).read() },
            // Optimised: single load instruction, no slice bounds check, no try_into
            unsafe { (fundraiser_data.as_ptr().add(80) as *const i64).read_unaligned() },
            unsafe { (fundraiser_data.as_ptr().add(64) as *const u64).read_unaligned() },
            unsafe { (fundraiser_data.as_ptr().add(72) as *const u64).read_unaligned() },
        )
    };
    // Verify mint_to_raise matches what's in the fundraiser

    if fundraiser_mint != mint_to_raise.address().as_array() {
        return Err(ProgramError::Custom(101));
    }

    let contributor_bump = [unsafe { data.as_ptr().add(8).read() }];

    let seed = [
        Seed::from(b"contributor"),
        Seed::from(fundraiser.address().as_ref()),
        Seed::from(contributor.address().as_ref()),
        Seed::from(&contributor_bump),
    ];
    let seeds = Signer::from(&seed);

    //inti if neede for maket ata
    if contributor_account.lamports() == 0 {
        let lamports = Rent::get()?.try_minimum_balance(8)?;

        pinocchio_system::instructions::CreateAccount {
            from: contributor,
            to: contributor_account,
            space: 8,
            lamports,
            owner: &ID,
        }
        .invoke_signed(&[seeds])?;
    }

    //program
    //
    //

    let amount = unsafe { (data.as_ptr() as *const u64).read_unaligned() };

    let mint_to_raise_decimals = unsafe { mint_to_raise.data_ptr().add(40).read() };
    // Check if the amount to contribute meets the minimum amount required
    if !(amount > 10_u8.pow(mint_to_raise_decimals as u32) as u64) {
        return Err(ProgramError::Custom(102));
    }

    // Check if the amount to contribute is less than the maximum allowed contribution
    //
    //
    let max_contri = (fundraiser_amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER;
    if !(amount <= max_contri) {
        return Err(ProgramError::Custom(103));
    }

    /*
    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
    */

    // Check if the fundraising duration has been reached
    let current_time = Clock::get()?.unix_timestamp;
    if !(fundraiser_duration <= ((current_time - fundraiser_time_started) / SECONDS_TO_DAYS) as u8)
    {
        return Err(ProgramError::Custom(104));
        // crate::FundraiserError::FundraiserEnded
    }

    let contributor_account_amount =
        unsafe { (contributor_account.data_ptr() as *const u64).read_unaligned() };

    // Check if the maximum contributions per contributor have been reached
    if !((contributor_account_amount <= max_contri)
        && (contributor_account_amount + amount <= max_contri))
    {
        return Err(ProgramError::Custom(105));
        // FundraiserError::MaximumContributionsReached
    }

    Transfer {
        from: contributor_ata,
        authority: contributor,
        to: vault,
        amount: amount,
    }
    .invoke()?;

    unsafe {
        // COMPUTE
        let new_val = fundraiser_current_amaount.unchecked_add(amount);

        let wptr = fundraiser.data_ptr().add(72) as *mut u64;

        wptr.write_unaligned(new_val);
    }

    // let fundraiser_current_amaount =
    //     u64::from_le_bytes(fundraiser_data[72..80].try_into().unwrap());
    // fundraiser_data[72..80].copy_from_slice(
    //     unsafe { fundraiser_current_amaount
    //         .unchecked_add(amount)
    //         .to_le_bytes()
    //         .as_ref() },
    // );

    //? - i Think
    //
    unsafe {

        let wptr = contributor_account.data_ptr() as *mut u64;

        wptr.write_unaligned(contributor_account_amount.unchecked_add(amount));
    }

    Ok(())
}
