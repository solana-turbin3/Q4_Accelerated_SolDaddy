#[cfg(test)]
mod tests {
    use anchor_lang::{solana_program, AnchorSerialize};
    use {
        anchor_lang::{
            prelude::msg,
            solana_program::program_pack::Pack,
            AccountDeserialize,
            InstructionData,
            ToAccountMetas
        }, anchor_spl::{
            associated_token::{
                self,
                spl_associated_token_account
            },
            token::spl_token
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID,
            CreateAssociatedTokenAccount,
            CreateMint, MintTo
        },
        solana_rpc_client::rpc_client::RpcClient,
        solana_account::Account,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        solana_address::Address,
        std::{
            path::PathBuf,
            str::FromStr
        }
    };
    use crate::state::Escrow;

    static PROGRAM_ID: Pubkey = crate::ID;

    fn setup() -> (LiteSVM, Keypair) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();

        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        // Load program SO file
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/anchor_escrow.so");

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        // let account_address = Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        // let fetched_account = rpc_client
        //     .get_account(&account_address)
        //     .expect("Failed to fetch account from devnet");
        //
        // program.set_account(payer.pubkey(), Account {
        //     lamports: fetched_account.lamports,
        //     data: fetched_account.data,
        //     owner: Pubkey::from(fetched_account.owner.to_bytes()),
        //     executable: fetched_account.executable,
        //     rent_epoch: fetched_account.rent_epoch
        // }).unwrap();

        // msg!("Lamports of fetched account: {}", fetched_account.lamports);

        // Return the LiteSVM instance and payer keypair
        (program, payer)
    }

    #[test]
    fn test_make() {

        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();

        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();


        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 10, seed: 123u64, receive: 10 }.data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);

    }
    // #[test]
    // fn test_take(){
    //     let (mut program, payer) = setup();
    //
    //     let maker = payer.pubkey();
    //
    //     let taker = solana_keypair::Keypair::new();
    //     program.airdrop(&taker.pubkey(), 10 * solana_native_token::LAMPORTS_PER_SOL).unwrap();
    //
    //     //Mints
    //     let mint_a = CreateMint::new(&mut program, &payer)
    //         .decimals(6)
    //         .authority(&maker)
    //         .send()
    //         .unwrap();
    //     msg!("Mint A: {}", mint_a);
    //
    //     let mint_b = CreateMint::new(&mut program, &payer)
    //         .decimals(6)
    //         .authority(&maker)
    //         .send()
    //         .unwrap();
    //     msg!("Mint B: {}", mint_b);
    //
    //     // Maker ATA for Mint A (needed to fund the vault)
    //     let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
    //         .owner(&maker)
    //         .send()
    //         .unwrap();
    //
    //     // Mint tokens to maker's ATA A
    //     MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000)
    //         .send()
    //         .unwrap();
    //
    //     // Taker ATA for Mint A (to receive from vault)
    //     let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
    //         .owner(&taker.pubkey())
    //         .send()
    //         .unwrap();
    //
    //     // Taker ATA for Mint B (to pay)
    //     let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
    //         .owner(&taker.pubkey())
    //         .send()
    //         .unwrap();
    //
    //     // Mint some Mint B tokens to taker so they can pay Maker
    //     MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1_000)
    //         .send()
    //         .unwrap();
    //
    //     // Maker ATA for Mint B (to receive taker's payment)
    //     let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
    //         .owner(&maker)
    //         .send()
    //         .unwrap();
    //
    //     // PDA Derivations
    //     let seed: u64 = 123;
    //     let (escrow, escrow_bump) = Pubkey::find_program_address(
    //         &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
    //         &PROGRAM_ID,
    //     );
    //
    //     let vault = anchor_spl::associated_token::get_associated_token_address(&escrow, &mint_a);
    //     msg!("Vault ATA: {}", vault);
    //
    //     // Create the vault ATA (owned by escrow PDA)
    //     let create_vault_ix = spl_associated_token_account::instruction::create_associated_token_account(
    //         &payer.pubkey(),
    //         &escrow,
    //         &mint_a,
    //         &TOKEN_PROGRAM_ID,
    //     );
    //
    //     let message = Message::new(&[create_vault_ix], Some(&payer.pubkey()));
    //     let recent_blockhash = program.latest_blockhash();
    //     let tx = Transaction::new(&[&payer], message, recent_blockhash);
    //     program.send_transaction(tx).unwrap();
    //
    //     // Transfer tokens from maker to vault (simulating what the Make instruction does)
    //     let transfer_to_vault_ix = spl_token::instruction::transfer(
    //         &TOKEN_PROGRAM_ID,
    //         &maker_ata_a,
    //         &vault,
    //         &maker,
    //         &[],
    //         10, // Amount that maker deposited
    //     ).unwrap();
    //
    //     let message = Message::new(&[transfer_to_vault_ix], Some(&payer.pubkey()));
    //     let recent_blockhash = program.latest_blockhash();
    //     let tx = Transaction::new(&[&payer], message, recent_blockhash);
    //     program.send_transaction(tx).unwrap();
    //
    //     // Create the Escrow state
    //     let escrow_state = Escrow {
    //         seed,
    //         maker,
    //         mint_a,
    //         mint_b,
    //         receive: 10,
    //         bump: escrow_bump,
    //     };
    //
    //     use anchor_lang::Discriminator;
    //     let discriminator = Escrow::DISCRIMINATOR;
    //
    //     let mut escrow_data = discriminator.to_vec();
    //     escrow_data.extend_from_slice(&escrow_state.try_to_vec().unwrap());
    //
    //     // Wrap it!
    //     let escrow_account = Account {
    //         lamports: 1_000_000_000, // arbitrary lamports for test
    //         data: escrow_data,
    //         owner: PROGRAM_ID,
    //         executable: false,
    //         rent_epoch: 0,
    //     };
    //
    //     // Set the account in LiteSVM
    //     program.set_account(escrow, escrow_account).unwrap();
    //
    //     let associated_token_program = spl_associated_token_account::ID;
    //     let token_program = TOKEN_PROGRAM_ID;
    //     let system_program = SYSTEM_PROGRAM_ID;
    //
    //     let take_ix = Instruction {
    //         program_id: PROGRAM_ID,
    //         accounts: crate::accounts::Take {
    //             taker: taker.pubkey(),
    //             maker,
    //             mint_a,
    //             mint_b,
    //             taker_ata_a,
    //             taker_ata_b,
    //             maker_ata_b,
    //             escrow,
    //             vault,
    //             associated_token_program,
    //             token_program,
    //             system_program,
    //         }.to_account_metas(None),
    //         data: crate::instruction::Take {}.data(),
    //     };
    //
    //     let message = Message::new(&[take_ix], Some(&taker.pubkey()));
    //     let recent_blockhash = program.latest_blockhash();
    //     let tx = Transaction::new(&[&taker], message, recent_blockhash);
    //     let result = program.send_transaction(tx).unwrap();
    //
    //     msg!("\n\nTake transaction successful");
    //     msg!("CUs Consumed: {}", result.compute_units_consumed);
    //     msg!("Tx Signature: {}", result.signature);
    //
    //     // Verify balances after take
    //     let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
    //     let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
    //     assert_eq!(taker_ata_a_data.amount, 10, "Taker should receive 10 tokens of Mint A");
    //
    //     let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
    //     let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
    //     assert_eq!(maker_ata_b_data.amount, 10, "Maker should receive 10 tokens of Mint B");
    // }
    #[test]
    fn test_take_before_unlock_time_fails() {
        let (mut program, payer) = setup();

        let maker = payer.pubkey();
        let seed: u64 = 456;

        // Create mints
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}", mint_b);

        // Maker ATA for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();

        // Mint tokens to maker
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1_000)
            .send()
            .unwrap();

        // Derive PDAs
        let (escrow, _) = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &seed.to_le_bytes()],
            &PROGRAM_ID,
        );

        let vault = anchor_spl::associated_token::get_associated_token_address(&escrow, &mint_a);

        // let initial_clock = 10000000;
        // initial timestamp 1,000,000
        let initial_clock = solana_program::clock::Clock {
            slot: 100,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_000_000,
        };
        program.set_sysvar::<solana_program::clock::Clock>(&initial_clock);

        // Create escrow with Make instruction
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 10,
                seed,
                receive: 10
            }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let tx = Transaction::new(&[&payer], message, recent_blockhash);
        program.send_transaction(tx).unwrap();
        msg!("Escrow created at timestamp: {} (unlock at {})",
         initial_clock.unix_timestamp,
         initial_clock.unix_timestamp + 1800);

        // Setup taker
        let taker = solana_keypair::Keypair::new();
        program.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        // Taker ATA for Mint A (to receive from vault)
        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Taker ATA for Mint B (to pay)
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Mint tokens to taker so they can pay
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1_000)
            .send()
            .unwrap();

        // Maker ATA for Mint B (to receive payment)
        let maker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&maker)
            .send()
            .unwrap();

        // Try to take BEFORE unlock time 15 minutes (900 seconds)...should FAIL
        let too_early_clock = solana_program::clock::Clock {
            slot: 300,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_000_000 + 900,
        };
        program.set_sysvar::<solana_program::clock::Clock>(&too_early_clock);
        msg!("Time traveled to timestamp: {} (15 minutes after creation BEFORE unlock)",
         too_early_clock.unix_timestamp);

        let take_ix_early = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix_early], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let tx = Transaction::new(&[&taker], message, recent_blockhash);
        let result_early = program.send_transaction(tx);

        msg!("Take attempt at 15 minutes result: {:?}", result_early);
        assert!(
            result_early.is_err(),
            "Expected error: Cannot take escrow before 30 minutes unlock time"
        );

        // Expire the current blockhash
        program.expire_blockhash();

        // Test at exactly 30 minutes
        let exact_unlock_clock = solana_program::clock::Clock {
            slot: 800,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_000_000 + 1_800,
        };
        program.set_sysvar::<solana_program::clock::Clock>(&exact_unlock_clock);
        msg!("Time at exactly 30 minutes: {}", exact_unlock_clock.unix_timestamp);

        let take_ix_exact = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix_exact], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let tx = Transaction::new(&[&taker], message, recent_blockhash);
        let result_exact = program.send_transaction(tx);

        if result_exact.is_ok() {
            msg!("Take succeeded at exactly 30 minutes (>= condition works!)");
            let result = result_exact.unwrap();
            msg!("CUs Consumed: {}", result.compute_units_consumed);

            // Verify balances
            let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
            let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
            assert_eq!(taker_ata_a_data.amount, 10, "Taker should receive 10 tokens of Mint A");

            let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
            let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
            assert_eq!(maker_ata_b_data.amount, 10, "Maker should receive 10 tokens of Mint B");

            msg!("YAYYAYAYAYAY!!! All assertions passed! Escrow completed successfully at exactly 30 minute unlock time.");
            return;
        }

        msg!("Take failed at exactly 30 minutes (this shouldn't happen with >= condition)");

        // Expire blockhash again for the next test
        program.expire_blockhash();

        // Time travel to 35 minutes - should definitely SUCCEED
        let after_unlock_clock = solana_program::clock::Clock {
            slot: 600,
            epoch_start_timestamp: 0,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_000_000 + 2_100,
        };
        program.set_sysvar::<solana_program::clock::Clock>(&after_unlock_clock);
        msg!("⏰ Time traveled to timestamp: {} (35 minutes after creation - AFTER unlock)",
         after_unlock_clock.unix_timestamp);

        let take_ix_valid = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: spl_associated_token_account::ID,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix_valid], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let tx = Transaction::new(&[&taker], message, recent_blockhash);
        let result_valid = program.send_transaction(tx).unwrap();

        msg!("Take transaction successful after unlock time!");
        msg!("CUs Consumed: {}", result_valid.compute_units_consumed);
        msg!("Tx Signature: {}", result_valid.signature);

        // Verify balances
        let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
        let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
        assert_eq!(taker_ata_a_data.amount, 10, "Taker should receive 10 tokens of Mint A");

        let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
        let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
        assert_eq!(maker_ata_b_data.amount, 10, "Maker should receive 10 tokens of Mint B");

        msg!("All assertions passed! Escrow completed successfully after 30-minute unlock period.");
    }

}