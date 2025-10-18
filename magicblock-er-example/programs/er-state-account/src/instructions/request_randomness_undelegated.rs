use anchor_lang::prelude::*;
use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::consts::DEFAULT_QUEUE;
use anchor_lang::solana_program::instruction::AccountMeta;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;
use crate::state::UserAccount;

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomnessUndelegated<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"user", user.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// CHECK: Oracle queue
    #[account(mut, address = DEFAULT_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomnessUndelegated<'info> {
    pub fn request(&mut self, client_seed: u8) -> Result<()> {
        msg!("Requesting randomness while undelegated...");

        // Define the accounts for the callback instruction using SerializableAccountMeta
        let callback_accounts = vec![
            // SerializableAccountMeta {
            //     pubkey: ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY,
            //     is_signer: true,
            //     is_writable: false,
            // },
            // SerializableAccountMeta {
            //     pubkey: self.user.key(),
            //     is_signer: false,
            //     is_writable: false,
            // },
            SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            },
        ];

        // Create the request randomness instruction
        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.user.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: crate::ID,
            callback_discriminator: crate::instruction::ConsumeRandomness::DISCRIMINATOR.to_vec(),
            caller_seed: [client_seed; 32],
            accounts_metas: Some(callback_accounts),
            ..Default::default()
        });


        // Invoke the VRF program using the vrf macro's helper method
        self.invoke_signed_vrf(&self.user.to_account_info(), &ix)?;

        msg!("Randomness request submitted (undelegated)");
        Ok(())
    }
}
