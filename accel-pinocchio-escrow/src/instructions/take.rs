use pinocchio::{msg, ProgramResult};
use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio_pubkey::derive_address;
use pinocchio_token_2022::instructions::{CloseAccount, Transfer};
use crate::state::Escrow;

pub fn process_take_instruction(
    accounts: &[AccountInfo],
) -> ProgramResult {
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
    ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    msg!("Accounts unpacked");


    {
        msg!("Validating taker ATAs");

        if taker_ata_a.data_len() > 0 && taker_ata_a.owner() != token_program.key() {
            msg!("Taker ATA A not owned by token program");
            return Err(ProgramError::IllegalOwner);
        }

        if taker_ata_b.data_len() > 0 && taker_ata_b.owner() != token_program.key() {
            msg!("Taker ATA B not owned by token program");
            return Err(ProgramError::IllegalOwner);
        }

        msg!("Taker ATAs validated");
    }

    assert!(taker.is_signer());
    msg!("Taker is signer");

    let escrow_state = Escrow::from_account_info(escrow_account)?;

    let amount_to_give = escrow_state.amount_to_give();
    let amount_to_receive = escrow_state.amount_to_receive();
    let bump = escrow_state.bump;

    msg!("Escrow state loaded");

    let escrow_account_pda = derive_address(
        &[b"escrow".as_ref(), maker.key().as_slice(), &[bump]],
        None,
        &crate::ID
    );

    assert_eq!(escrow_account_pda, *escrow_account.key());
    msg!("Escrow PDA verified");

    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.key()),
        Seed::from(&bump_bytes)
    ];
    let seeds = Signer::from(&seed);

    // Create taker's ATA for mint A if needed
    msg!("Creating taker ATA for mint A");
    pinocchio_associated_token_account::instructions::CreateIdempotent {
        funding_account: taker,
        account: taker_ata_a,
        wallet: taker,
        mint: mint_a,
        system_program: system_program,
        token_program: token_program,
    }.invoke()?;
    msg!("Taker ATA A ready");

    // Create maker's ATA for mint B if needed
    msg!("Creating maker ATA for mint B");
    pinocchio_associated_token_account::instructions::CreateIdempotent {
        funding_account: taker,
        account: maker_ata_b,
        wallet: maker,
        mint: mint_b,
        system_program: system_program,
        token_program: token_program,
    }.invoke()?;
    msg!("Maker ATA B ready");

    // Transfer from taker to maker
    msg!("Transferring from taker to maker");
    Transfer {
        from: taker_ata_b,
        to: maker_ata_b,
        authority: taker,
        amount: amount_to_receive,
        token_program: token_program.key(),
    }.invoke()?;
    msg!("Transfer to maker complete");

    // Transfer from escrow to taker
    msg!("Transferring from escrow to taker");
    Transfer {
        from: escrow_ata,
        to: taker_ata_a,
        authority: escrow_account,
        amount: amount_to_give,
        token_program: token_program.key(),
    }.invoke_signed(&[seeds.clone()])?;
    msg!("Transfer to taker complete");

    // Close escrow ATA
    msg!("Closing escrow ATA");
    CloseAccount {
        account: escrow_ata,
        destination: maker,
        authority: escrow_account,
        token_program: token_program.key(),
    }.invoke_signed(&[seeds])?;
    msg!("Escrow ATA closed");

    // Return escrow account lamports to maker
    msg!("Returning escrow lamports to maker");
    unsafe {
        let maker_lamports = maker.borrow_mut_lamports_unchecked();
        let escrow_lamports = escrow_account.borrow_mut_lamports_unchecked();

        *maker_lamports = maker_lamports
            .checked_add(*escrow_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        *escrow_lamports = 0;
    }

    msg!("Take instruction complete");
    Ok(())
}
