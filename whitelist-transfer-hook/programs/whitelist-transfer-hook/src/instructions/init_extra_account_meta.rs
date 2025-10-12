use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList
};

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: ExtraAccountMetaList Account, must use these seeds
    #[account(
        init,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        space = ExtraAccountMetaList::size_of(
            InitializeExtraAccountMetaList::extra_account_metas(&mint.key())?.len()
        )?,
        payer = payer
    )]
    pub extra_account_meta_list: AccountInfo<'info>,
    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeExtraAccountMetaList<'info> {
    pub fn extra_account_metas(mint_pubkey: &Pubkey) -> Result<Vec<ExtraAccountMeta>> {
        Ok(vec![
            // Index 5: Sender's whitelist PDA
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal { bytes: b"whitelist".to_vec() },
                    Seed::AccountKey { index: 1 }, // mint is at index 1
                    Seed::AccountKey { index: 3 }, // owner is at index 3
                ],
                false, // is_signer
                false, // is_writable
            )?,
            // Index 6: Destination owner
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::AccountData {
                        account_index: 2, // destination token account
                        data_index: 32,   // owner field starts at byte 32
                        length: 32,       // pubkey is 32 bytes
                    },
                ],
                false, // is_signer
                false, // is_writable
            )?,
            // Index 7: Destination owner's whitelist PDA
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal { bytes: b"whitelist".to_vec() },
                    Seed::AccountKey { index: 1 }, // mint
                    Seed::AccountKey { index: 6 }, // destination owner from index 6
                ],
                false,
                false,
            )?,
        ])
    }
}
