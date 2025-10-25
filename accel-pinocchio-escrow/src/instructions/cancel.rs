use pinocchio::account_info::AccountInfo;
use pinocchio::{msg, ProgramResult};
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::log;
use pinocchio_pubkey::derive_address;
use pinocchio_token_2022::instructions::{Transfer, CloseAccount};  // CHANGED: Use token-2022
use crate::state::Escrow;

pub fn process_cancel_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {

    msg!("Cancelling escrow request");

    let [
    maker,
    mint_a,
    escrow_account,
    maker_ata,
    escrow_ata,
    system_program,
    token_program,
    ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    msg!("Accounts unpacked");


    {
        msg!("Validating maker ATA");

        if maker_ata.owner() != token_program.key() {
            msg!("Maker ATA not owned by token program");
            return Err(ProgramError::IllegalOwner);
        }

        msg!("Maker ATA validated");
    }

    assert!(maker.is_signer());
    msg!("Maker is signer");

    let bump = data[0];
    let seed = [b"escrow".as_ref(), maker.key().as_slice(), &[bump]];

    let escrow_state = Escrow::from_account_info(escrow_account)?;
    let escrow_account_pda = derive_address(&seed, None, &crate::ID);

    msg!("Verifying escrow PDA");
    log(&escrow_account_pda);
    log(&escrow_account.key());
    assert_eq!(escrow_account_pda, *escrow_account.key());
    msg!("Escrow PDA verified");

    let amount_to_give = escrow_state.amount_to_give();

    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.key()),
        Seed::from(&bump_bytes)
    ];
    let seeds = Signer::from(&seed);


    msg!("Transferring tokens back to maker");
    Transfer {
        from: escrow_ata,
        to: maker_ata,
        authority: escrow_account,
        amount: amount_to_give,
        token_program: token_program.key(),
    }.invoke_signed(&[seeds.clone()])?;
    msg!("Transfer complete");

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
    // Working but not working as expected
    // Test is failing
    msg!("Returning escrow lamports to maker");
    unsafe {
        let maker_lamports = maker.borrow_mut_lamports_unchecked();
        let escrow_lamports = escrow_account.borrow_mut_lamports_unchecked();

        *maker_lamports = maker_lamports
            .checked_add(*escrow_lamports)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        *escrow_lamports = 0;
    }

    msg!("Cancel instruction complete");
    Ok(())
}
