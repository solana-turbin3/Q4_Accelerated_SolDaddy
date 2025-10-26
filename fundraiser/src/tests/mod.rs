#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{spl_token::{self, solana_program::{msg}}, CreateAssociatedTokenAccount, CreateMint, MintTo};
    use litesvm_token::spl_token::solana_program::clock::Clock;
    use litesvm_token::spl_token::solana_program::program_pack::Pack;
    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;
    use crate::constants::MIN_AMOUNT_TO_RAISE;
    use crate::state::Fundraiser;



    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let ata_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("ata_program.so");
        let ata_data = std::fs::read(&ata_path).expect("Failed to read ATA program");
        msg!("Resolved path: {:?}", ata_path);


        svm.add_program(spl_associated_token_account::ID, &ata_data);
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/sbpf-solana-solana/release/fundraiser.so");

        msg!("Resolved path: {:?}", so_path);
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(program_id(), &program_data);

        let mut clock = Clock::default();
        clock.unix_timestamp = 1700000000;
        svm.set_sysvar(&clock);


        (svm, payer)
    }
    #[test]
    fn test_initialize() {
        let (mut svm, payer) = setup();
        let program_id = program_id();

        msg!("Program ID: {}", program_id);

        // Create the mint for fundraising
        let mint_to_raise = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Mint created: {}", mint_to_raise);

        // Derive the fundraiser PDA
        let (fundraiser_pda, bump) = Pubkey::find_program_address(
            &[b"fundraiser", payer.pubkey().as_ref()],
            &program_id,
        );
        msg!("Fundraiser PDA: {}, bump: {}", fundraiser_pda, bump);

        // Derive the vault ATA
        let vault_pda = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&fundraiser_pda)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Vault PDA: {}", vault_pda);

        let expected_vault = spl_associated_token_account::get_associated_token_address_with_program_id(
            &fundraiser_pda,
            &mint_to_raise,
            &spl_token::ID
        );
        msg!("Expected vault: {}", expected_vault);
        assert_eq!(vault_pda, expected_vault, "Vault addresses should match");

        // Prepare instruction data
        let amount_to_raise = MIN_AMOUNT_TO_RAISE + 1_000_000;
        let duration: u8 = 30;

        let init_data = [
            vec![0u8],
            amount_to_raise.to_le_bytes().to_vec(),
            vec![duration],
        ].concat();

        msg!("Instruction data:");
        msg!("  Length: {} bytes", init_data.len());
        msg!("  Amount: {}", amount_to_raise);
        msg!("  Duration: {}", duration);

        // Build the Initialize instruction
        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false)
            ],
            data: init_data,
        };

        // Send transaction
        let message = Message::new(&[init_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, svm.latest_blockhash());

        msg!("Sending Initialize transaction...");
        let result = svm.send_transaction(transaction)
            .expect("Initialize transaction failed");

        msg!("Initialize successful!");
        msg!("CUs consumed: {}", result.compute_units_consumed);

        // Verify fundraiser account
        msg!("Verifying fundraiser account...");
        let fundraiser_account = svm.get_account(&fundraiser_pda).unwrap();

        assert_eq!(fundraiser_account.owner, program_id, "Fundraiser should be owned by program");
        assert_eq!(fundraiser_account.data.len(), Fundraiser::LEN, "Incorrect account size");
        msg!("Owner: {}", fundraiser_account.owner);
        msg!("Size: {} bytes", fundraiser_account.data.len());

        // Parse fundraiser state
        let data = &fundraiser_account.data;
        let maker_bytes = &data[0..32];
        let mint_bytes = &data[32..64];
        let amount_bytes = &data[64..72];
        let current_amount_bytes = &data[72..80];
        let time_started_bytes = &data[80..88];
        let duration_byte = data[88];
        let bump_byte = data[89];

        // Verify state
        assert_eq!(maker_bytes, payer.pubkey().as_ref(), "Maker mismatch");
        assert_eq!(mint_bytes, mint_to_raise.as_ref(), "Mint mismatch");

        let stored_amount = u64::from_le_bytes(amount_bytes.try_into().unwrap());
        assert_eq!(stored_amount, amount_to_raise, "Amount mismatch");

        let current_amount = u64::from_le_bytes(current_amount_bytes.try_into().unwrap());
        assert_eq!(current_amount, 0, "Current amount should be 0");

        let time_started = i64::from_le_bytes(time_started_bytes.try_into().unwrap());
        assert!(time_started > 0, "Time started not set");

        assert_eq!(duration_byte, duration, "Duration mismatch");
        assert_eq!(bump_byte, bump, "Bump mismatch");

        msg!("Maker: {}", payer.pubkey());
        msg!("Mint: {}", mint_to_raise);
        msg!("Amount to raise: {}", stored_amount);
        msg!("Current amount: {}", current_amount);
        msg!("Time started: {}", time_started);
        msg!("Duration: {}", duration_byte);
        msg!("Bump: {}", bump_byte);

        // Verify vault ATA
        msg!("Verifying vault ATA...");
        let vault_account = svm.get_account(&vault_pda).unwrap();

        assert_eq!(vault_account.owner, spl_token::ID, "Vault should be owned by token program");
        msg!("Owner: {}", vault_account.owner);

        let token_account = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(token_account.mint, mint_to_raise, "Vault mint mismatch");
        assert_eq!(token_account.owner, fundraiser_pda, "Vault owner mismatch");
        assert_eq!(token_account.amount, 0, "Vault should start empty");

        msg!("Mint: {}", token_account.mint);
        msg!("Owner: {}", token_account.owner);
        msg!("Balance: {} tokens", token_account.amount);

        msg!("All assertions passed!");
        msg!("Fundraiser initialized successfully!");
        msg!("Program: {}", program_id);
        msg!("Fundraiser: {}", fundraiser_pda);
        msg!("Vault: {}", vault_pda);
        msg!("Target: {} tokens", amount_to_raise);
        msg!("CUs: {}", result.compute_units_consumed);
        msg!("\n\n\n")
    }

    #[test]
    fn test_contribute() {
        let (mut svm, payer) = setup();
        let program_id = program_id();

        msg!("Starting contribute test...");

        // Initialize the fundraiser
        let mint_to_raise = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Mint created: {}", mint_to_raise);

        let (fundraiser_pda, _bump) = Pubkey::find_program_address(
            &[b"fundraiser", payer.pubkey().as_ref()],
            &program_id,
        );

        // Create vault ATA
        let vault_pda = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&fundraiser_pda)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Vault created: {}", vault_pda);

        // Initialize fundraiser
        let amount_to_raise = MIN_AMOUNT_TO_RAISE + 1_000_000;
        let duration: u8 = 30;
        let init_data = [
            vec![0u8],
            amount_to_raise.to_le_bytes().to_vec(),
            vec![duration],
        ].concat();

        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
            data: init_data,
        };

        let tx = Transaction::new(&[&payer], Message::new(&[init_ix], Some(&payer.pubkey())), svm.latest_blockhash());
        svm.send_transaction(tx).expect("Initialize failed");
        msg!("Fundraiser initialized");

        // Setup contributor
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        msg!("Contributor created: {}", contributor.pubkey());

        // Create contributor's token account
        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&contributor.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Contributor ATA created: {}", contributor_ata);

        // Mint tokens to contributor
        let contribution_amount = 50_000u64;
        MintTo::new(&mut svm, &payer, &mint_to_raise, &contributor_ata, contribution_amount * 2)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Minted {} tokens to contributor", contribution_amount * 2);

        // Derive contributor PDA
        let (contributor_pda, _) = Pubkey::find_program_address(
            &[b"contributor", fundraiser_pda.as_ref(), contributor.pubkey().as_ref()],
            &program_id,
        );
        msg!("Contributor PDA: {}", contributor_pda);

        // Build contribute instruction
        let contribute_data = [
            vec![1u8],
            contribution_amount.to_le_bytes().to_vec(),
        ].concat();

        let contribute_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(contributor_pda, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
            ],
            data: contribute_data,
        };

        msg!("Sending Contribute transaction...");
        let tx = Transaction::new(
            &[&contributor],
            Message::new(&[contribute_ix], Some(&contributor.pubkey())),
            svm.latest_blockhash()
        );

        let result = svm.send_transaction(tx).expect("Contribute failed");
        msg!("Contribute successful! CUs: {}", result.compute_units_consumed);

        // Verify results
        msg!("Verifying fundraiser state...");
        let fundraiser_account = svm.get_account(&fundraiser_pda).unwrap();
        let current_amount = u64::from_le_bytes(fundraiser_account.data[72..80].try_into().unwrap());
        assert_eq!(current_amount, contribution_amount);
        msg!("Current amount: {}", current_amount);

        msg!("Contribute test passed!");
        msg!("\n\n\n");
    }

    #[test]
    fn test_refund() {
        let (mut svm, payer) = setup();
        let program_id = program_id();

        msg!("Starting refund test...");

        // Initialize fundraiser
        let mint_to_raise = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Mint created: {}", mint_to_raise);

        let (fundraiser_pda, _bump) = Pubkey::find_program_address(
            &[b"fundraiser", payer.pubkey().as_ref()],
            &program_id,
        );

        let vault_pda = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&fundraiser_pda)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Vault created: {}", vault_pda);

        let amount_to_raise = MIN_AMOUNT_TO_RAISE + 1_000_000;
        let duration: u8 = 1;
        let init_data = [
            vec![0u8],
            amount_to_raise.to_le_bytes().to_vec(),
            vec![duration],
        ].concat();

        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
            data: init_data,
        };

        let tx = Transaction::new(&[&payer], Message::new(&[init_ix], Some(&payer.pubkey())), svm.latest_blockhash());
        svm.send_transaction(tx).unwrap();
        msg!("Fundraiser initialized");

        // Setup contributor and contribute
        let contributor = Keypair::new();
        svm.airdrop(&contributor.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let contributor_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&contributor.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();

        let contribution_amount = 50_000u64;
        MintTo::new(&mut svm, &payer, &mint_to_raise, &contributor_ata, contribution_amount * 2)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Minted {} tokens to contributor", contribution_amount * 2);

        let (contributor_pda, _) = Pubkey::find_program_address(
            &[b"contributor", fundraiser_pda.as_ref(), contributor.pubkey().as_ref()],
            &program_id,
        );

        // Contribute
        let contribute_data = [
            vec![1u8],
            contribution_amount.to_le_bytes().to_vec(),
        ].concat();

        let contribute_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(contributor_pda, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
            ],
            data: contribute_data,
        };

        let tx = Transaction::new(
            &[&contributor],
            Message::new(&[contribute_ix], Some(&contributor.pubkey())),
            svm.latest_blockhash()
        );
        svm.send_transaction(tx).unwrap();
        msg!("Contribution successful: {} tokens", contribution_amount);

        // Advance time past fundraiser duration
        let mut clock = Clock::default();
        clock.unix_timestamp = 1700000000 + (86400 * 2);
        svm.set_sysvar(&clock);
        msg!("Advanced time past fundraiser end");

        // Request refund
        let refund_data = vec![2u8];

        let refund_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor.pubkey(), true),
                AccountMeta::new_readonly(payer.pubkey(), false),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(contributor_pda, false),
                AccountMeta::new(contributor_ata, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(spl_token::ID, false),
            ],
            data: refund_data,
        };

        msg!("Sending Refund transaction...");
        let tx = Transaction::new(
            &[&contributor],
            Message::new(&[refund_ix], Some(&contributor.pubkey())),
            svm.latest_blockhash()
        );

        let result = svm.send_transaction(tx).expect("Refund failed");
        msg!("Refund successful! CUs: {}", result.compute_units_consumed);

        // Verify refund
        msg!("Verifying refund...");

        let fundraiser_account = svm.get_account(&fundraiser_pda).unwrap();
        let current_amount = u64::from_le_bytes(fundraiser_account.data[72..80].try_into().unwrap());
        assert_eq!(current_amount, 0, "Fundraiser current amount should be 0 after refund");
        msg!("Fundraiser current amount: {}", current_amount);

        let contributor_account = svm.get_account(&contributor_pda).unwrap();
        let contributor_amount = u64::from_le_bytes(contributor_account.data[0..8].try_into().unwrap());
        assert_eq!(contributor_amount, 0, "Contributor amount should be 0 after refund");
        msg!("Contributor amount: {}", contributor_amount);

        let vault_account = svm.get_account(&vault_pda).unwrap();
        let vault_token = spl_token_2022::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_token.amount, 0, "Vault should be empty after refund");
        msg!("Vault balance: {}", vault_token.amount);

        let contributor_ata_account = svm.get_account(&contributor_ata).unwrap();
        let contributor_token = spl_token_2022::state::Account::unpack(&contributor_ata_account.data).unwrap();
        assert_eq!(
            contributor_token.amount,
            contribution_amount * 2,
            "Contributor should have all tokens back"
        );
        msg!("Contributor balance: {}", contributor_token.amount);

        msg!("Refund test passed!");
        msg!("\n\n\n")
    }

    #[test]
    fn test_finalize() {
        let (mut svm, payer) = setup();
        let program_id = program_id();

        msg!("Starting finalize test...");

        // Initialize fundraiser
        let mint_to_raise = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Mint created: {}", mint_to_raise);

        let (fundraiser_pda, _bump) = Pubkey::find_program_address(
            &[b"fundraiser", payer.pubkey().as_ref()],
            &program_id,
        );

        let vault_pda = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&fundraiser_pda)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Vault created: {}", vault_pda);

        let amount_to_raise = MIN_AMOUNT_TO_RAISE + 1_000_000;
        let duration: u8 = 30;
        let init_data = [
            vec![0u8],
            amount_to_raise.to_le_bytes().to_vec(),
            vec![duration],
        ].concat();

        let init_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
            data: init_data,
        };

        let tx = Transaction::new(&[&payer], Message::new(&[init_ix], Some(&payer.pubkey())), svm.latest_blockhash());
        svm.send_transaction(tx).unwrap();
        msg!("Fundraiser initialized with target: {}", amount_to_raise);

        // Setup contributor
        let contributor1 = Keypair::new();
        svm.airdrop(&contributor1.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        let contributor1_ata = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_to_raise)
            .owner(&contributor1.pubkey())
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();

        let contribution_amount = amount_to_raise;

        MintTo::new(&mut svm, &payer, &mint_to_raise, &contributor1_ata, contribution_amount)
            .token_program_id(&spl_token::ID)
            .send()
            .unwrap();
        msg!("Minted {} tokens to contributor", contribution_amount);

        let (contributor1_pda, _) = Pubkey::find_program_address(
            &[b"contributor", fundraiser_pda.as_ref(), contributor1.pubkey().as_ref()],
            &program_id,
        );

        // Contribute
        let contribute_data = [
            vec![1u8],
            contribution_amount.to_le_bytes().to_vec(),
        ].concat();

        let contribute_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(contributor1.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(contributor1_pda, false),
                AccountMeta::new(contributor1_ata, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
            ],
            data: contribute_data,
        };

        let tx = Transaction::new(
            &[&contributor1],
            Message::new(&[contribute_ix], Some(&contributor1.pubkey())),
            svm.latest_blockhash()
        );
        svm.send_transaction(tx).unwrap();
        msg!("Contribution successful - goal reached!");

        // Create maker ATA
        let maker_ata = spl_associated_token_account::get_associated_token_address(
            &payer.pubkey(),
            &mint_to_raise
        );
        msg!("Maker ATA: {}", maker_ata);

        // Finalize
        let finalize_data = vec![3u8];

        let finalize_ix = Instruction {
            program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new_readonly(mint_to_raise, false),
                AccountMeta::new(fundraiser_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(maker_ata, false),
                AccountMeta::new_readonly(spl_token::ID, false),
                AccountMeta::new_readonly(Pubkey::new_from_array(pinocchio_system::ID), false),
                AccountMeta::new_readonly(solana_sdk_ids::sysvar::rent::ID, false),
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            ],
            data: finalize_data,
        };

        msg!("Sending Finalize transaction...");
        let tx = Transaction::new(
            &[&payer],
            Message::new(&[finalize_ix], Some(&payer.pubkey())),
            svm.latest_blockhash()
        );

        let result = svm.send_transaction(tx).expect("Finalize failed");
        msg!("Finalize successful! CUs: {}", result.compute_units_consumed);

        msg!("Finalize test passed!");
        msg!("\n\n\n");
    }

}


