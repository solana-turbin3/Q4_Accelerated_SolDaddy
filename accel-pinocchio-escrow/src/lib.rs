use pinocchio::{account_info::AccountInfo, entrypoint, pubkey::Pubkey, ProgramResult};

use crate::instructions::EscrowInstrctions;

mod tests;
mod state;
mod instructions;

entrypoint!(process_instruction);

pinocchio_pubkey::declare_id!("4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT");

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {

    assert_eq!(program_id, &ID);

    let (discriminator, data) = instruction_data.split_first()
        .ok_or(pinocchio::program_error::ProgramError::InvalidInstructionData)?;

    match EscrowInstrctions::try_from(discriminator)? {
        EscrowInstrctions::Make => instructions::process_make_instruction(accounts, data)?,
        EscrowInstrctions::Take => instructions::process_take_instruction(accounts)?,
        EscrowInstrctions::Cancel => instructions::process_cancel_instruction(accounts, data)?,
        // EscrowInstrctions::MakeV2 => instructions::process_make_instruction_v2(accounts, data)?,
        _ => return Err(pinocchio::program_error::ProgramError::InvalidInstructionData),
    }
    Ok(())
}