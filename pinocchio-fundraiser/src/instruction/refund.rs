use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_token::{instructions::Transfer};

use crate::constant::SECONDS_TO_DAYS;

pub fn process_refund_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    //amount u64 duration u8 -> u8-> 9
    let [contributor, maker, _mint_to_raise, fundraiser, contributor_account, contributor_ata, vault, _void @ ..] =
        accounts
    else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    //list of account contraints.
    //
    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // if !(Mint::LEN == mint_to_raise.data_len()) {
    //     return Err(ProgramError::AccountDataTooSmall);
    // }

    let (
        fundraiser_duration,
        fundraiser_time_started,
        fundraiser_amount_to_raise,
        fundraiser_current_amaount,
        seed_bump,
    ) = {
        let fundraiser_data = unsafe { fundraiser.borrow_unchecked() };

        (
            unsafe { fundraiser.data_ptr().add(88).read() },
            // Optimised: single load instruction, no slice bounds check, no try_into
            unsafe { (fundraiser_data.as_ptr().add(80) as *const i64).read_unaligned() },
            unsafe { (fundraiser_data.as_ptr().add(64) as *const u64).read_unaligned() },
            unsafe { (fundraiser_data.as_ptr().add(72) as *const u64).read_unaligned() },
            [unsafe { fundraiser.data_ptr().add(89).read() }],
        )
    };

    //progrma.
    //
    let current_time = Clock::get()?.unix_timestamp;

    if !(fundraiser_duration >= ((current_time - fundraiser_time_started) / SECONDS_TO_DAYS) as u8)
    {
        //        crate::FundraiserError::FundraiserNotEnded
        return Err(ProgramError::Custom(300));
    }

    let vault_amount =
        unsafe { (vault.borrow_unchecked().as_ptr().add(64) as *const u64).read_unaligned() };
    if !(vault_amount < fundraiser_amount_to_raise) {
        //   crate::FundraiserError::TargetMet
        return Err(ProgramError::Custom(301));
    }

    let contributor_account_amount = {
        let contributor_account_data = unsafe { contributor_account.borrow_unchecked() };

        unsafe { (contributor_account_data.as_ptr() as *const u64).read_unaligned() }
    };

    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&seed_bump),
    ];
    let seeds = Signer::from(&seed);
    Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: contributor_account_amount,
    }
    .invoke_signed(&[seeds])?;

    unsafe {
        // READ: single 8-byte load

        // COMPUTE
        let new_val = fundraiser_current_amaount.unchecked_sub(contributor_account_amount);

        // WRITE: single 8-byte store
        let wptr = fundraiser.data_ptr().add(72) as *mut u64;
        wptr.write_unaligned(new_val);
    }
    // let mut fundraiser_data = fundraiser.try_borrow_mut()?;

    // fundraiser_data[72..80].copy_from_slice(unsafe {
    //     fundraiser_current_amaount
    //         .unchecked_sub(contributor_account_amount)
    //         .to_le_bytes()
    //         .as_ref()
    // });

    maker.set_lamports(unsafe {
        maker
            .lamports()
            .unchecked_add(contributor_account.lamports())
    });
    contributor_account.close()?;
    Ok(())
}
