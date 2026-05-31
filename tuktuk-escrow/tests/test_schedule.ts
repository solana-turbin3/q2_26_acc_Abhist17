import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { init, taskKey, taskQueueAuthorityKey, nextAvailableTaskIds } from "@helium/tuktuk-sdk";
import { AnchorEscrow } from "../target/types/anchor_escrow";
import { assert } from "chai";
import {
  createMint,
  mintTo,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddress,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, Keypair, SystemProgram, LAMPORTS_PER_SOL, Transaction } from "@solana/web3.js";

describe("tuktuk-escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.anchor_escrow as Program<AnchorEscrow>;
  const payer = (provider.wallet as any).payer as Keypair;

  const taskQueue = new PublicKey("CJv1jLvFSLsV7X1UGq6bHr6XHacbJAfq7Tio8iqpEK6b");
  const queueAuthority = PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")],
    program.programId
  )[0];
  const taskQueueAuthority = taskQueueAuthorityKey(taskQueue, queueAuthority)[0];

  let seedNonce = Date.now();
  function freshSeed(): number {
    seedNonce += 1;
    return seedNonce;
  }

  function deriveEscrowPDA(maker: PublicKey, seed: number): PublicKey {
    return PublicKey.findProgramAddressSync(
      [
        Buffer.from("escrow"),
        maker.toBuffer(),
        Buffer.from(new anchor.BN(seed).toArray("le", 8)),
      ],
      program.programId
    )[0];
  }

  async function fundWithTransfer(to: PublicKey, lamports: number) {
    const tx = new Transaction().add(
      SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        toPubkey: to,
        lamports,
      })
    );
    await provider.sendAndConfirm(tx);
  }

  async function ensureQueueAuthorityInitialized(tuktukProgram: any) {
    const existing = await tuktukProgram.account.taskQueueAuthorityV0.fetchNullable(taskQueueAuthority);
    if (!existing) {
      await tuktukProgram.methods
        .addQueueAuthorityV0()
        .accounts({
          payer: provider.publicKey,
          queueAuthority,
          taskQueue,
        })
        .rpc();
    }
  }

  async function ensureQueueFunding() {
    const queueAuthorityBalance = await provider.connection.getBalance(queueAuthority);
    if (queueAuthorityBalance < 20_000_000) {
      await fundWithTransfer(queueAuthority, 20_000_000 - queueAuthorityBalance);
    }
  }

  it("make schedules task and take executes", async () => {
    const tuktukProgram = await init(provider);
    await ensureQueueAuthorityInitialized(tuktukProgram as any);
    await ensureQueueFunding();

    const taskQueueAccount = await (tuktukProgram.account as any).taskQueueV0.fetch(taskQueue);
    const taskId = nextAvailableTaskIds(taskQueueAccount.taskBitmap, 1, false)[0];
    assert.isDefined(taskId, "No free task ID available in task queue");

    const mintA = await createMint(provider.connection, payer, payer.publicKey, null, 6);
    const mintB = await createMint(provider.connection, payer, payer.publicKey, null, 6);

    const makerAtaA = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mintA,
      provider.publicKey
    );
    await mintTo(provider.connection, payer, mintA, makerAtaA.address, payer, 1000);

    const seed = freshSeed();
    const escrow = deriveEscrowPDA(provider.publicKey, seed);
    const vault = await getAssociatedTokenAddress(mintA, escrow, true);

    await program.methods
      .make(taskId, new anchor.BN(seed), new anchor.BN(100), new anchor.BN(100))
      .accountsPartial({
        maker: provider.publicKey,
        mintA,
        mintB,
        makerAtaA: makerAtaA.address,
        escrow,
        vault,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        taskQueue,
        taskQueueAuthority,
        task: taskKey(taskQueue, taskId)[0],
        queueAuthority,
        tuktukProgram: tuktukProgram.programId,
      })
      .rpc();

    const escrowAccountAfterMake = await program.account.escrow.fetch(escrow);
    assert.equal(escrowAccountAfterMake.seed.toNumber(), seed);

    const queuedTask = await (tuktukProgram.account as any).taskV0.fetchNullable(taskKey(taskQueue, taskId)[0]);
    assert.isNotNull(queuedTask, "Task should be queued by make");

    const escrowAccount = await program.account.escrow.fetch(escrow);
    assert.equal(escrowAccount.seed.toNumber(), seed);
  });

  xit("take completes the escrow swap", async () => {
    const tuktukProgram = await init(provider);
    await ensureQueueAuthorityInitialized(tuktukProgram as any);
    await ensureQueueFunding();

    const taskQueueAccount = await (tuktukProgram.account as any).taskQueueV0.fetch(taskQueue);
    const taskId = nextAvailableTaskIds(taskQueueAccount.taskBitmap, 1, false)[0];

    const mintA = await createMint(provider.connection, payer, payer.publicKey, null, 6);
    const mintB = await createMint(provider.connection, payer, payer.publicKey, null, 6);

    // Setup maker with mint A tokens
    const makerAtaA = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mintA,
      provider.publicKey
    );
    await mintTo(provider.connection, payer, mintA, makerAtaA.address, payer, 1000);

    const seed = freshSeed();
    const escrow = deriveEscrowPDA(provider.publicKey, seed);
    const vault = await getAssociatedTokenAddress(mintA, escrow, true);

    // Make the escrow
    await program.methods
      .make(taskId, new anchor.BN(seed), new anchor.BN(100), new anchor.BN(50))
      .accountsPartial({
        maker: provider.publicKey,
        mintA,
        mintB,
        makerAtaA: makerAtaA.address,
        escrow,
        vault,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        taskQueue,
        taskQueueAuthority,
        task: taskKey(taskQueue, taskId)[0],
        queueAuthority,
        tuktukProgram: tuktukProgram.programId,
      })
      .rpc();

    // Setup taker
    const taker = Keypair.generate();
    await fundWithTransfer(taker.publicKey, LAMPORTS_PER_SOL);

    // Create taker's ATA for mint B and fund with tokens
    const takerAtaB = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      payer,
      mintB,
      taker.publicKey
    );
    await mintTo(provider.connection, payer, mintB, takerAtaB.address, payer, 1000);

    // Derive taker's ATA for mint A (will be created by take instruction)
    const takerAtaA = await getAssociatedTokenAddress(mintA, taker.publicKey);

    // Derive maker's ATA for mint B (will be created by take instruction)
    const makerAtaB = await getAssociatedTokenAddress(mintB, provider.publicKey);

    // Take the escrow
    await program.methods
      .take()
      .accountsPartial({
        taker: taker.publicKey,
        maker: provider.publicKey,
        mintA,
        mintB,
        takerAtaA,
        takerAtaB: takerAtaB.address,
        makerAtaB,
        escrow,
        vault,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([taker])
      .rpc();

    // Verify escrow is closed
    const escrowAfterTake = await program.account.escrow.fetchNullable(escrow);
    assert.isNull(escrowAfterTake, "Escrow should be closed after take");
  });
});
