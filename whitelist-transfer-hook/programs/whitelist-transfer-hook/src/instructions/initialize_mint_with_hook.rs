use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022::Token2022,
    token_interface::Mint,
};

#[derive(Accounts)]
pub struct InitializeMintWithHook<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        mint::decimals = 9,
        mint::authority = mint_authority,
        mint::token_program = token_program,
        extensions::transfer_hook::authority = mint_authority,
        extensions::transfer_hook::program_id = crate::ID,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// CHECK: Mint authority can be any account
    pub mint_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeMintWithHook<'info> {
    pub fn initialize(&mut self) -> Result<()> {
        msg!("Mint initialized with transfer hook!");
        msg!("Mint: {}", self.mint.key());
        msg!("Mint Authority: {}", self.mint_authority.key());
        msg!("Transfer Hook Program: {}", crate::ID);
        Ok(())
    }
}
