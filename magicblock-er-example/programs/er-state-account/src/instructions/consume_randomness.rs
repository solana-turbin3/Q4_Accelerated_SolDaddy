use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::rnd::random_u64;
use crate::state::UserAccount;

#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,

    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,  // Removed seeds validation
}

impl<'info> ConsumeRandomness<'info> {
    pub fn consume(&mut self, randomness: [u8; 32]) -> Result<()> {
        msg!("Consuming randomness...");

        let random_value = random_u64(&randomness);
        self.user_account.random_value = random_value;

        msg!("Random value updated: {}", random_value);
        Ok(())
    }
}
