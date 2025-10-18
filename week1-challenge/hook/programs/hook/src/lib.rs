
use anchor_lang::{
    prelude::*,
    solana_program::program_error::ProgramError,
};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta,
    seeds::Seed,
    state::ExtraAccountMetaList,
};
use spl_transfer_hook_interface::instruction::{
    ExecuteInstruction,
    TransferHookInstruction
};

declare_id!("YTRoGAwEK7wZ4Fmi6Pp5QFuKttcqViwBRNnKkgjptzZ");

#[program]
pub mod transfer_hook {
    use super::*;

    pub fn add_to_whitelist(ctx: Context<AddToWhitelist>) -> Result<()> {
        let whitelist = &mut ctx.accounts.whitelist;
        whitelist.user = ctx.accounts.user.key();
        msg!("Added user {} to whitelist", whitelist.user);
        Ok(())
    }

    pub fn remove_from_whitelist(ctx: Context<RemoveFromWhitelist>) -> Result<()> {
        msg!("Removed user {} from whitelist", ctx.accounts.whitelist.user);
        Ok(())
    }

    pub fn initialize_extra_account_meta_list(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {
        // Define the whitelist account as an extra account meta
        // Index 0-3: source_token, mint, destination_token, owner
        // Index 4: extra_account_meta_list
        // Index 5: whitelist PDA
        let account_metas = vec![
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal {
                        bytes: b"hook".to_vec(),
                    },
                    Seed::AccountKey { index: 3 }, // owner at index 3
                ],
                false, // is_signer
                false, // is_writable
            )?,
        ];

        // Calculate account size
        let account_size = ExtraAccountMetaList::size_of(account_metas.len())? as u64;
        let lamports = Rent::get()?.minimum_balance(account_size as usize);

        let mint = ctx.accounts.mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"extra-account-metas",
            mint.as_ref(),
            &[ctx.bumps.extra_account_meta_list],
        ]];

        // Create account
        anchor_lang::system_program::create_account(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.extra_account_meta_list.to_account_info(),
                },
            ).with_signer(signer_seeds),
            lamports,
            account_size,
            ctx.program_id,
        )?;

        // Initialize the account with the extra account metas
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &account_metas,
        )?;

        msg!("Initialized ExtraAccountMetaList");
        Ok(())
    }


    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        msg!("Transfer hook triggered for amount: {}", amount);

        // Check whitelist
        require!(
            ctx.accounts.whitelist.user == ctx.accounts.owner.key(),
            HookError::NotWhitelisted
        );

        msg!(
            "Transfer approved for whitelisted user {}",
            ctx.accounts.owner.key()
        );

        Ok(())
    }


    // Is this even necessary though? Check back later
    pub fn fallback<'info>(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo<'info>],
        data: &[u8],
    ) -> Result<()> {
        let instruction = TransferHookInstruction::unpack(data)?;

        match instruction {
            TransferHookInstruction::Execute { amount } => {
                let amount_bytes = amount.to_le_bytes();
                __private::__global::transfer_hook(program_id, accounts, &amount_bytes)
            }
            _ => return Err(ProgramError::InvalidInstructionData.into()),
        }
    }
}

#[account]
pub struct WhitelistEntry {
    pub user: Pubkey,
}

#[derive(Accounts)]
pub struct InitializeExtraAccountMetaList<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    /// CHECK: It is safe
    pub extra_account_meta_list: AccountInfo<'info>,

    pub mint: InterfaceAccount<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddToWhitelist<'info> {
    #[account(
        init,
        space = 8 + 32,
        payer = authority,
        seeds = [b"hook", user.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, WhitelistEntry>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: User being added to whitelist
    pub user: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveFromWhitelist<'info> {
    #[account(
        mut,
        close = authority,
        seeds = [b"hook", whitelist.user.as_ref()],
        bump
    )]
    pub whitelist: Account<'info, WhitelistEntry>,

    #[account(mut)]
    pub authority: Signer<'info>,
}


#[derive(Accounts)]
pub struct TransferHook<'info> {
    
    #[account(
        token::mint = mint,
        token::authority = owner,
    )]
    // Index 0
    pub source_token: InterfaceAccount<'info, TokenAccount>,
    
    // Index 1
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(token::mint = mint)]
    // Index 2
    pub destination_token: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Source token account owner
    // Index 3
    pub owner: UncheckedAccount<'info>,

    /// CHECK: ExtraAccountMetaList Account
    #[account(
        seeds = [b"extra-account-metas", mint.key().as_ref()],
        bump
    )]
    // Index 4
    pub extra_account_meta_list: UncheckedAccount<'info>,

    #[account(
        seeds = [b"hook", owner.key().as_ref()],
        bump
    )]
    // Index 5: Custom extra account
    pub whitelist: Account<'info, WhitelistEntry>,
}

#[error_code]
pub enum HookError {
    #[msg("User is not whitelisted.")]
    NotWhitelisted,
}
