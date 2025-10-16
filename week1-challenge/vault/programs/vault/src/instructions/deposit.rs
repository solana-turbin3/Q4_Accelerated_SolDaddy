use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_spl::{
    token_interface::{Mint, TokenAccount, TokenInterface},
    associated_token::AssociatedToken,
};
use spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi;
use crate::Vault;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = vault,
        associated_token::token_program = token_program,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut, address = vault.mint)]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Transfer hook program
    pub hook_program: UncheckedAccount<'info>,

    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    /// CHECK: ExtraAccountMetaList PDA
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        seeds = [b"hook", user.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    /// CHECK: User whitelist PDA
    pub user_whitelist: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn deposit_handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::InvalidAmount);

    // Build the base transfer_checked instruction
    let mut transfer_ix = spl_token_2022::instruction::transfer_checked(
        ctx.accounts.token_program.key,
        &ctx.accounts.user_token_account.key(),
        &ctx.accounts.mint.key(),
        &ctx.accounts.vault_token_account.key(),
        ctx.accounts.user.key,
        &[],
        amount,
        ctx.accounts.mint.decimals,
    )?;

    // Start with base account infos for transfer
    let mut cpi_account_infos = vec![
        ctx.accounts.user_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.vault_token_account.to_account_info(),
        ctx.accounts.user.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    ];

    // IMPORTANT: Order MATTERSSSSS
    let additional_accounts = [
        ctx.accounts.hook_program.to_account_info(),           // Hook program FIRST
        ctx.accounts.extra_account_meta_list.to_account_info(), // Then the list itself
        ctx.accounts.user_whitelist.to_account_info(),          // Finally the whitelist
    ];

    // Add extra accounts for transfer hook CPI
    add_extra_accounts_for_execute_cpi(
        &mut transfer_ix,
        &mut cpi_account_infos,
        &ctx.accounts.hook_program.key(),
        ctx.accounts.user_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.vault_token_account.to_account_info(),
        ctx.accounts.user.to_account_info(),
        amount,
        &additional_accounts,
    )?;

    // Execute the CPI with all resolved accounts
    invoke(&transfer_ix, &cpi_account_infos)?;

    msg!("Deposited {} tokens to vault", amount);

    Ok(())
}

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
}
