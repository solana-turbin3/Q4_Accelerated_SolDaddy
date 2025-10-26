use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::{ProgramResult};

use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_token::instructions::{CloseAccount, Transfer};

use pinocchio_token::ID as PINOCCHIO_TOKEN_ID;

use crate::state::Fundraiser;
use crate::error::FundraiserError;


pub fn process_finalize(accounts: &[AccountInfo]) -> ProgramResult {
    let [
    maker,
    mint_to_raise,
    fundraiser,
    vault,
    maker_ata,
    token_program,
    system_program,
    ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;
    let bump = fundraiser_data.bump;

    // Check target met or not
    if fundraiser_data.current_amount < fundraiser_data.amount_to_raise {
        return Err(FundraiserError::TargetNotMet.into());
    }
    
    let bump_bytes = [bump];
    let seeds: [Seed; 3] = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key().as_ref()),
        Seed::from(&bump_bytes),
    ];
    let fundraiser_signer = Signer::from(&seeds);

  
    if maker_ata.data_is_empty() {
        CreateIdempotent {
            funding_account: maker,
            account: maker_ata,
            wallet: maker,
            mint: mint_to_raise,
            system_program,
            token_program,
        }.invoke()?; 
    } else {
        // Verify it's the correct ATA
        if maker_ata.owner() != &PINOCCHIO_TOKEN_ID {
            return Err(ProgramError::IllegalOwner);
        }
    }

    // Transfer all tokens from vault to maker_ata
    let amount_to_transfer = fundraiser_data.current_amount;

    Transfer {
        from: vault,
        to: maker_ata,
        authority: fundraiser,
        amount: amount_to_transfer,
    }.invoke_signed(&[fundraiser_signer.clone()])?;

    // Close vault to reclaim rent
    CloseAccount {
        account: vault,
        destination: maker,
        authority: fundraiser,
    }.invoke_signed(&[fundraiser_signer])?;

    // Close fundraiser account and transfer lamports to maker
    {
        let fundraiser_lamports = fundraiser.lamports();
        *maker.try_borrow_mut_lamports()? += fundraiser_lamports;
        *fundraiser.try_borrow_mut_lamports()? = 0;

    }

    // Zero out data
    fundraiser.realloc(0, false)?;


    Ok(())
}
