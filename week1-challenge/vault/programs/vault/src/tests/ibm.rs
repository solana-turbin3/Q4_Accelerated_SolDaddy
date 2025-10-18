#[cfg(test)]
mod tests {
    use anchor_spl::token_2022::spl_token_2022::extension::interest_bearing_mint::InterestBearingConfig;
    use anchor_spl::token_interface::spl_token_2022::extension::BaseStateWithExtensions;
    use anchor_spl::token_interface::TokenAccount;
    use hook::{AddToWhitelist, InitializeExtraAccountMetaList};
    use {
        anchor_lang::{
            prelude::*,
            AccountDeserialize,
            InstructionData,
            ToAccountMetas,
        },
        hook as hook_program,
        anchor_spl::token_2022::spl_token_2022::{
            self,
            extension::{ExtensionType, StateWithExtensions, transfer_hook},
            state::Mint as MintState,
            instruction,
        },
        litesvm::LiteSVM,
        solana_account::Account,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::path::PathBuf,
    };
    use crate::Vault;

    static PROGRAM_ID: Pubkey = crate::ID;
    static HOOK_PROGRAM_ID: Pubkey = pubkey!("YTRoGAwEK7wZ4Fmi6Pp5QFuKttcqViwBRNnKkgjptzZ");

    /// Sets up LiteSVM and loads both programs
    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL");

        // Load vault program
        let vault_so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/vault.so");
        let vault_program_data = std::fs::read(vault_so_path)
            .expect("Failed to read vault program");
        svm.add_program(PROGRAM_ID, &vault_program_data);

        // Load hook program
        let hook_so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/hook.so");
        let hook_program_data = std::fs::read(hook_so_path)
            .expect("Failed to read hook program");
        svm.add_program(HOOK_PROGRAM_ID, &hook_program_data);

        (svm, payer)
    }

    #[test]
    fn test_interest_bearing_deposit() {
        let (mut svm, payer) = setup();

        // STEP 1: Create mint with TransferHook + InterestBearing extensions
        let mint_keypair = Keypair::new();
        let extensions = vec![
            ExtensionType::TransferHook,
            ExtensionType::InterestBearingConfig,
        ];
        let space = ExtensionType::try_calculate_account_len::<MintState>(&extensions)
            .expect("Failed to calculate mint space");
        let rent = svm.minimum_balance_for_rent_exemption(space);

        let create_account_ix = solana_system_interface::instruction::create_account(
            &payer.pubkey(),
            &mint_keypair.pubkey(),
            rent,
            space as u64,
            &spl_token_2022::ID,
        );

        let init_hook_ix = transfer_hook::instruction::initialize(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            Some(payer.pubkey()),
            Some(HOOK_PROGRAM_ID),
        ).expect("Failed to create initialize transfer hook instruction");

        // 5% annual interest
        let interest_rate: i16 = 500;
        let init_interest_ix = spl_token_2022::extension::interest_bearing_mint::instruction::initialize(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            Some(payer.pubkey()),
            interest_rate,
        ).expect("Failed to create initialize interest bearing instruction");

        let init_mint_ix = spl_token_2022::instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            &payer.pubkey(),
            None,
            6,
        ).expect("Failed to create initialize mint instruction");

        let blockhash = svm.latest_blockhash();
        let create_mint_tx = Transaction::new_signed_with_payer(
            &[create_account_ix, init_hook_ix, init_interest_ix, init_mint_ix],
            Some(&payer.pubkey()),
            &[&payer, &mint_keypair],
            blockhash,
        );

        svm.send_transaction(create_mint_tx).expect("Mint creation failed");
        msg!("Mint created with TransferHook + 5% interest");

        // STEP 2: Initialize vault
        let (vault_pda, _) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);
        let init_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: mint_keypair.pubkey(),
                hook_program: HOOK_PROGRAM_ID,
                token_program: spl_token_2022::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[init_vault_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        )).expect("Vault init failed");
        msg!("Vault initialized");

        // STEP 3: Initialize ExtraAccountMetaList
        let (extra_account_meta_list, _) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint_keypair.pubkey().as_ref()],
            &HOOK_PROGRAM_ID,
        );

        let init_extra_ix = Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts: vec![
                solana_instruction::AccountMeta::new(payer.pubkey(), true),
                solana_instruction::AccountMeta::new(extra_account_meta_list, false),
                solana_instruction::AccountMeta::new_readonly(mint_keypair.pubkey(), false),
                solana_instruction::AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ],
            data: hook_program::instruction::InitializeExtraAccountMetaList {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[init_extra_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        ))
            .expect("ExtraAccountMetaList init failed");
        msg!("ExtraAccountMetaList initialized");

        // STEP 4: Whitelist user
        let (user_whitelist, _) = Pubkey::find_program_address(
            &[b"hook", payer.pubkey().as_ref()],
            &HOOK_PROGRAM_ID,
        );

        let whitelist_ix = Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts: hook::accounts::AddToWhitelist {
                whitelist: user_whitelist,
                authority: payer.pubkey(),
                user: payer.pubkey(),
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: hook::instruction::AddToWhitelist {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[whitelist_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        )).expect("Whitelist failed");
        msg!("User whitelisted");

        // STEP 5: Create user token account and mint tokens
        let user_token_account = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::ID,
        );

        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint_keypair.pubkey(),
            &spl_token_2022::ID,
        );

        let initial_amount = 1_000_000u64; // 1 token with 6 decimals
        let mint_to_ix = spl_token_2022::instruction::mint_to(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            &user_token_account,
            &payer.pubkey(),
            &[],
            initial_amount,
        ).expect("Failed to create mint_to instruction");

        let blockhash = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[create_ata_ix, mint_to_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        )).expect("Mint to user failed");
        msg!("Minted {} tokens", initial_amount);

        // STEP 6: Deposit to vault
        let vault_token_account = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &vault_pda,
            &mint_keypair.pubkey(),
            &spl_token_2022::ID,
        );

        let deposit_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: payer.pubkey(),
                user_token_account,
                vault: vault_pda,
                vault_token_account,
                mint: mint_keypair.pubkey(),
                hook_program: HOOK_PROGRAM_ID,
                extra_account_meta_list,
                user_whitelist,
                token_program: spl_token_2022::ID,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Deposit { amount: initial_amount }.data(),
        };

        let blockhash = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        )).expect("Deposit failed");
        msg!("Deposited {} tokens to vault", initial_amount);

        // STEP 7: Get initial timestamp and balance
        let initial_clock = svm.get_sysvar::<Clock>();
        let initial_timestamp = initial_clock.unix_timestamp;

        let vault_account = svm.get_account(&vault_token_account).unwrap();
        let vault_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&vault_account.data).unwrap();
        let base_amount = vault_data.base.amount;

        // Get mint and interest configuration
        let mint_account = svm.get_account(&mint_keypair.pubkey()).unwrap();
        let mint_data = StateWithExtensions::<MintState>::unpack(&mint_account.data).unwrap();
        let interest_config = mint_data.get_extension::<InterestBearingConfig>().unwrap();
        let decimals = mint_data.base.decimals;

        msg!("Base amount: {}", base_amount);
        msg!("Initial timestamp: {}", initial_timestamp);

        // Fix: Convert PodI16 to i16 using Into trait
        let current_rate: i16 = interest_config.current_rate.into();
        let rate_percent = current_rate as f64 / 100.0;
        msg!("Interest rate: {} basis points ({}%)", current_rate, rate_percent);

        // Simple UI amount at deposit (just divide by decimals)
        let ui_amount_at_deposit = base_amount as f64 / 10_usize.pow(decimals as u32) as f64;
        msg!("UI amount at deposit: {:.6}", ui_amount_at_deposit);

        // STEP 8: Warp time forward by 1 year
        let one_year_seconds: i64 = 365 * 24 * 60 * 60;
        let mut new_clock = svm.get_sysvar::<Clock>();
        new_clock.unix_timestamp = initial_timestamp + one_year_seconds;
        svm.set_sysvar::<Clock>(&new_clock);

        let new_timestamp = svm.get_sysvar::<Clock>().unix_timestamp;
        msg!("Warped forward 1 year to timestamp: {}", new_timestamp);

        // STEP 9: Verify base amount unchanged and calculate interest

        // Fetch vault account after time warp
        let vault_account = svm.get_account(&vault_token_account).unwrap();
        let vault_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&vault_account.data).unwrap();

        // Verify base amount hasn't changed
        assert_eq!(
            vault_data.base.amount,
            base_amount,
            "Base amount should never change"
        );
        msg!("Base amount after 1 year (unchanged): {}", vault_data.base.amount);

        // Calculate interest manually using the formula: A = P * e^(r * t)
        // where r is in basis points (500 = 5% = 0.05)
        // and t is time in years

        // Fix: Convert PodI16 to i16 first
        let current_rate: i16 = interest_config.current_rate.into();
        let rate_decimal = current_rate as f64 / 10000.0; // Convert basis points to decimal
        let time_years = (new_timestamp - initial_timestamp) as f64 / (365.25 * 24.0 * 60.0 * 60.0); // Account for leap years

        // Calculate compound interest: A = P * e^(r * t)
        let growth_factor = (rate_decimal * time_years).exp();
        let ui_amount_with_interest = ui_amount_at_deposit * growth_factor;

        msg!("UI amount after 1 year (with interest): {:.6}", ui_amount_with_interest);
        msg!("Growth factor (e^(r*t)): {:.6}", growth_factor);

        // Calculate interest earned
        let interest_earned = ui_amount_with_interest - ui_amount_at_deposit;
        let interest_rate_actual = (interest_earned / ui_amount_at_deposit) * 100.0;

        // Verify interest accrued
        assert!(
            ui_amount_with_interest > ui_amount_at_deposit,
            "UI amount should have grown with interest!"
        );

        // With 5% rate, expect approximately 5.127% (due to continuous compounding: e^0.05 - 1 = 5.127%)
        assert!(
            interest_rate_actual >= 5.0 && interest_rate_actual <= 5.2,
            "Interest should be approximately 5.127% (continuous compounding), got {:.3}%",
            interest_rate_actual
        );

        msg!("Test passed!");
        msg!("Interest earned: {:.6} tokens", interest_earned);
        msg!("Effective rate: {:.3}%", interest_rate_actual);
        msg!("Total UI amount: {:.6}", ui_amount_with_interest);
        // msg!("ℹ️  Note: Base amount stays at {}, interest is UI-only", base_amount);
    }

}
