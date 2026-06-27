use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, rent::Rent, Sysvar},
    AccountView, ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::InitializeAccount3;

use crate::{constant::MIN_AMOUNT_TO_RAISE};

pub fn process_initialize_instruction(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    //amount u64 duration u8 -> u8-> 9
    let [maker, mint_to_raise, fundraiser, vault, _system_program @ ..] = accounts else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    //list of account contraints.
    //
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }


    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let bump = [unsafe { data.as_ptr().read() }];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);

    unsafe {
        
        if fundraiser.owner() == &crate::ID {

            return Err(ProgramError::IllegalOwner);
        }
    }
let rent = Rent::get()?;
            CreateAccount {
                from: maker,
                to: fundraiser,
                lamports: rent.try_minimum_balance(90)?,
                space: 90 as u64,
                owner: &crate::ID,
            }
            .invoke_signed(&[seeds.clone()])?;

    CreateAccount {
        from: maker,
        to: vault,
        owner: &pinocchio_token::ID,
        space: 165,
        lamports: rent.minimum_balance_unchecked(165),
    }
    .invoke()?;

    InitializeAccount3 {
        account: vault,
        mint: mint_to_raise,
        owner: fundraiser.address(),
    }
    .invoke()?;

    // pinocchio_associated_token_account::instructions::Create {
    //     token_program,
    //     system_program,
    //     funding_account: maker,
    //     account: vault,
    //     mint: mint_to_raise,
    //     wallet: fundraiser,
    // }
    // .invoke()?;
    // pinocchio_system::instructions::CreateAccount {
    //     from: maker,
    //     to: vault,
    //     lamports: Rent::get()?.minimum_balance_unchecked(TokenAccount::LEN),
    //     space: TokenAccount::LEN as u64,
    //     owner: token_program.address(),  // ← token program owns the account
    // }.invoke_signed(&[seeds.clone()])?;  // ← fundraiser PDA signs for vault

    // // Step 2: initialise it as a token account directly
    // pinocchio_token::instructions::InitializeAccount3 {
    //     account: vault,
    //     mint: mint_to_raise,
    //     owner: fundraiser.address(),
    // }.invoke()?;

    let amount = unsafe { (data.as_ptr().add(1) as *const u64).read_unaligned() };

    let duration = unsafe { *(data.as_ptr().add(9) as *const u8) };

    let decimals = unsafe { mint_to_raise.data_ptr().add(40).read() };

    let min_amount = (MIN_AMOUNT_TO_RAISE as u64)
        .saturating_mul(10u64.pow(decimals as u32));

    if amount <= min_amount {

        return Err(ProgramError::Custom(12));
    }

    /*    pub maker: [u8; 32],
    pub mint_to_raise: [u8; 32],
    pub amount_to_raise: [u8; 8],
    pub current_amount: [u8; 8],
    pub time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8, */

    let clock_ts = Clock::get()?.unix_timestamp;
    // let time_started: i64 = clock.unix_timestamp;


    unsafe {
        // Write the whole struct into the account bytes in one shot — no index math, no copies field by field
        let base = fundraiser.data_ptr();
                // maker pubkey — 32 bytes
                core::ptr::copy_nonoverlapping(maker.address().as_array().as_ptr(), base, 32);
                // mint_to_raise pubkey — 32 bytes
                core::ptr::copy_nonoverlapping(mint_to_raise.address().as_array().as_ptr(), base.add(32), 32);
                // amount_to_raise — u64 LE
                (base.add(64) as *mut u64).write_unaligned(amount);
                // current_amount — u64 zero
                (base.add(72) as *mut u64).write_unaligned(0u64);
                // time_started — i64 LE
                (base.add(80) as *mut i64).write_unaligned(clock_ts);
                // duration — u8
                base.add(88).write(duration);
                // bump — u8
                base.add(89).write(bump[0]);
    }

    Ok(())
}
