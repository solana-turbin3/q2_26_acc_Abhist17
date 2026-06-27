use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    AccountView, ProgramResult,
};

use pinocchio_token::instructions::{ Transfer};

/*use anchor_lang::prelude::*;





*/
pub fn process_checker_instruction(accounts: &[AccountView], _data: &[u8]) -> ProgramResult {
    //amount u64 duration u8 -> u8-> 9
    let [maker, mint_to_raise, fundraiser, vault, maker_ata, system_program, token_program, _associated_token_program] =
        accounts
    else {
        return Err(pinocchio::error::ProgramError::NotEnoughAccountKeys);
    };

    //list of account contraints.
    //
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }
    // if !(Mint::LEN == mint_to_raise.data_len()) {
    //     return Err(ProgramError::AccountDataTooSmall);
    // }

    let vault_token_amount = 
    unsafe { (vault.data_ptr().add(64) as *const u64).read_unaligned() };


    let (fundraiser_amount_raise, bump) = 
    (
        unsafe { (fundraiser.data_ptr().add(64) as *const u64).read_unaligned() },
        unsafe { fundraiser.data_ptr().add(89).read() },
    );
    // no borrow opened at all — data_ptr() is just a pointer


    if vault_token_amount < fundraiser_amount_raise {
        return Err(ProgramError::AccountDataTooSmall);
    }



    //inti if neede for maket ata
    if maker_ata.lamports() == 0 {
        pinocchio_associated_token_account::instructions::Create {
            funding_account: maker,
            account: maker_ata,
            wallet: maker,
            mint: mint_to_raise,
            token_program: token_program,
            system_program: system_program,
        }
        .invoke()?;
    }

    let bump = [bump];
    let seed = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_ref()),
        Seed::from(&bump),
    ];
    let seeds = Signer::from(&seed);
    Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: vault_token_amount,
    }
    .invoke_signed(&[seeds.clone()])?;

   
       
    maker.set_lamports(unsafe {
        maker
            .lamports()
            .unchecked_add(fundraiser.lamports())
    });
    fundraiser.close()?;

    Ok(())
}
