use pinocchio::account_info::AccountInfo;
use pinocchio::{msg, ProgramResult};
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::log;
use pinocchio_pubkey::derive_address;
use pinocchio_token::instructions::Transfer;
use crate::state::Escrow;

pub fn process_cancel_instruction(
    accounts: &[AccountInfo],
    data: &[u8],
) ->ProgramResult{

    msg!("Cancelling your escrow request");
    let [
    maker,
    mint_a,
    escrow_account,
    maker_ata,
    escrow_ata,
    // system_program,
    // token_program,
    // _associated_token_program,
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let maker_ata_state = pinocchio_token::state::TokenAccount::from_account_info(&maker_ata)?;
    if maker_ata_state.owner() != maker.key() {
        return Err(pinocchio::program_error::ProgramError::IllegalOwner);
    }
    if maker_ata_state.mint() != mint_a.key() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    let bump = data[0];
    let seed = [b"escrow".as_ref(), maker.key().as_slice(), &[bump]];
    let seeds = &seed[..];

    let escrow_state = Escrow::from_account_info(escrow_account)?;
    let escrow_account_pda = derive_address(&seed, None, &crate::ID);
    log(&escrow_account_pda);
    log(&escrow_account.key());
    assert_eq!(escrow_account_pda, *escrow_account.key());

    let amount_to_give = escrow_state.amount_to_give();

    let bump_bytes = [bump];
    let seed = [
        Seed::from(b"escrow"),
        Seed::from(maker.key()),
        Seed::from(&bump_bytes)
    ];
    let seeds = Signer::from(&seed);

    Transfer{
        from: escrow_ata,
        to: maker_ata,
        authority: escrow_account,
        amount: amount_to_give,
    }.invoke_signed(&[seeds])?;
    Ok(())
}