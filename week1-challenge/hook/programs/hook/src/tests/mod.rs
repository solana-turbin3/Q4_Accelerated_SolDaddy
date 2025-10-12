#[cfg(test)]
mod tests {
    use anchor_lang::{AccountDeserialize, InstructionData, ToAccountMetas};
    use anchor_lang::prelude::msg;
    use {
        litesvm::LiteSVM,
        solana_keypair::Keypair,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_signer::Signer,
        solana_instruction::Instruction,
        solana_transaction::Transaction,
        std::path::PathBuf,
        crate::{WhitelistEntry, HookError},
    };

    static PROGRAM_ID: Pubkey = crate::ID;

    fn setup() -> (LiteSVM, Keypair) {
        let mut program = LiteSVM::new();
        let payer = Keypair::new();
        program.airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();

        // Load the compiled program
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/hook.so");
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        program.add_program(PROGRAM_ID, &program_data);
        (program, payer)
    }

    fn derive_whitelist_pda(user: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"hook", user.as_ref()], &PROGRAM_ID)
    }

    #[test]
    fn test_add_to_whitelist() {
        let (mut program, payer) = setup();
        let user = Keypair::new();
        let (whitelist_pda, _bump) = derive_whitelist_pda(&user.pubkey());

        // Build and send AddToWhitelist ix
        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::AddToWhitelist {
                whitelist: whitelist_pda,
                authority: payer.pubkey(),
                user: user.pubkey(),
                system_program: anchor_lang::system_program::ID,
            }
                .to_account_metas(None),
            data: crate::instruction::AddToWhitelist {}.data(),
        };

        let blockhash = program.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&payer.pubkey()),
            &[&payer],
            blockhash,
        );
        program.send_transaction(tx).expect("Add whitelist failed");

        // Verify PDA contents
        let whitelist_data = program.get_account(&whitelist_pda).expect("Whitelist PDA not found");
        let whitelist_entry = WhitelistEntry::try_deserialize(&mut whitelist_data.data.as_slice())
            .expect("Failed to deserialize WhitelistEntry");

        assert_eq!(whitelist_entry.user, user.pubkey());
        println!("Added user {} to whitelist successfully!", user.pubkey());
    }

    #[test]
    fn test_validate_transfer_passes_for_whitelisted_user() {
        let (mut program, payer) = setup();
        let user = Keypair::new();

        // Derive the whitelist PDA
        let (whitelist_pda, _bump) =
            Pubkey::find_program_address(&[b"hook", user.pubkey().as_ref()], &PROGRAM_ID);

        //  Add user to whitelist via instruction
        let ix_add = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::AddToWhitelist {
                whitelist: whitelist_pda,
                authority: payer.pubkey(),
                user: user.pubkey(),
                system_program: anchor_lang::system_program::ID,
            }
                .to_account_metas(None),
            data: crate::instruction::AddToWhitelist {}.data(),
        };

        let bh = program.latest_blockhash();
        let tx_add =
            Transaction::new_signed_with_payer(&[ix_add], Some(&payer.pubkey()), &[&payer], bh);
        program.send_transaction(tx_add).expect("Add to whitelist failed");

        // Validate transfer should succeed
        let ix_validate = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::ValidateTransfer {
                from: user.pubkey(),
                to: payer.pubkey(),
                whitelist: whitelist_pda,
            }
                .to_account_metas(None),
            data: crate::instruction::ValidateTransfer {}.data(),
        };

        let bh = program.latest_blockhash();

        // Sign with BOTH the payer (for fees) AND the whitelisted user (required signer)
        let tx_validate =
            Transaction::new_signed_with_payer(&[ix_validate], Some(&payer.pubkey()), &[&payer, &user], bh);

        program.send_transaction(tx_validate).expect("Validate transfer failed");

        println!("Transfer validated successfully for whitelisted user!");
    }


    #[test]
    fn test_validate_transfer_fails_for_non_whitelisted_user() {
        let (mut program, _payer) = setup();
        let user = Keypair::new();
        let (whitelist_pda, _bump) = derive_whitelist_pda(&user.pubkey());

        // Do NOT add to whitelist
        let ix_validate = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::ValidateTransfer {
                from: user.pubkey(),
                to: Pubkey::new_unique(),
                whitelist: whitelist_pda,
            }
                .to_account_metas(None),
            data: crate::instruction::ValidateTransfer {}.data(),
        };

        let bh = program.latest_blockhash();
        let tx_validate = Transaction::new_signed_with_payer(
            &[ix_validate],
            Some(&user.pubkey()),
            &[&user],
            bh,
        );

        let err = program.send_transaction(tx_validate).unwrap_err();
        let err_str = format!("{:?}", err);
        assert!(
            err_str.contains("AccountNotFound") || err_str.contains("NotWhitelisted"),
            "Expected NotWhitelisted, got: {}",
            err_str
        );
        println!("Non-whitelisted user correctly rejected!");
    }

    #[test]
    fn test_remove_from_whitelist() {
        let (mut program, payer) = setup();
        let user = Keypair::new();
        let (whitelist_pda, _bump) = derive_whitelist_pda(&user.pubkey());

        // Add to whitelist first
        let ix_add = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::AddToWhitelist {
                whitelist: whitelist_pda,
                authority: payer.pubkey(),
                user: user.pubkey(),
                system_program: anchor_lang::system_program::ID,
            }
                .to_account_metas(None),
            data: crate::instruction::AddToWhitelist {}.data(),
        };
        let bh = program.latest_blockhash();
        let tx_add =
            Transaction::new_signed_with_payer(&[ix_add], Some(&payer.pubkey()), &[&payer], bh);
        program.send_transaction(tx_add).unwrap();

        // Remove from whitelist
        let ix_remove = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::RemoveFromWhitelist {
                whitelist: whitelist_pda,
                authority: payer.pubkey(),
            }
                .to_account_metas(None),
            data: crate::instruction::RemoveFromWhitelist {}.data(),
        };
        let bh = program.latest_blockhash();
        let tx_remove =
            Transaction::new_signed_with_payer(&[ix_remove], Some(&payer.pubkey()), &[&payer], bh);
        program.send_transaction(tx_remove).expect("Remove failed");

        // PDA should now fail to deserialize (simulates account closure)
        let whitelist_account = program.get_account(&whitelist_pda).expect("Whitelist PDA not found");
        let result = WhitelistEntry::try_deserialize(&mut whitelist_account.data.as_slice());
        assert!(result.is_err(), "Whitelist PDA should be closed");

        println!("Removed user {} and closed PDA successfully!", user.pubkey());
    }

}
