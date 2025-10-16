use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_spl::{
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi;
use crate::Vault;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key(),
        constraint = user_token_account.mint == mint.key(),
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"vault"],
        bump = vault.bump,
    )]
    pub vault: Account<'info, Vault>,

    #[account(
        mut,
        constraint = vault_token_account.owner == vault.key(),
        constraint = vault_token_account.mint == mint.key(),
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(mut)]
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
        seeds = [b"hook", vault.key().as_ref()],
        bump,
        seeds::program = hook_program.key(),
    )]
    /// CHECK: Vault whitelist PDA
    pub vault_whitelist: UncheckedAccount<'info>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub fn withdraw_handler(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, WithdrawError::InvalidAmount);

    // Build the base transfer_checked instruction
    let mut transfer_ix = spl_token_2022::instruction::transfer_checked(
        ctx.accounts.token_program.key,
        &ctx.accounts.vault_token_account.key(),
        &ctx.accounts.mint.key(),
        &ctx.accounts.user_token_account.key(),
        &ctx.accounts.vault.key(),
        &[],
        amount,
        ctx.accounts.mint.decimals,
    )?;

    // Start with base account infos
    let mut cpi_account_infos = vec![
        ctx.accounts.vault_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.user_token_account.to_account_info(),
        ctx.accounts.vault.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
    ];

    // Additional accounts for the hook
    let additional_accounts = [
        ctx.accounts.hook_program.to_account_info(),
        ctx.accounts.extra_account_meta_list.to_account_info(),
        ctx.accounts.vault_whitelist.to_account_info(),
    ];

    // Add extra accounts for transfer hook CPI
    add_extra_accounts_for_execute_cpi(
        &mut transfer_ix,
        &mut cpi_account_infos,
        &ctx.accounts.hook_program.key(),
        ctx.accounts.vault_token_account.to_account_info(),
        ctx.accounts.mint.to_account_info(),
        ctx.accounts.user_token_account.to_account_info(),
        ctx.accounts.vault.to_account_info(),
        amount,
        &additional_accounts,
    )?;

    // Execute the CPI with PDA signer
    let vault_seeds = &[b"vault".as_ref(), &[ctx.accounts.vault.bump]];
    let signer_seeds = &[&vault_seeds[..]];

    invoke_signed(&transfer_ix, &cpi_account_infos, signer_seeds)?;

    msg!("Withdrew {} tokens from vault to user", amount);

    Ok(())
}

#[error_code]
pub enum WithdrawError {
    #[msg("Amount must be greater than zero")]
    InvalidAmount,
}
