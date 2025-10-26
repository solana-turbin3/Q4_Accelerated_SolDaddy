use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey,
    ProgramResult,
    sysvars::clock::Clock,
    sysvars::Sysvar,
};
use pinocchio_token::instructions::Transfer;
use crate::constants::SECONDS_TO_DAYS;
use crate::error::FundraiserError;
use crate::state::{Fundraiser, Contributor};

pub fn process_refund(accounts: &[AccountInfo]) -> ProgramResult {
    let [
    contributor,
    maker,
    mint_to_raise,
    fundraiser,
    contributor_account,
    contributor_ata,
    vault,
    token_program,
    ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut fundraiser_data = Fundraiser::from_account_info(fundraiser)?;
    let mut contributor_data = Contributor::from_account_info(contributor_account)?;

    // Check fundraiser ended
    let current_time = Clock::get()?.unix_timestamp;
    let elapsed_days = ((current_time - fundraiser_data.time_started) / SECONDS_TO_DAYS) as u8;
    if elapsed_days <= fundraiser_data.duration {
        return Err(FundraiserError::FundraiserNotEnded.into());
    }

    // Ensure target NOT met
    if fundraiser_data.current_amount >= fundraiser_data.amount_to_raise {
        return Err(FundraiserError::TargetMet.into());
    }

    if contributor_data.amount == 0 {
        return Err(FundraiserError::NoContribution.into());
    }

    let refund_amount = contributor_data.amount;

    // ✅ Use the bump stored in fundraiser_data
    let bump_bytes = [fundraiser_data.bump];
    let fundraiser_seeds: [Seed; 3] = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key().as_ref()),
        Seed::from(&bump_bytes),
    ];
    let fundraiser_signer = Signer::from(&fundraiser_seeds);

    // Transfer tokens from vault → contributor ATA
    Transfer {
        from: vault,
        to: contributor_ata,
        authority: fundraiser,
        amount: refund_amount,
    }.invoke_signed(&[fundraiser_signer])?;

    // Update state
    fundraiser_data.current_amount = fundraiser_data.current_amount.saturating_sub(refund_amount);
    contributor_data.amount = 0;

    Ok(())
}
