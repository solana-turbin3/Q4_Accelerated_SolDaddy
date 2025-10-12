use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use crate::Vault;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = 8 + std::mem::size_of::<Vault>(),
        seeds = [b"vault".as_ref()],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(
        init,
        payer = user,
        mint::decimals = 6,
        mint::authority = user,
    )]
    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,

    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.bump = ctx.bumps.vault;
    vault.mint = ctx.accounts.mint.key();
    Ok(())
}
