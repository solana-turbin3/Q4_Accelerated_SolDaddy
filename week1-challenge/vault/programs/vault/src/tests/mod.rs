#[cfg(test)]
mod tests {
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
    use crate::Vault;

    static PROGRAM_ID: Pubkey = crate::ID;

    fn setup() -> (LiteSVM, Keypair) {

        fn derive_whitelist_pda(user: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[b"whitelist", user.as_ref()], &PROGRAM_ID)
        }
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();

        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        // Load program SO file
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/vault.so");
        msg!("{:?}", so_path);

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        program.add_program(PROGRAM_ID, &program_data);


        // Return the LiteSVM instance and payer keypair

        
        (program, payer)
    }
    #[test]
    fn test_init() {
        let (mut program, payer) = setup();

        // Derive PDA for vault (standard)
        let (vault_pda, vault_bump) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);
        msg!("Vault PDA: {}", vault_pda);

        // Create mint keypair (payer will be authority)
        let mint_keypair = Keypair::new();
        msg!("Vault Mint: {}", mint_keypair.pubkey());

        // Construct Initialize instruction
        let ix = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        // Send transaction with both payer and mint keypair
        let blockhash = program.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer, &mint_keypair],
            blockhash,
        );

        let result = program
            .send_transaction(tx)
            .expect("Initialize transaction failed");

        msg!("Initialize transaction succeeded!");
        msg!("Compute Units: {}", result.compute_units_consumed);

        // Verify vault account
        let vault_account = program
            .get_account(&vault_pda)
            .expect("Vault account not found");

        let vault_data =
            Vault::try_deserialize(&mut vault_account.data.as_ref()).expect("Deserialize failed");

        assert_eq!(vault_data.bump, vault_bump, "Vault bump mismatch");
        assert_eq!(vault_data.mint, mint_keypair.pubkey(), "Vault mint mismatch");
        msg!("Vault initialized successfully!");
    }

    #[test]
    fn test_deposit() {
        let (mut program, payer) = setup();

        // Derive Vault PDA
        let (vault_pda, vault_bump) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);
        msg!("Vault PDA: {}", vault_pda);

        // Create a new mint keypair
        let mint_keypair = Keypair::new();
        msg!("Vault Mint: {}", mint_keypair.pubkey());

        // Initialize Vault & Mint via program
        let ix_init = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        let blockhash = program.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix_init],
            Some(&payer.pubkey()),
            &[&payer, &mint_keypair], // mint keypair signs because it's being created
            blockhash,
        );
        program.send_transaction(tx).expect("Vault initialization failed");
        msg!("Vault & mint initialized");

        // Create token accounts
        let user_ata = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_keypair.pubkey())
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        let vault_ata = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_keypair.pubkey())
            .owner(&vault_pda)
            .send()
            .unwrap();
        msg!("User ATA: {}", user_ata);
        msg!("Vault ATA: {}", vault_ata);

        // Mint tokens to user
        MintTo::new(&mut program, &payer, &mint_keypair.pubkey(), &user_ata, 1_000_000_000)
            .owner(&payer) // payer is the mint authority
            .send()
            .unwrap();
        msg!("Minted 1_000_000_000 tokens to user ATA");

        // Deposit instruction
        let deposit_amount = 500_000_000;
        let ix_deposit = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: payer.pubkey(),
                user_token_account: user_ata,
                vault: vault_pda,
                vault_token_account: vault_ata,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Deposit { amount: deposit_amount }.data(),
        };

        let blockhash = program.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix_deposit],
            Some(&payer.pubkey()),
            &[&payer], // only payer signs
            blockhash,
        );
        program.send_transaction(tx).expect("Deposit failed");
        msg!("Deposit transaction succeeded");

        // Verify balances
        let vault_account_data = program.get_account(&vault_ata).unwrap();
        let vault_token_data = spl_token::state::Account::unpack(&vault_account_data.data).unwrap();

        let user_account_data = program.get_account(&user_ata).unwrap();
        let user_token_data = spl_token::state::Account::unpack(&user_account_data.data).unwrap();

        assert_eq!(vault_token_data.amount, deposit_amount, "Vault token balance mismatch");
        assert_eq!(user_token_data.amount, 1_000_000_000 - deposit_amount, "User token balance mismatch");

        assert_eq!(vault_token_data.owner, vault_pda);
        assert_eq!(vault_token_data.mint, mint_keypair.pubkey());
        assert_eq!(user_token_data.owner, payer.pubkey());
        assert_eq!(user_token_data.mint, mint_keypair.pubkey());

        msg!("Deposit verified successfully!");
    }


    #[test]
    fn test_withdraw() {
        let (mut program, payer) = setup();

        // Initialize vault and mint
        let (vault_pda, vault_bump) = Pubkey::find_program_address(&[b"vault"], &PROGRAM_ID);
        let mint_keypair = Keypair::new();

        let ix_init = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Initialize {
                user: payer.pubkey(),
                vault: vault_pda,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Initialize {}.data(),
        };

        let blockhash = program.latest_blockhash();
        let tx_init = Transaction::new_signed_with_payer(
            &[ix_init],
            Some(&payer.pubkey()),
            &[&payer, &mint_keypair],
            blockhash,
        );
        program.send_transaction(tx_init).expect("Vault initialization failed");
        msg!("Vault initialized");

        // Create token accounts
        let user_ata = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_keypair.pubkey())
            .owner(&payer.pubkey())
            .send()
            .unwrap();
        let vault_ata = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_keypair.pubkey())
            .owner(&vault_pda)
            .send()
            .unwrap();

        // 3️⃣ Mint tokens to user
        MintTo::new(&mut program, &payer, &mint_keypair.pubkey(), &user_ata, 1_000_000_000)
            .owner(&payer)
            .send()
            .unwrap();
        msg!("Minted tokens to user ATA");

        // Deposit half into vault
        let deposit_amount = 500_000_000;
        let ix_deposit = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Deposit {
                user: payer.pubkey(),
                user_token_account: user_ata,
                vault: vault_pda,
                vault_token_account: vault_ata,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
                associated_token_program: spl_associated_token_account::ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Deposit { amount: deposit_amount }.data(),
        };
        let blockhash = program.latest_blockhash();
        let tx_deposit = Transaction::new_signed_with_payer(&[ix_deposit], Some(&payer.pubkey()), &[&payer], blockhash);
        program.send_transaction(tx_deposit).expect("Deposit failed");
        msg!("Deposit succeeded");

        let withdraw_amount = 200_000_000;
        let ix_withdraw = solana_instruction::Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Withdraw {
                user: payer.pubkey(),
                user_token_account: user_ata,
                vault: vault_pda,
                vault_token_account: vault_ata,
                mint: mint_keypair.pubkey(),
                token_program: spl_token::ID,
            }
                .to_account_metas(None),
            data: crate::instruction::Withdraw { amount: withdraw_amount }.data(),
        };
        let blockhash = program.latest_blockhash();
        let tx_withdraw = Transaction::new_signed_with_payer(&[ix_withdraw], Some(&payer.pubkey()), &[&payer], blockhash);
        program.send_transaction(tx_withdraw).expect("Withdraw failed");
        msg!("Withdraw succeeded");

        
        let vault_account_data = program.get_account(&vault_ata).unwrap();
        let vault_token_data = spl_token::state::Account::unpack(&vault_account_data.data).unwrap();
        let user_account_data = program.get_account(&user_ata).unwrap();
        let user_token_data = spl_token::state::Account::unpack(&user_account_data.data).unwrap();

        assert_eq!(vault_token_data.amount, 300_000_000);
        assert_eq!(user_token_data.amount, 700_000_000);
        msg!("Withdraw balances verified successfully!");
    }


}