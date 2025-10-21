
use pinocchio::{msg,ProgramResult};
use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::{CloseAccount, Transfer};
use pinocchio_token::state::TokenAccount;
use crate::state::Escrow;

pub fn process_take_instruction(
    accounts: &[AccountInfo],
) -> ProgramResult{
    msg!("Invoking take instruction");

    let [
        taker,
        maker,
        mint_a,
        mint_b,
        escrow_account,
        escrow_ata,
        taker_ata_a,
        taker_ata_b,
        maker_ata_b,
        system_program,
        token_program,
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let taker_ata_state = TokenAccount::from_account_info(&taker_ata_a)?;
    if taker_ata_state.owner() != taker.key(){
        return Err(ProgramError::IllegalOwner);
    }
    let taker_ata_state = TokenAccount::from_account_info(&taker_ata_b)?;
    if taker_ata_state.owner() != taker.key(){
        return Err(ProgramError::IllegalOwner);
    }

    assert!(taker.is_signer());

    let escrow_state = Escrow::from_account_info(escrow_account)?;
    // if (escrow_state.owner() != &crate::ID){
    //     return Err(ProgramError::IllegalOwner);
    // };
    let amount_to_give = escrow_state.amount_to_give();
    let amount_to_receive = escrow_state.amount_to_receive();
    let bump = escrow_state.bump;

    let escrow_account_pda = derive_address(
        &[b"escrow".as_ref(), maker.key().as_slice(), &[bump]],
        None,
        &crate::ID
    );

    assert_eq!(escrow_account_pda, *escrow_account.key());

    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.key()),
        Seed::from(&bump_bytes)
    ];
    let seeds = Signer::from(&seed);

    // Initialize taker's ATA for mint_a if needed
    if taker_ata_a.data_len() == 0 {
        pinocchio_associated_token_account::instructions::Create {
            funding_account: taker,
            account: taker_ata_a,
            wallet: taker,
            mint: mint_a,
            token_program: token_program,
            system_program: system_program,
        }.invoke()?;
    }

    // Initialize maker's ATA for mint_b if needed
    if maker_ata_b.data_len() == 0 {
        pinocchio_associated_token_account::instructions::Create {
            funding_account: taker,
            account: maker_ata_b,
            wallet: maker,
            mint: mint_b,
            token_program: token_program,
            system_program: system_program,
        }.invoke()?;
    }

    Transfer{
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive
    }.invoke()?;

    Transfer{
        from: escrow_ata,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give
    }.invoke()?;

    CloseAccount{
        account: escrow_ata,
        destination: maker,
        authority: escrow_account
    }.invoke_signed(&[seeds])?;
    unsafe {
        let maker_lamports = maker.borrow_mut_lamports_unchecked();
        let escrow_lamports = escrow_account.borrow_mut_lamports_unchecked();

        *maker_lamports = maker_lamports
            .checked_add(*escrow_lamports)
            .ok_or(pinocchio::program_error::ProgramError::ArithmeticOverflow)?;

        *escrow_lamports = 0;
    }
    Ok(())
}