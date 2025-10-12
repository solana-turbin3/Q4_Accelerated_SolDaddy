use std::cell::RefMut;
use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::spl_token_2022::{
        extension::{
            transfer_hook::TransferHookAccount,
            BaseStateWithExtensionsMut,
            PodStateWithExtensionsMut
        },
        pod::PodAccount
    },
    token_interface::{
        Mint,
        TokenAccount
    }
};
use crate::state::Whitelist;

#[derive(Accounts)]
pub struct TransferHook<'info> {
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    pub source_token: InterfaceAccount<'info, TokenAccount>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(token::mint = mint)]
    pub destination_token: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: source token account owner
    pub owner: UncheckedAccount<'info>,

    /// CHECK: ExtraAccountMetaList Account
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    // Index 5: Sender's whitelist (from ExtraAccountMeta)
    #[account(
        seeds = [b"whitelist", mint.key().as_ref(), owner.key().as_ref()],
        bump = sender_whitelist.bump,
    )]
    pub sender_whitelist: Account<'info, Whitelist>,

    // Index 6: Destination owner (resolved from destination token account)
    /// CHECK: Destination owner resolved from token account
    pub destination_owner: UncheckedAccount<'info>,

    // Index 7: Destination owner's whitelist
    #[account(
        seeds = [b"whitelist", mint.key().as_ref(), destination_owner.key().as_ref()],
        bump = destination_whitelist.bump,
    )]
    pub destination_whitelist: Account<'info, Whitelist>,
}

impl<'info> TransferHook<'info> {
    pub fn transfer_hook(&mut self, _amount: u64) -> Result<()> {
        self.check_is_transferring()?;

        // Validate sender is whitelisted
        require!(
            self.sender_whitelist.user == self.owner.key()
            && self.sender_whitelist.mint == self.mint.key(),
            ErrorCode::SenderNotWhitelisted
        );

        // Validate destination owner is whitelisted
        require!(
            self.destination_whitelist.user == self.destination_owner.key()
            && self.destination_whitelist.mint == self.mint.key(),
            ErrorCode::DestinationNotWhitelisted
        );

        msg!("Transfer approved: both parties whitelisted");
        Ok(())
    }

    fn check_is_transferring(&mut self) -> Result<()> {
        let source_token_info = self.source_token.to_account_info();
        let mut account_data_ref: RefMut<&mut [u8]> = source_token_info.try_borrow_mut_data()?;
        let mut account = PodStateWithExtensionsMut::<PodAccount>::unpack(*account_data_ref)?;
        let account_extension = account.get_extension_mut::<TransferHookAccount>()?;

        require!(
            bool::from(account_extension.transferring),
            ErrorCode::NotTransferring
        );

        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Sender is not whitelisted")]
    SenderNotWhitelisted,
    #[msg("Destination is not whitelisted")]
    DestinationNotWhitelisted,
    #[msg("Not currently transferring")]
    NotTransferring,
}
