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
use bytemuck::{Pod, Zeroable};
use core::mem::size_of;
use crate::constants::{MAX_CONTRIBUTION_PERCENTAGE, PERCENTAGE_SCALER, SECONDS_TO_DAYS};
use crate::error::FundraiserError;
use crate::state::{Fundraiser, Contributor};



#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ContributeIxData {
    pub amount: u64,
}

impl ContributeIxData {
    pub const LEN: usize = 8;

    #[inline(always)]
    pub fn load_ix_data(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() < Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Create aligned buffer on stack
        let mut aligned_data = [0u8; Self::LEN];
        aligned_data.copy_from_slice(&bytes[..Self::LEN]);

        Ok(*bytemuck::from_bytes::<Self>(&aligned_data))
    }
}

pub fn process_contribute(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [
        contributor,
        _mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _token_program,
        _system_program,
        rent_sysvar,
        ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let ix_data = ContributeIxData::load_ix_data(data)?;

    if !contributor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    // Create contributor account if it doesn't exist
    if contributor_account.data_is_empty() {
        use pinocchio::sysvars::rent::Rent;
        use pinocchio_system::instructions::CreateAccount;

        let rent = Rent::from_account_info(rent_sysvar)?;

        let seeds = &[
            b"contributor",
            fundraiser.key().as_ref(),
            contributor.key().as_ref(),
        ];
        let (pda, bump) = pubkey::find_program_address(seeds, &crate::ID);

        if pda != *contributor_account.key() {
            return Err(ProgramError::InvalidSeeds);
        }

        let bump_seed = [bump];
        let signer_seeds: [Seed; 4] = [
            Seed::from(b"contributor"),
            Seed::from(fundraiser.key().as_ref()),
            Seed::from(contributor.key().as_ref()),
            Seed::from(&bump_seed),
        ];
        let signer = Signer::from(&signer_seeds);

        CreateAccount {
            from: contributor,
            to: contributor_account,
            lamports: rent.minimum_balance(Contributor::LEN),
            space: Contributor::LEN as u64,
            owner: &crate::ID,
        }.invoke_signed(&[signer])?;


        let new_contributor = Contributor::from_account_info(contributor_account)?;
        new_contributor.amount = 0;
    }

    let mut contributor_data = Contributor::from_account_info(contributor_account)?;

    let min_amount = 1;
    if ix_data.amount < min_amount {
        return Err(FundraiserError::ContributionTooSmall.into());
    }

    let max_amount = (fundraiser_data.amount_to_raise * MAX_CONTRIBUTION_PERCENTAGE) / PERCENTAGE_SCALER;
    if ix_data.amount > max_amount {
        return Err(FundraiserError::ContributionTooBig.into());
    }

    let current_time = Clock::get()?.unix_timestamp;
    let elapsed_days = ((current_time - fundraiser_data.time_started) / SECONDS_TO_DAYS) as u8;
    if elapsed_days > fundraiser_data.duration {
        return Err(FundraiserError::FundraiserEnded.into());
    }

    let contributor_new_amount = contributor_data.amount + ix_data.amount;
    if contributor_new_amount > max_amount {
        return Err(FundraiserError::MaximumContributionsReached.into());
    }

    let amount = ix_data.amount;
    let transfer_ix = Transfer {
        from: contributor_ata,
        to: vault,
        authority: contributor,
        amount
    };

    transfer_ix.invoke()?;

    fundraiser_data.current_amount += amount;
    contributor_data.amount += amount;

    Ok(())
}
