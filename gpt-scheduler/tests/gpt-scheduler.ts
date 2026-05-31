import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { GptScheduler } from "../target/types/gpt_scheduler";
import { SolanaGptOracle } from "../target/types/solana_gpt_oracle";
import {
  PublicKey,
  SystemProgram,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import {
  init as initTuktuk,
  taskKey,
  taskQueueAuthorityKey,
} from "@helium/tuktuk-sdk";
import { 
  PROGRAM_ID, 
  TASK_QUEUE, 
  GPT_SCHEDULER, 
  QUEUE_AUTHORITY,
  ORACLE_PROGRAM_ID 
} from "./constants";

const TUKTUK_PROGRAM_ID = new PublicKey("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

describe("gpt-scheduler", function () {
    this.timeout(30000);
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.GptScheduler as Program<GptScheduler>;
  const oracleProgram = anchor.workspace.SolanaGptOracle as Program<SolanaGptOracle>;
  const payer = provider.wallet as anchor.Wallet;

  const [oracleCounter] = PublicKey.findProgramAddressSync(
    [Buffer.from("counter")],
    ORACLE_PROGRAM_ID
  );

  let llmContext: PublicKey;
  let interaction: PublicKey;
  let currentCount: number;
  let queryCountBefore: number;

  const taskQueueAuthorityPda = taskQueueAuthorityKey(TASK_QUEUE, QUEUE_AUTHORITY)[0];
  let taskId = Math.floor(Math.random() * 50);

  before(async () => {
    console.log("\n============ GPT SCHEDULER SETUP ============");
    console.log("Program ID:       ", PROGRAM_ID.toBase58());
    console.log("Task Queue:       ", TASK_QUEUE.toBase58());
    console.log("Queue Authority:  ", QUEUE_AUTHORITY.toBase58());
    console.log("GPT Scheduler:    ", GPT_SCHEDULER.toBase58());
    console.log("Oracle Program:   ", ORACLE_PROGRAM_ID.toBase58());
    console.log("Payer:            ", payer.publicKey.toBase58());
    console.log("=============================================\n");

    // Read counter
    const counterAcc = await provider.connection.getAccountInfo(oracleCounter);
    currentCount = counterAcc!.data.readUInt32LE(8);
    console.log("Current counter value:", currentCount);
  });

  it("1. Initialize GPT Scheduler (skip if exists)", async () => {
    // Check if already initialized
    const existingAccount = await provider.connection.getAccountInfo(GPT_SCHEDULER);
    
    if (existingAccount) {
      console.log("✓ GPT Scheduler already initialized, skipping...");
      const scheduler = await program.account.gptScheduler.fetch(GPT_SCHEDULER);
      llmContext = scheduler.context;
      console.log("  Context:", llmContext.toBase58());
      return;
    }

    // Derive context for new init
    llmContext = PublicKey.findProgramAddressSync(
      [
        Buffer.from("test-context"),
        new BN(currentCount).toArrayLike(Buffer, "le", 4),
      ],
      ORACLE_PROGRAM_ID
    )[0];

    console.log("LLM Context:", llmContext.toBase58());

    const tx = await program.methods
      .initialize()
      .accountsPartial({
        payer: payer.publicKey,
        gptScheduler: GPT_SCHEDULER,
        counter: oracleCounter,
        llmContext: llmContext,
        oracleProgram: ORACLE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc({ skipPreflight: true });

    console.log("Tx:", tx);
    console.log("✓ Initialized");
  });

  it("2. Query GPT", async () => {
    const scheduler = await program.account.gptScheduler.fetch(GPT_SCHEDULER);
    llmContext = scheduler.context;
    queryCountBefore = scheduler.queryCount;

    interaction = PublicKey.findProgramAddressSync(
      [
        Buffer.from("interaction"),
        payer.publicKey.toBuffer(),
        llmContext.toBuffer(),
      ],
      ORACLE_PROGRAM_ID
    )[0];

    console.log("Context:", llmContext.toBase58());
    console.log("Interaction:", interaction.toBase58());
    console.log("Query count before:", queryCountBefore);

    const tx = await program.methods
      .queryGpt()
      .accountsPartial({
        payer: payer.publicKey,
        gptScheduler: GPT_SCHEDULER,
        interaction: interaction,
        contextAccount: llmContext,
        systemProgram: SystemProgram.programId,
        oracleProgram: ORACLE_PROGRAM_ID,
      })
      .rpc({ skipPreflight: true });

    console.log("Tx:", tx);

    const schedulerAfter = await program.account.gptScheduler.fetch(GPT_SCHEDULER);
    console.log("✓ Query count after:", schedulerAfter.queryCount);
    
    // Check it incremented by 1
    if (schedulerAfter.queryCount !== queryCountBefore + 1) {
      throw new Error(`Expected query count to be ${queryCountBefore + 1}, got ${schedulerAfter.queryCount}`);
    }

    console.log("\n⏳ Query sent to oracle!");
  });

  it("3. Schedule automated GPT query via TukTuk", async () => {
    console.log("\n--- Scheduling Automated GPT Query ---");

    const scheduler = await program.account.gptScheduler.fetch(GPT_SCHEDULER);
    llmContext = scheduler.context;

    interaction = PublicKey.findProgramAddressSync(
      [
        Buffer.from("interaction"),
        payer.publicKey.toBuffer(),
        llmContext.toBuffer(),
      ],
      ORACLE_PROGRAM_ID
    )[0];

    const tuktukProgram = await initTuktuk(provider);
    const task = taskKey(TASK_QUEUE, taskId)[0];

    console.log("Task ID:", taskId);
    console.log("Task PDA:", task.toBase58());

    const computeIx = ComputeBudgetProgram.setComputeUnitLimit({
      units: 400_000,
    });

    const tx = await program.methods
      .scheduleQuery(taskId)
      .accountsStrict({
        payer: payer.publicKey,
        gptScheduler: GPT_SCHEDULER,
        taskQueue: TASK_QUEUE,
        taskQueueAuthority: taskQueueAuthorityPda,
        task: task,
        queueAuthority: QUEUE_AUTHORITY,
        interaction: interaction,
        contextAccount: llmContext,
        oracleProgram: ORACLE_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        tuktukProgram: TUKTUK_PROGRAM_ID,
      })
      .preInstructions([computeIx])
      .rpc({ skipPreflight: true });

    console.log("Tx:", tx);
    console.log("✓ GPT query scheduled!");
    console.log("Task Address:", task.toBase58());
  });

  it("4. Check scheduler state", async () => {
    const scheduler = await program.account.gptScheduler.fetch(GPT_SCHEDULER);
    
    console.log("\n--- GPT Scheduler State ---");
    console.log("Context:", scheduler.context.toBase58());
    console.log("Authority:", scheduler.authority.toBase58());
    console.log("Query:", scheduler.query);
    console.log("Query Count:", scheduler.queryCount);
    console.log("Last Response:", scheduler.lastResponse || "(awaiting response)");
  });
});