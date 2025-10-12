mod tests;

use anchor_lang::prelude::*;

// Program ID will be generated automatically by Anchor
declare_id!("YTRoGAwEK7wZ4Fmi6Pp5QFuKttcqViwBRNnKkgjptzZ");

#[program]
pub mod transfer_hook {
    use super::*;

    /// Add a user to the whitelist (creates PDA)
    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>) -> Result<()> {
        let whitelist = &mut ctx.accounts.whitelist;
        whitelist.user = ctx.accounts.user.key();
        msg!("Added user {} to whitelist", whitelist.user);
        Ok(())
    }

    /// Remove a user from the whitelist (closes PDA)
    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>) -> Result<()> {
        msg!("Removed user {} from whitelist", ctx.accounts.whitelist.user);
        Ok(())
    }

    /// Validate a transfer: only checks whitelist
    pub fn validate_transfer(ctx: Context<ValidateTransfer>) -> Result<()> {
        // Check that the whitelist PDA exists
        require!(
            ctx.accounts.whitelist.user == ctx.accounts.from.key(),
            HookError::NotWhitelisted
        );


        msg!(
            "Transfer approved for user {}",
            ctx.accounts.from.key()
        );

        Ok(())
    }
}

/// Each whitelisted user gets their own PDA
#[account]
pub struct WhitelistEntry {
    pub user: Pubkey,
}

#[derive(Accounts)]
pub struct AddToWhitelist<'info> {
    #[account(
        init,
        space = 8 + std::mem::size_of::<WhitelistEntry>(),
        payer = authority,
        seeds = [b"hook", user.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, WhitelistEntry>,

    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: User being added
    pub user: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveFromWhitelist<'info> {
    #[account(mut, close = authority)]
    pub whitelist: Account<'info, WhitelistEntry>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ValidateTransfer<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    /// CHECK: Destination account
    pub to: AccountInfo<'info>,

    #[account(
        seeds = [b"hook", from.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, WhitelistEntry>,
}

#[error_code]
pub enum HookError {
    #[msg("User is not whitelisted.")]
    NotWhitelisted,
}
