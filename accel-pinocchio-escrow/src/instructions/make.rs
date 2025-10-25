use pinocchio::{
    account_info::AccountInfo, instruction::{Seed, Signer}, msg, pubkey::log, sysvars::{rent::Rent, Sysvar}, ProgramResult
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::state::Escrow;

pub fn process_make_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {

    msg!("Processing Make instruction");

    let [
    maker,
    mint_a,
    mint_b,
    escrow_account,
    maker_ata,
    escrow_ata,
    system_program,
    token_program,
    _associated_token_program,
    _rent_sysvar @ ..
    ] = accounts else {
        return Err(pinocchio::program_error::ProgramError::NotEnoughAccountKeys);
    };

    msg!("Accounts unpacked");

    {
        msg!("Validating maker ATA");

        if maker_ata.owner() != token_program.key() {
            msg!("ATA not owned by token program");
            return Err(pinocchio::program_error::ProgramError::IllegalOwner);
        }

        msg!("ATA owner validated");
    }

    msg!("ATA validation complete");
    msg!("Deriving PDA");

    let bump = data[0];
    let seed = [b"escrow".as_ref(), maker.key().as_slice(), &[bump]];

    let escrow_account_pda = derive_address(&seed, None, &crate::ID);
    msg!("PDA derived");
    log(&escrow_account_pda);
    log(&escrow_account.key());

    assert_eq!(escrow_account_pda, *escrow_account.key());
    msg!("PDA verified");

    let amount_to_receive = unsafe{ *(data.as_ptr().add(1) as *const u64) };
    let amount_to_give = unsafe{ *(data.as_ptr().add(9) as *const u64) };
    msg!("Amounts parsed");

    let bump_bytes = [bump.to_le()];
    let seed = [Seed::from(b"escrow"), Seed::from(maker.key()), Seed::from(&bump_bytes)];
    let seeds = Signer::from(&seed);

    if escrow_account.owner() != &crate::ID {
        msg!("Creating escrow account");

        CreateAccount {
            from: maker,
            to: escrow_account,
            lamports: Rent::get()?.minimum_balance(Escrow::LEN),
            space: Escrow::LEN as u64,
            owner: &crate::ID,
        }.invoke_signed(&[seeds.clone()])?;

        msg!("Account created");

        {
            msg!("Initializing state");
            let escrow_state = Escrow::from_account_info(escrow_account)?;

            escrow_state.set_maker(maker.key());
            escrow_state.set_mint_a(mint_a.key());
            escrow_state.set_mint_b(mint_b.key());
            escrow_state.set_amount_to_receive(amount_to_receive);
            escrow_state.set_amount_to_give(amount_to_give);
            escrow_state.bump = data[0];

            msg!("State initialized");
        }

        msg!("State write complete");
    }
    else {
        msg!("Escrow already exists");
        return Err(pinocchio::program_error::ProgramError::IllegalOwner);
    }

    msg!("Creating escrow ATA");

    pinocchio_associated_token_account::instructions::CreateIdempotent {
        funding_account: maker,
        account: escrow_ata,
        wallet: escrow_account,
        mint: mint_a,
        system_program: system_program,
        token_program: token_program,
    }.invoke()?;

    msg!("ATA created");
    msg!("Transferring tokens");

    // FIXED: Pass token_program as a parameter
    pinocchio_token_2022::instructions::Transfer {
        from: maker_ata,
        to: escrow_ata,
        authority: maker,
        amount: amount_to_give,
        token_program: token_program.key(),
    }.invoke()?;

    msg!("Transfer complete");
    msg!("Make instruction success");

    Ok(())
}
