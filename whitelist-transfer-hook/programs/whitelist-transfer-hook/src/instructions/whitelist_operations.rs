use anchor_lang::{
    prelude::*, 
    system_program
};

use crate::state::whitelist::Whitelist;

// #[derive(Accounts)]
// pub struct WhitelistOperations<'info> {
//     #[account(
//         mut,
//         //address =
//     )]
//     pub admin: Signer<'info>,
//     #[account(
//         mut,
//         seeds = [b"whitelist"],
//         bump,
//     )]
//     pub whitelist: Account<'info, Whitelist>,
//     pub system_program: Program<'info, System>,
// }

#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// The user to whitelist or remove
    pub user: Signer<'info>,

    #[account(
        init_if_needed,
        payer = admin,
        seeds = [b"whitelist", mint.key().as_ref(), user.key().as_ref()],
        bump,
        space = 8 + 32 + 32 + 1 // discriminator + user + mint + bump
    )]
    pub whitelist: Account<'info, Whitelist>,

    /// CHECK: The mint that this whitelist applies to
    pub mint: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// The user to remove
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"whitelist", mint.key().as_ref(), user.key().as_ref()],
        bump,
        close = admin
    )]
    pub whitelist: Account<'info, Whitelist>,

    /// CHECK: The mint that this whitelist applies to
    pub mint: UncheckedAccount<'info>,
}



impl<'info> WhitelistOperations<'info> {
    // pub fn add_to_whitelist(&mut self, address: Pubkey) -> Result<()> {
    //     if !self.whitelist.address.contains(&address) {
    //         self.realloc_whitelist(true)?;
    //         self.whitelist.address.push(address);
    //     }
    //     Ok(())
    // }

    pub fn add_to_whitelist(&mut self) -> Result<()> {

        self.whitelist.set_inner(crate::state::Whitelist {
            user: self.user.key(),
            mint: self.mint.key(),
            bump: self.whitelist.bump,
        });

        Ok(())
    }

    // pub fn remove_from_whitelist(&mut self) -> Result<()> {
    //     // Close the PDA and send lamports to admin
    //     let whitelist_info = self.whitelist.to_account_info();
    //     **self.admin.to_account_info().lamports.borrow_mut() += whitelist_info.lamports();
    //     **whitelist_info.lamports.borrow_mut() = 0;
    //
    //     Ok(())
    // }
    //
    // pub fn realloc_whitelist(&self, is_adding: bool) -> Result<()> {
    //     // Get the account info for the whitelist
    //     let account_info = self.whitelist.to_account_info();
    //
    //     if is_adding {  // Adding to whitelist
    //         let new_account_size = account_info.data_len() + std::mem::size_of::<Pubkey>();
    //         // Calculate rent required for the new account size
    //         let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
    //         // Determine additional rent required
    //         let rent_diff = lamports_required - account_info.lamports();
    //
    //         // Perform transfer of additional rent
    //         let cpi_program = self.system_program.to_account_info();
    //         let cpi_accounts = system_program::Transfer{
    //             from: self.admin.to_account_info(),
    //             to: account_info.clone(),
    //         };
    //         let cpi_context = CpiContext::new(cpi_program, cpi_accounts);
    //         system_program::transfer(cpi_context,rent_diff)?;
    //
    //         // Reallocate the account
    //         account_info.resize(new_account_size)?;
    //         msg!("Account Size Updated: {}", account_info.data_len());
    //
    //     } else {        // Removing from whitelist
    //         let new_account_size = account_info.data_len() - std::mem::size_of::<Pubkey>();
    //         // Calculate rent required for the new account size
    //         let lamports_required = (Rent::get()?).minimum_balance(new_account_size);
    //         // Determine additional rent to be refunded
    //         let rent_diff = account_info.lamports() - lamports_required;
    //
    //         // Reallocate the account
    //         account_info.resize(new_account_size)?;
    //         msg!("Account Size Downgraded: {}", account_info.data_len());
    //
    //         // Perform transfer to refund additional rent
    //         **self.admin.to_account_info().try_borrow_mut_lamports()? += rent_diff;
    //         **self.whitelist.to_account_info().try_borrow_mut_lamports()? -= rent_diff;
    //     }
    //
    //     Ok(())
    // }
}

impl<'info> RemoveWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self) -> Result<()> {
        Ok(())
    }
}
