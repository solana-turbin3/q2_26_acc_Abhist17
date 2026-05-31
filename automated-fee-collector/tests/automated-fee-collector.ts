import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { AutomatedFeeCollector } from "../target/types/automated_fee_collector";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAccount,
  getTransferFeeAmount,
} from "@solana/spl-token";
import { assert } from "chai";
import {
  init as initTuktuk,
  taskKey,
  taskQueueAuthorityKey,
} from "@helium/tuktuk-sdk";
import { PROGRAM_ID, TASK_QUEUE, FEE_AUTHORITY, QUEUE_AUTHORITY } from "./constants";

describe("automated-fee-collector", function () {
  this.timeout(60000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.AutomatedFeeCollector as Program<AutomatedFeeCollector>;
  const payer = provider.wallet as anchor.Wallet;

  // Fresh accounts for each test run
  const mintKeypair = Keypair.generate();
  const alice = Keypair.generate();
  const bob = Keypair.generate();
  const treasuryAuthority = Keypair.generate();

  // Task Queue Authority PDA (from tuktuk)
  const taskQueueAuthorityPda = taskQueueAuthorityKey(TASK_QUEUE, QUEUE_AUTHORITY)[0];

  // Token accounts
  let aliceTokenAccount: PublicKey;
  let bobTokenAccount: PublicKey;
  let treasuryTokenAccount: PublicKey;

  // Parameters
  const DECIMALS = 9;
  const TRANSFER_FEE_BASIS_POINTS = 500; // 5%
  const MAXIMUM_FEE = new BN("1000000000000"); // 1000 tokens
  const MINT_AMOUNT = new BN("1000000000000"); // 1000 tokens
  const TRANSFER_AMOUNT = new BN("100000000000"); // 100 tokens

  // Minimal SOL amounts (in lamports)
  // 0.01 SOL = 10_000_000 lamports - enough for a few transactions + ATA rent
  const FUNDING_AMOUNT = 0.01 * LAMPORTS_PER_SOL; // 10,000,000 lamports

  let taskId = Math.floor(Math.random() * 100);

  before(async () => {
    console.log("\n============ SETUP ============");
    console.log("Program ID:      ", PROGRAM_ID.toBase58());
    console.log("Task Queue:      ", TASK_QUEUE.toBase58());
    console.log("Fee Authority:   ", FEE_AUTHORITY.toBase58());
    console.log("Queue Authority: ", QUEUE_AUTHORITY.toBase58());
    console.log("Payer:           ", payer.publicKey.toBase58());
    console.log("================================\n");

    // Check payer balance
    const payerBalance = await provider.connection.getBalance(payer.publicKey);
    console.log("Payer balance:", payerBalance / LAMPORTS_PER_SOL, "SOL");
    
    const totalNeeded = FUNDING_AMOUNT * 3; // For Alice, Bob, Treasury
    if (payerBalance < totalNeeded + 0.01 * LAMPORTS_PER_SOL) {
      throw new Error(`Insufficient balance. Need at least ${(totalNeeded + 0.01 * LAMPORTS_PER_SOL) / LAMPORTS_PER_SOL} SOL`);
    }

    // Fund test accounts by transferring from payer wallet
    console.log("Funding test accounts (0.01 SOL each)...");
    
    const fundingTx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: alice.publicKey,
        lamports: FUNDING_AMOUNT,
      }),
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: bob.publicKey,
        lamports: FUNDING_AMOUNT,
      }),
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: treasuryAuthority.publicKey,
        lamports: FUNDING_AMOUNT,
      })
    );

    const sig = await provider.sendAndConfirm(fundingTx);
    console.log("Funding tx:", sig);
    console.log(`✓ Alice:    ${alice.publicKey.toBase58()}`);
    console.log(`✓ Bob:      ${bob.publicKey.toBase58()}`);
    console.log(`✓ Treasury: ${treasuryAuthority.publicKey.toBase58()}`);

    // Derive token account addresses
    aliceTokenAccount = getAssociatedTokenAddressSync(
      mintKeypair.publicKey,
      alice.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    bobTokenAccount = getAssociatedTokenAddressSync(
      mintKeypair.publicKey,
      bob.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    treasuryTokenAccount = getAssociatedTokenAddressSync(
      mintKeypair.publicKey,
      treasuryAuthority.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID
    );

    console.log("\nMint:", mintKeypair.publicKey.toBase58());
    console.log("Alice ATA:", aliceTokenAccount.toBase58());
    console.log("Bob ATA:", bobTokenAccount.toBase58());
    console.log("Treasury ATA:", treasuryTokenAccount.toBase58());
    console.log("");
  });

  it("1. Initialize mint with transfer fee", async () => {
    const tx = await program.methods
      .initMint(DECIMALS, TRANSFER_FEE_BASIS_POINTS, MAXIMUM_FEE)
      .accounts({
        authority: payer.publicKey,
        mint: mintKeypair.publicKey,
      })
      .signers([mintKeypair])
      .rpc();

    console.log("Tx:", tx);
    console.log("✓ Mint created with 5% transfer fee");
  });

  it("2. Initialize treasury", async () => {
    const tx = await program.methods
      .initTreasury()
      .accounts({
        payer: payer.publicKey,
        treasuryAuthority: treasuryAuthority.publicKey,
        mint: mintKeypair.publicKey,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .rpc();

    console.log("Tx:", tx);
    console.log("✓ Treasury initialized");
  });

  it("3. Mint tokens to Alice", async () => {
    const tx = await program.methods
      .mintTo(MINT_AMOUNT)
      .accounts({
        authority: payer.publicKey,
        mint: mintKeypair.publicKey,
        recipient: alice.publicKey,
      })
      .rpc();

    console.log("Tx:", tx);
    
    const balance = (await getAccount(provider.connection, aliceTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID)).amount;
    console.log("✓ Minted 1000 tokens to Alice. Balance:", balance.toString());
  });

  it("4. Mint tokens to Bob", async () => {
    const tx = await program.methods
      .mintTo(MINT_AMOUNT)
      .accounts({
        authority: payer.publicKey,
        mint: mintKeypair.publicKey,
        recipient: bob.publicKey,
      })
      .rpc();

    console.log("Tx:", tx);
    console.log("✓ Minted 1000 tokens to Bob");
  });

  it("5. Alice → Bob transfer (fees accumulate)", async () => {
    const tx = await program.methods
      .transfer(TRANSFER_AMOUNT)
      .accounts({
        sender: alice.publicKey,
        recipient: bob.publicKey,
        mintAccount: mintKeypair.publicKey,
      })
      .signers([alice])
      .rpc();

    console.log("Tx:", tx);

    const bobAccount = await getAccount(provider.connection, bobTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const withheld = getTransferFeeAmount(bobAccount)?.withheldAmount || BigInt(0);
    
    console.log("✓ Transferred 100 tokens");
    console.log("  Fee withheld on Bob's account:", withheld.toString());
  });

  it("6. Bob → Alice transfer (more fees)", async () => {
    const tx = await program.methods
      .transfer(TRANSFER_AMOUNT)
      .accounts({
        sender: bob.publicKey,
        recipient: alice.publicKey,
        mintAccount: mintKeypair.publicKey,
      })
      .signers([bob])
      .rpc();

    console.log("Tx:", tx);

    const aliceAccount = await getAccount(provider.connection, aliceTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const withheld = getTransferFeeAmount(aliceAccount)?.withheldAmount || BigInt(0);
    
    console.log("✓ Transferred 100 tokens");
    console.log("  Fee withheld on Alice's account:", withheld.toString());
  });

  it("7. Collect fees to treasury", async () => {
    // Get totals before
    const treasuryBefore = await getAccount(provider.connection, treasuryTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    
    const aliceAcc = await getAccount(provider.connection, aliceTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const bobAcc = await getAccount(provider.connection, bobTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const totalWithheld = 
      (getTransferFeeAmount(aliceAcc)?.withheldAmount || BigInt(0)) +
      (getTransferFeeAmount(bobAcc)?.withheldAmount || BigInt(0));
    
    console.log("Total withheld fees:", totalWithheld.toString());

    const tx = await program.methods
      .manualCollect()
      .accounts({
        mintAccount: mintKeypair.publicKey,
        treasuryTokenAccount: treasuryTokenAccount,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .remainingAccounts([
        { pubkey: aliceTokenAccount, isSigner: false, isWritable: true },
        { pubkey: bobTokenAccount, isSigner: false, isWritable: true },
      ])
      .rpc();

    console.log("Tx:", tx);

    // Verify
    const treasuryAfter = await getAccount(provider.connection, treasuryTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const collected = BigInt(treasuryAfter.amount.toString()) - BigInt(treasuryBefore.amount.toString());
    
    console.log("✓ Fees collected:", collected.toString());
    assert.equal(collected.toString(), totalWithheld.toString());

    // Verify withheld is now 0
    const aliceAfter = await getAccount(provider.connection, aliceTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const bobAfter = await getAccount(provider.connection, bobTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    
    assert.equal(getTransferFeeAmount(aliceAfter)?.withheldAmount.toString(), "0");
    assert.equal(getTransferFeeAmount(bobAfter)?.withheldAmount.toString(), "0");
    console.log("✓ All withheld amounts reset to 0");
  });

    it("8. Schedule automated collection via TukTuk", async () => {
    // Make more transfers first
    console.log("Making transfers to accumulate fees...");
    
    await program.methods
      .transfer(TRANSFER_AMOUNT)
      .accounts({
        sender: alice.publicKey,
        recipient: bob.publicKey,
        mintAccount: mintKeypair.publicKey,
      })
      .signers([alice])
      .rpc();

    // Check fees accumulated
    const bobAcc = await getAccount(provider.connection, bobTokenAccount, "confirmed", TOKEN_2022_PROGRAM_ID);
    const withheld = getTransferFeeAmount(bobAcc)?.withheldAmount || BigInt(0);
    console.log("Fees to collect:", withheld.toString());

    // Schedule
    const tuktukProgram = await initTuktuk(provider);
    const task = taskKey(TASK_QUEUE, taskId)[0];

    console.log("Scheduling task ID:", taskId);
    console.log("Task PDA:", task.toBase58());
    console.log("Queue Authority:", QUEUE_AUTHORITY.toBase58());
    console.log("Fee Authority:", FEE_AUTHORITY.toBase58());
    console.log("Task Queue Authority:", taskQueueAuthorityPda.toBase58());

    try {
      // Build the transaction manually to add compute budget
      const modifyComputeUnits = anchor.web3.ComputeBudgetProgram.setComputeUnitLimit({
        units: 400_000,
      });

      const tx = await program.methods
        .schedule(taskId)
        .accountsStrict({
          payer: payer.publicKey,
          taskQueue: TASK_QUEUE,
          taskQueueAuthority: taskQueueAuthorityPda,
          task: task,
          queueAuthority: QUEUE_AUTHORITY,
          feeAuthority: FEE_AUTHORITY,
          mintAccount: mintKeypair.publicKey,
          treasuryTokenAccount: treasuryTokenAccount,
          tokenProgram: TOKEN_2022_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
          tuktukProgram: new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA"),
        })
        .remainingAccounts([
          { pubkey: aliceTokenAccount, isSigner: false, isWritable: true },
          { pubkey: bobTokenAccount, isSigner: false, isWritable: true },
        ])
        .preInstructions([modifyComputeUnits])
        .rpc();

      console.log("Tx:", tx);
      console.log("✓ Task scheduled!");
      console.log("  Task Address:", task.toBase58());
      
    } catch (e: any) {
      // Try to get transaction logs
      if (e.signature) {
        const txDetails = await provider.connection.getTransaction(e.signature, {
          commitment: "confirmed",
          maxSupportedTransactionVersion: 0,
        });
        console.log("\n=== Transaction Logs ===");
        console.log(txDetails?.meta?.logMessages?.join("\n"));
      }
      
      // Also try from e.logs if available
      if (e.logs) {
        console.log("\n=== Error Logs ===");
        console.log(e.logs.join("\n"));
      }

      console.log("\n=== Full Error ===");
      console.log(JSON.stringify(e, null, 2));
      
      throw e;
    }
  });
});