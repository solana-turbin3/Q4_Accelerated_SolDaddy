import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {DELEGATION_PROGRAM_ID} from "@magicblock-labs/ephemeral-rollups-sdk";
import { ErStateAccount } from "../target/types/er_state_account";

describe("er-state-account", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const providerEphemeralRollup = new anchor.AnchorProvider(
    new anchor.web3.Connection(process.env.EPHEMERAL_PROVIDER_ENDPOINT || "https://devnet.magicblock.app/", {wsEndpoint: process.env.EPHEMERAL_WS_ENDPOINT || "wss://devnet.magicblock.app/"}
    ),
    anchor.Wallet.local()
  );
  console.log("Base Layer Connection: ", provider.connection.rpcEndpoint);
  console.log("Ephemeral Rollup Connection: ", providerEphemeralRollup.connection.rpcEndpoint);
  console.log(`Current SOL Public Key: ${anchor.Wallet.local().publicKey}`)

  before(async function () {

    const balance = await provider.connection.getBalance(anchor.Wallet.local().publicKey);
    console.log('Current balance is', balance / LAMPORTS_PER_SOL, ' SOL\n');

    // Check if account already exists
    try {
      const accountInfo = await provider.connection.getAccountInfo(userAccount);

      if (accountInfo) {
        console.log(`Account ${userAccount.toString()} already exists`);
        console.log(`Owner: ${accountInfo.owner.toString()}`);
        console.log(`Expected owner: ${program.programId.toString()}`);

        // Check if it's delegated (owned by delegation program)
        const DELEGATION_PROGRAM = "DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh";

        if (accountInfo.owner.toString() === DELEGATION_PROGRAM) {
          console.log("Account is still delegated from previous test!");
        } else {
          console.log("Account exists and owned by correct program - skipping initialization");
          return;
        }
      }
    } catch (error) {
      if (error.message.includes("delegated") || error.message.includes("wrong program")) {
        throw error; // Re-throw our custom errors
      }
      // Account doesn't exist - will be created in test
      console.log("No existing account found - will create fresh account");
    }
  });

  const program = anchor.workspace.erStateAccount as Program<ErStateAccount>;

  const userAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("user"), anchor.Wallet.local().publicKey.toBuffer()],
    program.programId
  )[0];

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods.initialize().accountsPartial({
      user: anchor.Wallet.local().publicKey,
      userAccount: userAccount,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();
    console.log("User Account initialized: ", tx);
  });
  //
  // it("Update State!", async () => {
  //   const tx = await program.methods.update(new anchor.BN(42)).accountsPartial({
  //     user: anchor.Wallet.local().publicKey,
  //     userAccount: userAccount,
  //   })
  //   .rpc();
  //   console.log("\nUser Account State Updated: ", tx);
  // });
  //
  // it("Delegate to Ephemeral Rollup!", async () => {
  //
  //   let tx = await program.methods.delegate().accountsPartial({
  //     user: anchor.Wallet.local().publicKey,
  //     userAccount: userAccount,
  //     validator: new PublicKey("MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57"),
  //     systemProgram: anchor.web3.SystemProgram.programId,
  //   }).rpc({skipPreflight: true});
  //
  //   console.log("\nUser Account Delegated to Ephemeral Rollup: ", tx);
  // });
  //
  // it("Update State and Commit to Base Layer!", async () => {
  //   let tx = await program.methods.updateCommit(new anchor.BN(43)).accountsPartial({
  //     user: providerEphemeralRollup.wallet.publicKey,
  //     userAccount: userAccount,
  //   })
  //   .transaction();
  //
  //   tx.feePayer = providerEphemeralRollup.wallet.publicKey;
  //
  //   tx.recentBlockhash = (await providerEphemeralRollup.connection.getLatestBlockhash()).blockhash;
  //   tx = await providerEphemeralRollup.wallet.signTransaction(tx);
  //   const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {skipPreflight: false});
  //   const txCommitSgn = await GetCommitmentSignature(
  //     txHash,
  //     providerEphemeralRollup.connection
  // );
  //
  //   console.log("\nUser Account State Updated: ", txHash);
  // });
  //
  // it("Commit and undelegate from Ephemeral Rollup!", async () => {
  //   let info = await providerEphemeralRollup.connection.getAccountInfo(userAccount);
  //
  //   console.log("User Account Info: ", info);
  //
  //   console.log("User account", userAccount.toBase58());
  //
  //   let tx = await program.methods.undelegate().accounts({
  //     user: providerEphemeralRollup.wallet.publicKey,
  //   })
  //   .transaction();
  //
  //   tx.feePayer = providerEphemeralRollup.wallet.publicKey;
  //
  //   tx.recentBlockhash = (await providerEphemeralRollup.connection.getLatestBlockhash()).blockhash;
  //   tx = await providerEphemeralRollup.wallet.signTransaction(tx);
  //   const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], {skipPreflight: false});
  //   const txCommitSgn = await GetCommitmentSignature(
  //     txHash,
  //     providerEphemeralRollup.connection
  // );
  //
  //   console.log("\nUser Account Undelegated: ", txHash);
  // });

  // it("Update State!", async () => {
  //   let tx = await program.methods.update(new anchor.BN(45)).accountsPartial({
  //     user: anchor.Wallet.local().publicKey,
  //     userAccount: userAccount,
  //   })
  //   .rpc();
  //
  //   console.log("\nUser Account State Updated: ", tx);
  // });



  it("Request Randomness (Undelegated)", async () => {
    const clientSeed = Math.floor(Math.random() * 256);

    const tx = await program.methods
        .requestRandomnessUndelegated(clientSeed)
        .accountsPartial({
          user: anchor.Wallet.local().publicKey,
          userAccount: userAccount,
          // oracleQueue: new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh"),
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc({ skipPreflight: true });

    console.log("\nRandomness requested (undelegated) with seed:", clientSeed);
    console.log("Transaction:", tx);
    console.log(`View: https://solscan.io/tx/${tx}?cluster=devnet`);

    // Wait longer for oracle callback
    console.log("Waiting 15 seconds for oracle callback...");
    await new Promise(resolve => setTimeout(resolve, 15000));

    // Fetch and verify the random_value was updated
    const userAccountData = await program.account.userAccount.fetch(userAccount);
    console.log("Random value:", userAccountData.randomValue.toString());
    console.log("Current data:", userAccountData.data.toString());

    if (userAccountData.randomValue.toString() !== "0") {
      console.log("VRF callback executed successfully!");
    } else {
      console.log("Oracle isn't responding :(");
    }
  });

  // //
  // it("Delegate to Ephemeral Rollup", async () => {
  //   let tx = await program.methods.delegate().accountsPartial({
  //     user: anchor.Wallet.local().publicKey,
  //     userAccount: userAccount,
  //     validator: new PublicKey("MAS1Dt9qreoRMQ14YQuhg8UTZMMzDdKhmkZMECCzk57"),
  //     systemProgram: anchor.web3.SystemProgram.programId,
  //   }).rpc({skipPreflight: true});
  //
  //   console.log("\nUser Account Delegated to Ephemeral Rollup: ", tx);
  //
  //   // Wait for delegation to process
  //   console.log("Waiting for account to be transferred to ER...");
  //   await new Promise(resolve => setTimeout(resolve, 5000));
  //
  //   // Verify account is on ER by fetching it
  //   const programER = new Program(program.idl, providerEphemeralRollup);
  //   try {
  //     const accountOnER = await programER.account.userAccount.fetch(userAccount);
  //     console.log("Account successfully delegated to ER");
  //     console.log("Data on ER:", accountOnER.data.toString());
  //   } catch (error) {
  //     console.log("Account not yet on ER");
  //   }
  // });
  //
  //
  // it("Request Randomness (Delegated)", async () => {
  //   const accountInfo = await providerEphemeralRollup.connection.getAccountInfo(userAccount);
  //   console.log("Account owner:", accountInfo?.owner.toString());
  //   console.log("Expected owner:", program.programId.toString());
  //
  //   const clientSeed = Math.floor(Math.random() * 256);
  //
  //   try {
  //     let tx = await program.methods
  //         .requestRandomnessDelegated(clientSeed)
  //         .accountsPartial({
  //           user: providerEphemeralRollup.wallet.publicKey,
  //           userAccount: userAccount,
  //           oracleQueue: new PublicKey("Cuj97ggrhhidhbu39TijNVqE74xvKJ69gDervRUXAxGh"),
  //         })
  //         .transaction();
  //
  //     tx.feePayer = providerEphemeralRollup.wallet.publicKey;
  //     tx.recentBlockhash = (await providerEphemeralRollup.connection.getLatestBlockhash()).blockhash;
  //     tx = await providerEphemeralRollup.wallet.signTransaction(tx);
  //     const txHash = await providerEphemeralRollup.sendAndConfirm(tx, [], { skipPreflight: false });
  //
  //     console.log("\nRandomness requested (delegated) with seed:", clientSeed);
  //     console.log("Transaction:", txHash);
  //
  //     await new Promise(resolve => setTimeout(resolve, 10000));
  //
  //     const programER = new Program(program.idl, providerEphemeralRollup);
  //     const userAccountData = await programER.account.userAccount.fetch(userAccount);
  //     console.log("Random value:", userAccountData.randomValue.toString());
  //
  //     if (userAccountData.randomValue.toString() !== "0") {
  //       console.log("VRF callback executed successfully!");
  //     }
  //   } catch (error: any) {
  //     console.error("Full error:", error);
  //     if (error.logs) {
  //       console.error("Transaction logs:", error.logs);
  //     }
  //     throw error;
  //   }
  // });

  it("Close Account!", async () => {
    const tx = await program.methods.close().accountsPartial({
      user: anchor.Wallet.local().publicKey,
      userAccount: userAccount,
      systemProgram: anchor.web3.SystemProgram.programId,
    })
        .rpc();
    console.log("\nUser Account Closed: ", tx);
  });


});
