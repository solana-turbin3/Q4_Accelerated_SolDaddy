#[cfg(test)]
mod tests {
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

    /// Helper to create a mint with transfer hook extension
    fn create_mint_with_hook(
        svm: &mut LiteSVM,
        payer: &Keypair,
        hook_program: Pubkey,
        decimals: u8,
    ) -> Keypair {
        let mint_keypair = Keypair::new();

        let extensions = vec![ExtensionType::TransferHook];
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
            Some(hook_program),
        ).expect("Failed to create initialize transfer hook instruction");

        let init_mint_ix = instruction::initialize_mint(
            &spl_token_2022::ID,
            &mint_keypair.pubkey(),
            &payer.pubkey(),
            None,
            decimals,
        ).expect("Failed to create initialize mint instruction");

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[create_account_ix, init_hook_ix, init_mint_ix],
            Some(&payer.pubkey()),
            &[payer, &mint_keypair],
            blockhash,
        );

        svm.send_transaction(tx).expect("Mint creation failed");
        mint_keypair
    }

    /// Helper to initialize vault
    fn initialize_vault(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey) -> Pubkey {
        let (vault_pda, _) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);

        let init_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: *mint,
                hook_program: HOOK_PROGRAM_ID,
                token_program: spl_token_2022::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[init_vault_ix],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );

        svm.send_transaction(tx).expect("Vault initialization failed");
        vault_pda
    }

    /// Helper to initialize extra account meta list
    fn initialize_extra_account_meta_list(svm: &mut LiteSVM, payer: &Keypair, mint: &Pubkey) -> Pubkey {
        let (extra_account_meta_list, _) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint.as_ref()],
            &HOOK_PROGRAM_ID,
        );

        let init_extra_ix = Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(extra_account_meta_list, false),
                AccountMeta::new_readonly(*mint, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            ],
            data: hook_program::instruction::InitializeExtraAccountMetaList {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[init_extra_ix],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        svm.send_transaction(tx).expect("ExtraAccountMetaList init failed");
        extra_account_meta_list
    }

    /// Helper to whitelist a user
    fn whitelist_user(svm: &mut LiteSVM, payer: &Keypair, user: &Pubkey) -> Pubkey {
        let (whitelist_pda, _) = Pubkey::find_program_address(
            &[b"hook", user.as_ref()],
            &HOOK_PROGRAM_ID,
        );

        let whitelist_ix = Instruction {
            program_id: HOOK_PROGRAM_ID,
            accounts: hook::accounts::AddToWhitelist {
                whitelist: whitelist_pda,
                authority: payer.pubkey(),
                user: *user,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: hook::instruction::AddToWhitelist {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[whitelist_ix],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        svm.send_transaction(tx).expect("Whitelist failed");
        whitelist_pda
    }

    /// Helper to create ATA and mint tokens
    fn mint_tokens_to_user(
        svm: &mut LiteSVM,
        payer: &Keypair,
        mint: &Pubkey,
        amount: u64,
    ) -> Pubkey {
        let user_token_account = anchor_spl::associated_token::get_associated_token_address_with_program_id(
            &payer.pubkey(),
            mint,
            &spl_token_2022::ID,
        );

        let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            mint,
            &spl_token_2022::ID,
        );

        let mint_to_ix = instruction::mint_to(
            &spl_token_2022::ID,
            mint,
            &user_token_account,
            &payer.pubkey(),
            &[],
            amount,
        ).expect("Failed to create mint_to instruction");

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[create_ata_ix, mint_to_ix],
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        );
        svm.send_transaction(tx).expect("Mint to user failed");
        user_token_account
    }

    #[test]
    fn test_init_with_transfer_hook() {
        let (mut svm, payer) = setup();
        let (vault_pda, vault_bump) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);

        let mint_keypair = create_mint_with_hook(&mut svm, &payer, HOOK_PROGRAM_ID, 6);
        msg!("Mint: {}", mint_keypair.pubkey());

        let init_vault_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: mint_keypair.pubkey(),
                hook_program: HOOK_PROGRAM_ID,
                token_program: spl_token_2022::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        let blockhash = svm.latest_blockhash();
        let vault_tx = Transaction::new_signed_with_payer(
            &[init_vault_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        let vault_result = svm.send_transaction(vault_tx).expect("Vault initialization failed");
        msg!("Vault initialized - CU: {}", vault_result.compute_units_consumed);

        // Verify vault
        let vault_account = svm.get_account(&vault_pda).expect("Vault not found");
        let vault_data = Vault::try_deserialize(&mut vault_account.data.as_ref())
            .expect("Failed to deserialize vault");

        assert_eq!(vault_data.bump, vault_bump);
        assert_eq!(vault_data.mint, mint_keypair.pubkey());

        // Verify transfer hook extension
        let mint_account = svm.get_account(&mint_keypair.pubkey()).expect("Mint not found");
        let mint_with_extensions = StateWithExtensions::<MintState>::unpack(&mint_account.data)
            .expect("Failed to unpack mint");

        let hook_extension = mint_with_extensions
            .get_extension::<transfer_hook::TransferHook>()
            .expect("Transfer hook extension not found");

        assert_eq!(
            Option::<Pubkey>::from(hook_extension.program_id),
            Some(HOOK_PROGRAM_ID),
            "Hook program ID mismatch"
        );

        msg!("Test passed!");
    }

    #[test]
    fn test_deposit_without_whitelist() {
        let (mut svm, payer) = setup();

        let mint_keypair = create_mint_with_hook(&mut svm, &payer, HOOK_PROGRAM_ID, 6);
        let vault_pda = initialize_vault(&mut svm, &payer, &mint_keypair.pubkey());
        let extra_account_meta_list = initialize_extra_account_meta_list(&mut svm, &payer, &mint_keypair.pubkey());

        msg!("User NOT whitelisted. Expect deposit to fail");

        let initial_amount = 10_000_000u64;
        let user_token_account = mint_tokens_to_user(&mut svm, &payer, &mint_keypair.pubkey(), initial_amount);

        // Try to deposit (SHOULD FAIL)
        let deposit_amount = 5_000_000u64;
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
                user_whitelist: Default::default(),
                token_program: spl_token_2022::ID,
                associated_token_program: anchor_spl::associated_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Deposit { amount: deposit_amount }.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        match svm.send_transaction(tx) {
            Ok(_) => panic!("Test FAILED: Deposit should have failed for non-whitelisted user!"),
            Err(e) => {
                msg!("Deposit correctly failed: {:?}", e);
                let error_str = format!("{:?}", e);
                if error_str.contains("AccountNotFound") || error_str.contains("3012") {
                    msg!("Failed with expected error: Whitelist PDA doesn't exist");
                }
            }
        }

        // Verify balances unchanged
        let user_account = svm.get_account(&user_token_account).expect("User token account not found");
        let user_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&user_account.data)
            .expect("Failed to unpack user token account");

        assert_eq!(user_data.base.amount, initial_amount, "User balance should be unchanged");
        msg!("Test passed!");
    }

    #[test]
    fn test_deposit_with_whitelist() {
        let (mut svm, payer) = setup();

        let mint_keypair = create_mint_with_hook(&mut svm, &payer, HOOK_PROGRAM_ID, 6);
        let vault_pda = initialize_vault(&mut svm, &payer, &mint_keypair.pubkey());
        let extra_account_meta_list = initialize_extra_account_meta_list(&mut svm, &payer, &mint_keypair.pubkey());
        let user_whitelist = whitelist_user(&mut svm, &payer, &payer.pubkey());

        let initial_amount = 100u64;
        let user_token_account = mint_tokens_to_user(&mut svm, &payer, &mint_keypair.pubkey(), initial_amount);

        // Deposit to vault
        let deposit_amount = 50u64;
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
            }.to_account_metas(None),
            data: crate::instruction::Deposit { amount: deposit_amount }.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        let result = svm.send_transaction(tx).expect("Deposit should succeed");
        msg!("Deposit successful - CU: {}", result.compute_units_consumed);

        // Verify balances
        let user_account = svm.get_account(&user_token_account).expect("User token account not found");
        let user_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&user_account.data)
            .expect("Failed to unpack user token account");

        let vault_account = svm.get_account(&vault_token_account).expect("Vault token account not found");
        let vault_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&vault_account.data)
            .expect("Failed to unpack vault token account");

        assert_eq!(user_data.base.amount, initial_amount - deposit_amount);
        assert_eq!(vault_data.base.amount, deposit_amount);

        msg!("Test passed!");
    }

    #[test]
    fn test_withdraw_with_whitelisted_vault() {
        let (mut svm, payer) = setup();

        let mint_keypair = create_mint_with_hook(&mut svm, &payer, HOOK_PROGRAM_ID, 6);
        let vault_pda = initialize_vault(&mut svm, &payer, &mint_keypair.pubkey());
        let extra_account_meta_list = initialize_extra_account_meta_list(&mut svm, &payer, &mint_keypair.pubkey());

        // Whitelist both user and vault
        let user_whitelist = whitelist_user(&mut svm, &payer, &payer.pubkey());
        let vault_whitelist = whitelist_user(&mut svm, &payer, &vault_pda);

        let initial_amount = 100u64;
        let user_token_account = mint_tokens_to_user(&mut svm, &payer, &mint_keypair.pubkey(), initial_amount);

        // Deposit
        let deposit_amount = 50u64;
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
            }.to_account_metas(None),
            data: crate::instruction::Deposit { amount: deposit_amount }.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[deposit_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );
        svm.send_transaction(tx).expect("Deposit failed");

        // Withdraw
        let withdraw_amount = 50u64;
        let withdraw_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Withdraw {
                user: payer.pubkey(),
                user_token_account,
                vault: vault_pda,
                vault_token_account,
                mint: mint_keypair.pubkey(),
                hook_program: HOOK_PROGRAM_ID,
                extra_account_meta_list,
                vault_whitelist,
                token_program: spl_token_2022::ID,
            }.to_account_metas(None),
            data: crate::instruction::Withdraw { amount: withdraw_amount }.data(),
        };

        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[withdraw_ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );

        let result = svm.send_transaction(tx).expect("Withdraw should succeed");
        msg!("Withdraw successful - CU: {}", result.compute_units_consumed);

        // Verify final balances
        let user_account = svm.get_account(&user_token_account).expect("User token account not found");
        let user_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&user_account.data)
            .expect("Failed to unpack user token account");

        let vault_account = svm.get_account(&vault_token_account).expect("Vault token account not found");
        let vault_data = StateWithExtensions::<spl_token_2022::state::Account>::unpack(&vault_account.data)
            .expect("Failed to unpack vault token account");

        assert_eq!(user_data.base.amount, initial_amount);
        assert_eq!(vault_data.base.amount, 0);

        msg!("Test passed!");
    }
}
