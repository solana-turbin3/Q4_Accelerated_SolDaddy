import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddressSync,
  createInitializeMintInstruction,
  getMintLen,
  ExtensionType,
  createTransferCheckedWithTransferHookInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createInitializeTransferHookInstruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
} from "@solana/spl-token";
import { SendTransactionError, SystemProgram, Transaction, sendAndConfirmTransaction } from '@solana/web3.js';
import { WhitelistTransferHook } from "../target/types/whitelist_transfer_hook";

describe("whitelist-transfer-hook", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const wallet = provider.wallet as anchor.Wallet;

  const program = anchor.workspace.whitelistTransferHook as Program<WhitelistTransferHook>;

  const mint2022 = anchor.web3.Keypair.generate();

  // Sender token account address
  const sourceTokenAccount = getAssociatedTokenAddressSync(
    mint2022.publicKey,
    wallet.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  // Recipient token account address
  const recipient = anchor.web3.Keypair.generate();
  const destinationTokenAccount = getAssociatedTokenAddressSync(
    mint2022.publicKey,
    recipient.publicKey,
    false,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  // ExtraAccountMetaList address
  // Store extra accounts required by the custom transfer hook instruction
  const [extraAccountMetaListPDA] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from('extra-account-metas'), mint2022.publicKey.toBuffer()],
    program.programId,
  );

  const whitelist = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("whitelist"),
        mint2022.publicKey.toBuffer(),
        wallet.publicKey.toBuffer()
    ],
    program.programId
  )[0];

    it('Create Mint Account with Transfer Hook Extension (via Program)', async () => {
        const tx = await program.methods
            .initializeMintWithHook()
            .accountsPartial({
                payer: wallet.publicKey,
                mint: mint2022.publicKey,
                mintAuthority: wallet.publicKey,
                tokenProgram: TOKEN_2022_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .signers([mint2022])
            .rpc({ skipPreflight: true });

        console.log("\nMint created via program:", mint2022.publicKey.toBase58());
        console.log("Mint Authority:", wallet.publicKey.toBase58());
        console.log("Transaction Signature:", tx);
    });

  it("Initializes the Whitelist", async () => {
    const tx = await program.methods.initializeWhitelist()
      .accountsPartial({
        admin: provider.publicKey,
          user: wallet.publicKey,
          mint: mint2022.publicKey,
        whitelist,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("\nWhitelist initialized:", whitelist.toBase58());
    console.log("Transaction signature:", tx);
  });

  it("Add user to whitelist", async () => {
    const tx = await program.methods.addToWhitelist()
      .accountsPartial({
        admin: provider.publicKey,
        user: wallet.publicKey,
        mint: mint2022.publicKey,
        whitelist,
      })
      .rpc();

    console.log("\nUser added to whitelist:", provider.publicKey.toBase58());
    console.log("Transaction signature:", tx);
  });

  it("Remove user to whitelist", async () => {
    const tx = await program.methods.removeFromWhitelist()
      .accountsPartial({
        admin: provider.publicKey,
        user: wallet.publicKey,
        mint: mint2022.publicKey,
        whitelist,
      })
      .rpc();

    console.log("\nUser removed from whitelist:", provider.publicKey.toBase58());
    console.log("Transaction signature:", tx);
  });

  it('Create Mint Account with Transfer Hook Extension', async () => {
    const extensions = [ExtensionType.TransferHook];
    const mintLen = getMintLen(extensions);
    const lamports = await provider.connection.getMinimumBalanceForRentExemption(mintLen);

    const transaction = new Transaction().add(
      SystemProgram.createAccount({
        fromPubkey: wallet.publicKey,
        newAccountPubkey: mint2022.publicKey,
        space: mintLen,
        lamports: lamports,
        programId: TOKEN_2022_PROGRAM_ID,
      }),
      createInitializeTransferHookInstruction(
        mint2022.publicKey,
        wallet.publicKey,
        program.programId, // Transfer Hook Program ID
        TOKEN_2022_PROGRAM_ID,
      ),
      createInitializeMintInstruction(mint2022.publicKey, 9, wallet.publicKey, null, TOKEN_2022_PROGRAM_ID),
    );

    const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer, mint2022], {
      skipPreflight: true,
      commitment: 'finalized',
    });

    const txDetails = await program.provider.connection.getTransaction(txSig, {
      maxSupportedTransactionVersion: 0,
      commitment: 'confirmed',
    });
    //console.log(txDetails.meta.logMessages);

    console.log("\nTransaction Signature: ", txSig);
  });

  it('Create Token Accounts and Mint Tokens', async () => {
    // 100 tokens
    const amount = 100 * 10 ** 9;

    const transaction = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mint2022.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        destinationTokenAccount,
        recipient.publicKey,
        mint2022.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createMintToInstruction(mint2022.publicKey, sourceTokenAccount, wallet.publicKey, amount, [], TOKEN_2022_PROGRAM_ID),
    );

    const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: true });

    console.log("\nTransaction Signature: ", txSig);
  });



  it("Initialize whitelists for sender and recipient", async () => {
    // Whitelist sender
    const senderWhitelist = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("whitelist"),
          mint2022.publicKey.toBuffer(),
          wallet.publicKey.toBuffer()
        ],
        program.programId
    )[0];

    await program.methods.initializeWhitelist()
        .accountsPartial({
          admin: wallet.publicKey,
          user: wallet.publicKey,
          mint: mint2022.publicKey,
          whitelist: senderWhitelist,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

    console.log("Sender whitelisted:", wallet.publicKey.toBase58());

    // Whitelist recipient
    const recipientWhitelist = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("whitelist"),
          mint2022.publicKey.toBuffer(),
          recipient.publicKey.toBuffer()
        ],
        program.programId
    )[0];

    await program.methods.initializeWhitelist()
        .accountsPartial({
          admin: wallet.publicKey,
          user: recipient.publicKey,
          mint: mint2022.publicKey,
          whitelist: recipientWhitelist,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();

    console.log("Recipient whitelisted:", recipient.publicKey.toBase58());
  });


  // Account to store extra accounts required by the transfer hook instruction
  it('Create ExtraAccountMetaList Account', async () => {
    const initializeExtraAccountMetaListInstruction = await program.methods
      .initializeTransferHook()
      .accountsPartial({
        payer: wallet.publicKey,
        mint: mint2022.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
      //.rpc();

    const transaction = new Transaction().add(initializeExtraAccountMetaListInstruction);

    const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: true, commitment: 'confirmed' });
    console.log("\nExtraAccountMetaList Account created:", extraAccountMetaListPDA.toBase58());
    console.log('Transaction Signature:', txSig);
  });

  it('Transfer Hook with Extra Account Meta', async () => {
    // 1 tokens
    const amount = 1 * 10 ** 9;
    const amountBigInt = BigInt(amount);

    const transferInstructionWithHelper = await createTransferCheckedWithTransferHookInstruction(
      provider.connection,
      sourceTokenAccount,
      mint2022.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amountBigInt,
      9,
      [],
      'confirmed',
      TOKEN_2022_PROGRAM_ID,
    );

    const transaction = new Transaction().add(transferInstructionWithHelper);

    try {
      // Send the transaction
      const txSig = await sendAndConfirmTransaction(provider.connection, transaction, [wallet.payer], { skipPreflight: false });
      console.log("\nTransfer Signature:", txSig);
    }
    catch (error) {
      if (error instanceof SendTransactionError) {
        console.error("\nTransaction failed:", error.logs[4]);
      } else {
        console.error("\nUnexpected error:", error);
      }
    }
  });
});
