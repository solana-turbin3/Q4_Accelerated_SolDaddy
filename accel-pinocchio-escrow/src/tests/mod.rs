#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use litesvm::LiteSVM;
    use litesvm_token::{spl_token::{self, solana_program::{msg, rent::Rent, sysvar::SysvarId}}, CreateAssociatedTokenAccount, CreateMint, MintTo};

    use solana_instruction::{AccountMeta, Instruction};
    use solana_keypair::Keypair;
    use solana_message::Message;
    use solana_native_token::LAMPORTS_PER_SOL;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;
    use spl_token::ID;

    const PROGRAM_ID: &str = "4ibrEMW5F6hKnkW4jVedswYv6H6VtwPN6ar6dvXDN1nT";
    const TOKEN_PROGRAM_ID: Pubkey = ID;
    const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    fn program_id() -> Pubkey {
        Pubkey::from(crate::ID)
    }

    fn setup() -> (LiteSVM, Keypair) {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();

        svm
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop failed");

        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/sbpf-solana-solana/release/escrow.so");

        msg!("Resolved path: {:?}", so_path);
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        svm.add_program(program_id(), &program_data);

        (svm, payer)
    }

    #[test]
    pub fn test_make_instruction() {
        let (mut svm, payer) = setup();

        let program_id = program_id();
        assert_eq!(program_id.to_string(), PROGRAM_ID);

        // FIXED: Use .token_program_id() not .token_program()
        let mint_a = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut svm, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint B: {}", mint_b);


        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &payer, &mint_a)
            .owner(&payer.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), payer.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        msg!("Escrow PDA: {}\n", escrow.0);

        let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
            &escrow.0,
            &mint_a,
            &TOKEN_PROGRAM_ID
        );
        msg!("Vault PDA: {}\n", vault);

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        MintTo::new(&mut svm, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        let amount_to_receive: u64 = 100000000;
        let amount_to_give: u64 = 500000000;
        let bump: u8 = escrow.1;

        msg!("Bump: {}", bump);


        let make_data = [
            vec![0u8],              // Discriminator
            vec![bump],
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ].concat();

        let make_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
                AccountMeta::new(Rent::id(), false),
            ],
            data: make_data,
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        let tx = svm.send_transaction(transaction).unwrap();

        msg!("\n\nMake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_take_instruction() {
        let (mut svm, maker) = setup();
        let taker = Keypair::new();


        svm.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Airdrop to taker failed");

        let program_id = program_id();


        let mint_a = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint B: {}", mint_b);


        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .owner(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}", maker_ata_a);


        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut svm, &taker, &mint_b)
            .owner(&taker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Taker ATA B: {}", taker_ata_b);


        MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();
        msg!("Minted 1000 tokens of mint A to maker");

        MintTo::new(&mut svm, &maker, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();
        msg!("Minted 1000 tokens of mint B to taker");


        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), maker.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        msg!("Escrow PDA: {}", escrow.0);


        let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
            &escrow.0,
            &mint_a,
            &TOKEN_PROGRAM_ID
        );
        msg!("Vault PDA: {}", vault);

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        let amount_to_receive: u64 = 100000000;
        let amount_to_give: u64 = 500000000;
        let bump: u8 = escrow.1;

        // Execute MAKE instruction
        let make_data = [
            vec![0u8],
            vec![bump],
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ].concat();

        let make_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
                AccountMeta::new(Rent::id(), false),
            ],
            data: make_data,
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        svm.send_transaction(transaction).unwrap();
        msg!("Make instruction executed successfully");

        // Derive taker's ATA for mint A
        let taker_ata_a = spl_associated_token_account::get_associated_token_address_with_program_id(
            &taker.pubkey(),
            &mint_a,
            &TOKEN_PROGRAM_ID
        );

        // Derive maker's ATA for mint B
        let maker_ata_b = spl_associated_token_account::get_associated_token_address_with_program_id(
            &maker.pubkey(),
            &mint_b,
            &TOKEN_PROGRAM_ID
        );

        // Execute TAKE instruction
        let take_data = vec![1u8];

        let take_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(taker.pubkey(), true),
                AccountMeta::new(maker.pubkey(), false),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(taker_ata_a, false),
                AccountMeta::new(taker_ata_b, false),
                AccountMeta::new(maker_ata_b, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
            ],
            data: take_data,
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);

        let tx = svm.send_transaction(transaction).unwrap();

        msg!("\n\nTake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
    }

    #[test]
    pub fn test_cancel_instruction() {
        let (mut svm, maker) = setup();

        let program_id = program_id();
        assert_eq!(program_id.to_string(), PROGRAM_ID);

        // Create mints
        let mint_a = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut svm, &maker)
            .decimals(6)
            .authority(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Mint B: {}", mint_b);

        // Create maker's ATA for mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut svm, &maker, &mint_a)
            .owner(&maker.pubkey())
            .token_program_id(&TOKEN_PROGRAM_ID)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}", maker_ata_a);

        // Mint tokens to maker
        MintTo::new(&mut svm, &maker, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();
        msg!("Minted 1000 tokens of mint A to maker");

        // Derive escrow PDA
        let escrow = Pubkey::find_program_address(
            &[b"escrow".as_ref(), maker.pubkey().as_ref()],
            &PROGRAM_ID.parse().unwrap(),
        );
        msg!("Escrow PDA: {}", escrow.0);

        // Derive vault
        let vault = spl_associated_token_account::get_associated_token_address_with_program_id(
            &escrow.0,
            &mint_a,
            &TOKEN_PROGRAM_ID
        );
        msg!("Vault PDA: {}", vault);

        let associated_token_program = ASSOCIATED_TOKEN_PROGRAM_ID.parse::<Pubkey>().unwrap();
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = solana_sdk_ids::system_program::ID;

        let amount_to_receive: u64 = 100000000;
        let amount_to_give: u64 = 500000000;
        let bump: u8 = escrow.1;

        msg!("Bump: {}", bump);

        // Execute MAKE instruction
        let make_data = [
            vec![0u8],
            vec![bump],
            amount_to_receive.to_le_bytes().to_vec(),
            amount_to_give.to_le_bytes().to_vec(),
        ].concat();

        let make_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(mint_b, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
                AccountMeta::new(associated_token_program, false),
                AccountMeta::new(Rent::id(), false),
            ],
            data: make_data,
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        svm.send_transaction(transaction).unwrap();
        msg!("Make instruction executed successfully");

        // Get maker's token balance before cancel
        let maker_balance_before = svm
            .get_account(&maker_ata_a)
            .map(|acc| {
                let data = acc.data;
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            })
            .unwrap_or(0);
        msg!("Maker balance before cancel: {}", maker_balance_before);

        // Execute CANCEL instruction
        let cancel_data = [
            vec![2u8],    // Discriminator
            vec![bump],
        ].concat();

        let cancel_ix = Instruction {
            program_id: program_id,
            accounts: vec![
                AccountMeta::new(maker.pubkey(), true),
                AccountMeta::new(mint_a, false),
                AccountMeta::new(escrow.0, false),
                AccountMeta::new(maker_ata_a, false),
                AccountMeta::new(vault, false),
                AccountMeta::new(system_program, false),
                AccountMeta::new(token_program, false),
            ],
            data: cancel_data,
        };

        let message = Message::new(&[cancel_ix], Some(&maker.pubkey()));
        let recent_blockhash = svm.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);

        let tx = svm.send_transaction(transaction).unwrap();

        msg!("\n\nCancel transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);

        // Verify maker received tokens back
        let maker_balance_after = svm
            .get_account(&maker_ata_a)
            .map(|acc| {
                let data = acc.data;
                u64::from_le_bytes(data[64..72].try_into().unwrap())
            })
            .unwrap_or(0);
        msg!("Maker balance after cancel: {}", maker_balance_after);

        // Verify the escrow ATA is closed (should not exist)
        // let vault_exists = svm.get_account(&vault).is_some();
        // assert!(!vault_exists, "Vault should be closed");
        // msg!("Vault successfully closed");

        // Verify maker got tokens back
        assert_eq!(
            maker_balance_after,
            maker_balance_before + amount_to_give,
            "Maker should have received tokens back"
        );
        msg!("Maker successfully received {} tokens back", amount_to_give);
    }


}
