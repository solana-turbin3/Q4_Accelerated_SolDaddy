#![allow(unexpected_cfgs)]
use pinocchio::{account_info::AccountInfo,entrypoint, program_error::ProgramError, pubkey::Pubkey, ProgramResult};

pub mod instructions;
pub use instructions::*;
pub mod state;
pub mod constants;
pub mod error;
pub mod tests;


entrypoint!(process_instruction);


pinocchio_pubkey::declare_id!("BytFyQcJjBVSH6gARHCixGFa4wca1K3zERKGf3ZGCQVt");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {

    assert_eq!(program_id, &ID);

    
    let (discriminator, rest_data) = data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match FundraiserInstructions::try_from(discriminator)? {
        FundraiserInstructions::Initialize => {
            process_initialize(accounts, rest_data)?
        }

        FundraiserInstructions::Contribute => {
            process_contribute(accounts, rest_data)?
        }

        FundraiserInstructions::Refund => {
            process_refund(accounts)?
        }

        FundraiserInstructions::Finalize =>{
            process_finalize(accounts)?
        }
    }

    Ok(())
}
