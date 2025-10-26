use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::{Seed, Signer};
use pinocchio::program_error::ProgramError;
use pinocchio::{pubkey, ProgramResult};
use pinocchio::sysvars::clock::Clock;
use pinocchio::sysvars::rent::Rent;
use pinocchio::sysvars::Sysvar;
use pinocchio_associated_token_account::instructions::CreateIdempotent;
use pinocchio_system::instructions::CreateAccount;
use crate::constants::MIN_AMOUNT_TO_RAISE;
use crate::state::Fundraiser;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct InitializeIxData {
    pub amount: u64,
    pub duration: u8,
}

impl InitializeIxData {
    pub const LEN: usize = core::mem::size_of::<Self>();

    #[inline(always)]
    pub unsafe fn load_ix_data(bytes: &[u8]) -> Result<Self, ProgramError> {
        if bytes.len() != Self::LEN {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(*(bytes.as_ptr() as *const Self))
    }
}

pub fn process_initialize(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let [maker,
        mint_to_raise,
        fundraiser,
        vault,
        system_account,
        token_program,
        rent_account,
        ..
    ] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    let ix_data = unsafe { InitializeIxData::load_ix_data(data)? };

    // Basic checks
    if !maker.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !fundraiser.data_is_empty() {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if ix_data.amount < MIN_AMOUNT_TO_RAISE {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Load rent from the passed account
    let rent = Rent::from_account_info(rent_account)?;

    // Clock for timestamp
    let clock = Clock::get()?;
    let time_started = clock.unix_timestamp;

    // Derive fundraiser PDA
    let seeds = &[b"fundraiser", maker.key().as_ref()];
    let (pda_fundraiser, bump) = pubkey::find_program_address(seeds, &crate::ID);

    if pda_fundraiser != *fundraiser.key() {
        return Err(ProgramError::InvalidAccountOwner);
    }

    let bump_seed = [bump];
    let fundraiser_seeds: [Seed; 3] = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key().as_ref()),
        Seed::from(&bump_seed),
    ];
    let fundraiser_signer = Signer::from(&fundraiser_seeds);

    // Create the fundraiser account using rent-exempt lamports
    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: rent.minimum_balance(Fundraiser::LEN),
        space: Fundraiser::LEN as u64,
        owner: &crate::ID,
    }.invoke_signed(&[fundraiser_signer.clone()])?;

    CreateIdempotent {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint_to_raise,
        system_program: system_account,
        token_program,
    }.invoke_signed(&[fundraiser_signer])?;

    // Initialize the fundraiser state
    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;
    fundraiser_data.new(
        maker.key(),
        mint_to_raise.key(),
        ix_data.amount,
        time_started,
        ix_data.duration,
        bump,
    );

    Ok(())
}
