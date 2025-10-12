use anchor_lang::prelude::*;

#[account]
pub struct Vault{
    pub mint: Pubkey,
    pub bump: u8,
}