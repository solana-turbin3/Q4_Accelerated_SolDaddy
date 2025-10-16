use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use crate::Vault;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + std::mem::size_of::<Vault>(),
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,

    /// The mint must already exist, created off-chain with a transfer hook attached
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Transfer hook program (We don't really need it here)
    pub hook_program: UncheckedAccount<'info>,

    /// It will support both
    pub token_program: Interface<'info, TokenInterface>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.bump = ctx.bumps.vault;
    vault.mint = ctx.accounts.mint.key();

    msg!("Vault initialized for mint with hook: {}", ctx.accounts.hook_program.key());
    Ok(())
}
