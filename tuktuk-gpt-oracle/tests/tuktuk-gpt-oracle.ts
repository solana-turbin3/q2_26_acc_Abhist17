import * as anchor from "@coral-xyz/anchor";
import { BN, Program, web3 } from "@coral-xyz/anchor";
import { assert } from "chai";
import { TuktukGptOracle } from "../target/types/tuktuk_gpt_oracle";
import { SolanaGptOracle } from "../app/solana-gpt-oracle";
import IDL_LLM from "../app/solana-gpt-oracle.json";
import {
  init,
  taskKey,
  taskQueueAuthorityKey,
  nextAvailableTaskIds,
} from "@helium/tuktuk-sdk";
import {
  PublicKey,
  Transaction,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

describe("tuktuk-gpt-oracle", () => {
  // ============================================================
  // Setup
  // ============================================================
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Our program (loaded via anchor workspace)
  const program = anchor.workspace.TuktukGptOracle as Program<TuktukGptOracle>;

  // Oracle program — instantiated from the LLM oracle IDL JSON
  const ORACLE_PROGRAM_ID = new web3.PublicKey(
    "LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab"
  );

  const oracleProgram = new anchor.Program(
    IDL_LLM as anchor.Idl,
    provider
  ) as unknown as Program<SolanaGptOracle>;

  // ============================================================
  // PDA Derivations
  // ============================================================

  // Agent PDA — our program, seeds = ["agent"]
  const [agentPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("agent")],
    program.programId
  );

  // Counter PDA — oracle program, seeds = ["counter"]
  const [counterPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("counter")],
    ORACLE_PROGRAM_ID
  );

  // Identity PDA — oracle program, seeds = ["identity"]
  const [identityPda] = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("identity")],
    ORACLE_PROGRAM_ID
  );

  // ============================================================
  // Helpers
  // ============================================================

  // ============================================================
  // TukTuk Scheduler Constants
  // ============================================================

  const taskQueue = new web3.PublicKey(
    "CJv1jLvFSLsV7X1UGq6bHr6XHacbJAfq7Tio8iqpEK6b"
  );
  const queueAuthority = web3.PublicKey.findProgramAddressSync(
    [Buffer.from("queue_authority")],
    program.programId
  )[0];
  const taskQueueAuthorityPda = taskQueueAuthorityKey(
    taskQueue,
    queueAuthority
  )[0];

  let scheduleSeedNonce = Date.now();
  function freshScheduleSeed(): number {
    scheduleSeedNonce += 1;
    return scheduleSeedNonce;
  }

  async function fundWithTransfer(to: web3.PublicKey, lamports: number) {
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
    const existing =
      await tuktukProgram.account.taskQueueAuthorityV0.fetchNullable(
        taskQueueAuthorityPda
      );
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
    const queueAuthorityBalance = await provider.connection.getBalance(
      queueAuthority
    );
    if (queueAuthorityBalance < 20_000_000) {
      await fundWithTransfer(
        queueAuthority,
        20_000_000 - queueAuthorityBalance
      );
    }
  }

  // ============================================================
  // Helpers
  // ============================================================

  /** Derive oracle ContextAccount PDA: seeds = ["test-context", count_le_u32] */
  const deriveContextPda = (count: number): web3.PublicKey => {
    return web3.PublicKey.findProgramAddressSync(
      [Buffer.from("test-context"), new BN(count).toArrayLike(Buffer, "le", 4)],
      ORACLE_PROGRAM_ID
    )[0];
  };

  /** Derive AnalysisResult PDA: seeds = ["analysis", user_pubkey] on our program */
  const deriveAnalysisResultPda = (
    userPubkey: web3.PublicKey
  ): web3.PublicKey => {
    return web3.PublicKey.findProgramAddressSync(
      [Buffer.from("analysis"), userPubkey.toBuffer()],
      program.programId
    )[0];
  };

  /** Derive Interaction PDA: seeds = ["interaction", payer, context_account] on oracle */
  const deriveInteractionPda = (
    payer: web3.PublicKey,
    contextAccount: web3.PublicKey
  ): web3.PublicKey => {
    return web3.PublicKey.findProgramAddressSync(
      [Buffer.from("interaction"), payer.toBuffer(), contextAccount.toBuffer()],
      ORACLE_PROGRAM_ID
    )[0];
  };

  /** Confirm a transaction on-chain */
  const confirmTx = async (tx: string) => {
    const latestBlockhash = await provider.connection.getLatestBlockhash();
    await provider.connection.confirmTransaction(
      { signature: tx, ...latestBlockhash },
      "confirmed"
    );
  };

  // ============================================================
  // Shared State
  // ============================================================

  let contextAccountPda: web3.PublicKey;
  const userToAnalyze = web3.Keypair.generate();

  // ============================================================
  // 1. INITIALIZE
  // ============================================================

  describe("1. Initialize", () => {
    it("initializes the oracle program if needed (Counter + Identity)", async () => {
      // The oracle must be initialized before our program can CPI into it.
      // If already initialized on devnet, this is a no-op.
      try {
        const counterAccount = await oracleProgram.account.counter.fetch(
          counterPda
        );
        console.log(
          `  Oracle already initialized — counter value: ${counterAccount.count}`
        );
      } catch {
        console.log("  Oracle not initialized. Calling oracle.initialize()…");
        try {
          const tx = await oracleProgram.methods
            .initialize()
            .accountsPartial({
              payer: provider.wallet.publicKey,
            })
            .rpc();
          await confirmTx(tx);
          console.log("  Oracle initialized, tx:", tx);
        } catch (e: any) {
          // Could be already initialized (race condition)
          const counterAccount = await oracleProgram.account.counter.fetch(
            counterPda
          );
          console.log(
            `  Oracle init race, counter exists — value: ${counterAccount.count}`
          );
        }
      }
    });

    it("initializes the Agent and creates an LLM context via CPI", async () => {
      // Check if Agent PDA already exists (persists across devnet runs)
      let agentAlreadyExists = false;
      try {
        const existing = await program.account.agent.fetch(agentPda);
        if (existing) {
          agentAlreadyExists = true;
          contextAccountPda = existing.context;
          console.log("  Agent already initialized from a previous run");
          console.log("  Agent account:", {
            context: existing.context.toBase58(),
            bump: existing.bump,
          });
        }
      } catch {
        // Agent does not exist yet — proceed with init
      }

      if (!agentAlreadyExists) {
        // Read current counter to know which context PDA will be created
        const counterAccount = await oracleProgram.account.counter.fetch(
          counterPda
        );
        const currentCount = counterAccount.count;
        contextAccountPda = deriveContextPda(currentCount);

        console.log(`  Current oracle counter: ${currentCount}`);
        console.log(`  Expected context PDA : ${contextAccountPda.toBase58()}`);
        console.log(`  Agent PDA            : ${agentPda.toBase58()}`);

        const tx = await program.methods
          .initialize()
          .accountsPartial({
            llmContext: contextAccountPda,
            counter: counterPda,
          })
          .rpc({ skipPreflight: true });

        await confirmTx(tx);
        console.log("  Initialize tx:", tx);
      }

      // ---- Verify Agent account ----
      const agentAccount = await program.account.agent.fetch(agentPda);

      assert.ok(
        agentAccount.context.equals(contextAccountPda),
        "Agent.context should equal the context PDA"
      );
      assert.isAbove(agentAccount.bump, 0, "Agent.bump should be non-zero");

      if (!agentAlreadyExists) {
        console.log("  Agent account:", {
          context: agentAccount.context.toBase58(),
          bump: agentAccount.bump,
        });
      }

      // ---- Verify context account was created on the oracle program ----
      const contextInfo = await provider.connection.getAccountInfo(
        contextAccountPda
      );
      assert.isNotNull(contextInfo, "Context account should exist on-chain");
      assert.ok(
        contextInfo!.owner.equals(ORACLE_PROGRAM_ID),
        "Context account should be owned by the oracle program"
      );
    });

    it("fails when calling initialize again (Agent PDA already exists)", async () => {
      // Agent uses `init`, so a second call must fail
      try {
        const dummyContext = deriveContextPda(9999);
        await program.methods
          .initialize()
          .accountsPartial({
            llmContext: dummyContext,
            counter: counterPda,
          })
          .rpc();

        assert.fail("Should have thrown — Agent PDA already initialized");
      } catch (e: any) {
        console.log(
          "  Expected error on re-init:",
          e.message?.substring(0, 120)
        );
        assert.ok(e, "Re-initialization must fail");
      }
    });
  });

  // ============================================================
  // 2. ANALYZE USER
  // ============================================================

  describe("2. Analyze User", () => {
    before(async () => {
      // Reload the Agent to get the context address
      const agentAccount = await program.account.agent.fetch(agentPda);
      contextAccountPda = agentAccount.context;
    });

    it("submits user data for LLM analysis", async () => {
      const analysisResultPda = deriveAnalysisResultPda(
        userToAnalyze.publicKey
      );
      const interactionPda = deriveInteractionPda(
        provider.wallet.publicKey,
        contextAccountPda
      );

      console.log("  User to analyze    :", userToAnalyze.publicKey.toBase58());
      console.log("  Analysis result PDA:", analysisResultPda.toBase58());
      console.log("  Interaction PDA    :", interactionPda.toBase58());

      const userData = JSON.stringify({
        recentTransactions: [
          {
            type: "swap",
            amount: 10,
            from: "SOL",
            to: "USDC",
            protocol: "Jupiter",
          },
          { type: "stake", amount: 5, token: "SOL", protocol: "Marinade" },
          { type: "transfer", amount: 2, token: "SOL", direction: "out" },
        ],
        balances: { SOL: 15.5, USDC: 1200, mSOL: 5.0 },
        walletAge: "6 months",
      });

      const tx = await program.methods
        .analyzeUser(userToAnalyze.publicKey, userData)
        .accountsPartial({
          contextAccount: contextAccountPda,
        })
        .rpc({ skipPreflight: true });

      await confirmTx(tx);
      console.log("  Analyze user tx:", tx);

      // ---- Verify analysis_result was created and initialized ----
      const analysisResult = await program.account.analysisResult.fetch(
        analysisResultPda
      );

      // analyze_user now initializes user + bump; analysis is empty until callback
      assert.ok(
        analysisResult.user.equals(userToAnalyze.publicKey),
        "user field should match the user_pubkey argument"
      );
      assert.isString(
        analysisResult.analysis,
        "analysis field should be a string"
      );
      assert.strictEqual(
        analysisResult.analysis,
        "",
        "analysis should be empty (awaiting oracle callback)"
      );
      assert.isNumber(analysisResult.bump, "bump should be a number");
      assert.isAbove(analysisResult.bump, 0, "bump should be non-zero");

      console.log("  Analysis result account:", {
        user: analysisResult.user.toBase58(),
        analysis:
          analysisResult.analysis || "(empty — awaiting oracle callback)",
        timestamp: analysisResult.timestamp.toString(),
        bump: analysisResult.bump,
      });

      // Verify the oracle interaction was created
      const interactionInfo = await provider.connection.getAccountInfo(
        interactionPda
      );
      assert.isNotNull(
        interactionInfo,
        "Interaction account should exist on oracle program"
      );
      console.log("  Interaction created:", interactionInfo ? "YES" : "NO");
    });

    it("re-submits for the same user (init_if_needed is idempotent)", async () => {
      const analysisResultPda = deriveAnalysisResultPda(
        userToAnalyze.publicKey
      );
      const interactionPda = deriveInteractionPda(
        provider.wallet.publicKey,
        contextAccountPda
      );

      const updatedData =
        "Updated: User recently swapped 50 SOL to USDC on Jupiter.";

      const tx = await program.methods
        .analyzeUser(userToAnalyze.publicKey, updatedData)
        .accountsPartial({
          contextAccount: contextAccountPda,
        })
        .rpc({ skipPreflight: true });

      await confirmTx(tx);
      console.log("  Re-submit tx:", tx);

      // Account still exists and is valid
      const result = await program.account.analysisResult.fetch(
        analysisResultPda
      );
      assert.ok(result, "Analysis result account should still exist");
    });

    it("creates a separate analysis for a different user", async () => {
      const anotherUser = web3.Keypair.generate();
      const analysisResultPda = deriveAnalysisResultPda(anotherUser.publicKey);
      const interactionPda = deriveInteractionPda(
        provider.wallet.publicKey,
        contextAccountPda
      );

      const tx = await program.methods
        .analyzeUser(
          anotherUser.publicKey,
          "User has been active for 1 year on Raydium and Orca."
        )
        .accountsPartial({
          contextAccount: contextAccountPda,
        })
        .rpc({ skipPreflight: true });

      await confirmTx(tx);
      console.log("  Analyze another user tx:", tx);

      const result = await program.account.analysisResult.fetch(
        analysisResultPda
      );
      assert.ok(result, "Second user's analysis result should exist");
    });
  });

  // ============================================================
  // 3. CALLBACK FROM AGENT (negative tests — requires oracle signer)
  // ============================================================

  describe("3. Callback From Agent", () => {
    it("fails when called directly — identity is not a CPI signer", async () => {
      const analysisResultPda = deriveAnalysisResultPda(
        userToAnalyze.publicKey
      );

      try {
        await program.methods
          .callbackFromAgent("Fake analysis — should not succeed")
          .accountsPartial({
            userPubkey: userToAnalyze.publicKey,
            identity: identityPda,
          })
          .rpc();

        assert.fail(
          "Should have thrown — identity is not a signer outside CPI"
        );
      } catch (e: any) {
        const errStr = e.toString();
        console.log(
          "  Expected error (identity not signer):",
          errStr.substring(0, 160)
        );

        // The program checks identity.is_signer and returns InvalidOracleCallback
        assert.ok(
          errStr.includes("InvalidOracleCallback") ||
            errStr.includes("invalid") ||
            errStr.includes("Error") ||
            errStr.includes("6000") ||
            (e.logs &&
              e.logs.some(
                (l: string) =>
                  l.includes("InvalidOracleCallback") || l.includes("6000")
              )),
          "Should fail with oracle callback validation error"
        );
      }
    });

    it("fails with a non-existent analysis result account", async () => {
      const randomUser = web3.Keypair.generate();
      const pda = deriveAnalysisResultPda(randomUser.publicKey);

      try {
        await program.methods
          .callbackFromAgent("Some response")
          .accountsPartial({
            userPubkey: randomUser.publicKey,
            identity: identityPda,
          })
          .rpc();

        assert.fail("Should have thrown — account does not exist");
      } catch (e: any) {
        console.log(
          "  Expected error (no account):",
          e.message?.substring(0, 120)
        );
        assert.ok(e, "Should fail for non-existent analysis result");
      }
    });
  });

  // ============================================================
  // 4. GET ANALYSIS
  // ============================================================

  describe("4. Get Analysis", () => {
    it("retrieves the (empty) analysis for an analyzed user", async () => {
      const analysisResultPda = deriveAnalysisResultPda(
        userToAnalyze.publicKey
      );

      // Try .view() for a simulated read; fall back to direct account fetch
      try {
        const result = await program.methods
          .getAnalysis()
          .accountsPartial({
            userPubkey: userToAnalyze.publicKey,
          })
          .view();

        console.log("  Analysis via view():", result);
        assert.isString(result, "Result should be a string");
      } catch {
        // .view() may not be supported on all clusters
        console.log("  view() unavailable — reading account directly");
        const result = await program.account.analysisResult.fetch(
          analysisResultPda
        );
        console.log(
          "  Analysis:",
          result.analysis || "(empty — awaiting oracle callback)"
        );
        assert.isString(result.analysis, "Analysis should be a string");
      }
    });

    it("reads analysis result account data directly", async () => {
      const analysisResultPda = deriveAnalysisResultPda(
        userToAnalyze.publicKey
      );

      const account = await program.account.analysisResult.fetch(
        analysisResultPda
      );

      assert.isNotNull(account, "Account should not be null");
      assert.isString(account.analysis, "Analysis field should be a string");
      assert.isNumber(account.bump, "Bump should be a number");
      assert.isAbove(account.bump, 0, "Bump should be non-zero");

      console.log("  Account data:", {
        user: account.user.toBase58(),
        analysis: account.analysis || "(empty)",
        timestamp: account.timestamp.toString(),
        bump: account.bump,
      });
    });

    it("fails for a user that was never analyzed", async () => {
      const unknownUser = web3.Keypair.generate();
      const pda = deriveAnalysisResultPda(unknownUser.publicKey);

      try {
        await program.account.analysisResult.fetch(pda);
        assert.fail("Should have thrown — account does not exist");
      } catch (e: any) {
        console.log(
          "  Expected error (no analysis):",
          e.message?.substring(0, 100)
        );
        assert.ok(
          e.message.includes("Account does not exist") ||
            e.message.includes("Could not find") ||
            e.message.includes("does not exist"),
          "Should fail for non-existent analysis"
        );
      }
    });
  });

  // ============================================================
  // 5. PDA DERIVATION VERIFICATION
  // ============================================================

  describe("5. PDA Derivation Verification", () => {
    it("Agent PDA matches manual derivation", () => {
      const [computed] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("agent")],
        program.programId
      );
      assert.ok(computed.equals(agentPda), "Agent PDAs should match");
    });

    it("AnalysisResult PDA matches manual derivation", () => {
      const testUser = web3.Keypair.generate();
      const [manual] = web3.PublicKey.findProgramAddressSync(
        [Buffer.from("analysis"), testUser.publicKey.toBuffer()],
        program.programId
      );
      const helper = deriveAnalysisResultPda(testUser.publicKey);
      assert.ok(manual.equals(helper), "Analysis result PDAs should match");
    });

    it("Interaction PDA (oracle) matches manual derivation", () => {
      const testPayer = web3.Keypair.generate();
      const testCtx = web3.Keypair.generate();
      const [manual] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("interaction"),
          testPayer.publicKey.toBuffer(),
          testCtx.publicKey.toBuffer(),
        ],
        ORACLE_PROGRAM_ID
      );
      const helper = deriveInteractionPda(
        testPayer.publicKey,
        testCtx.publicKey
      );
      assert.ok(manual.equals(helper), "Interaction PDAs should match");
    });

    it("Context PDA (oracle) matches manual derivation", () => {
      const count = 42;
      const [manual] = web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("test-context"),
          new BN(count).toArrayLike(Buffer, "le", 4),
        ],
        ORACLE_PROGRAM_ID
      );
      const helper = deriveContextPda(count);
      assert.ok(manual.equals(helper), "Context PDAs should match");
    });
  });

  // ============================================================
  // 7. SCHEDULE (TukTuk Scheduler Integration)
  // ============================================================

  describe("7. Schedule", () => {
    it("schedules an analyze_user task via tuktuk", async () => {
      const tuktukProgram = await init(provider);
      await ensureQueueAuthorityInitialized(tuktukProgram as any);
      await ensureQueueFunding();

      // Get the next available task ID from the task queue bitmap
      const taskQueueAccount = await (
        tuktukProgram.account as any
      ).taskQueueV0.fetch(taskQueue);
      const taskId = nextAvailableTaskIds(
        taskQueueAccount.taskBitmap,
        1,
        false
      )[0];
      assert.isDefined(taskId, "No free task ID available in task queue");

      // Reload the agent to get the context address
      const agentAccount = await program.account.agent.fetch(agentPda);
      const ctxPda = agentAccount.context;

      const userToSchedule = web3.Keypair.generate();
      const analysisResultPda = deriveAnalysisResultPda(
        userToSchedule.publicKey
      );
      const interactionPda = deriveInteractionPda(
        provider.wallet.publicKey,
        ctxPda
      );

      const seed = freshScheduleSeed();

      console.log("  Task ID             :", taskId);
      console.log(
        "  User to schedule    :",
        userToSchedule.publicKey.toBase58()
      );
      console.log("  Analysis result PDA :", analysisResultPda.toBase58());
      console.log("  Interaction PDA     :", interactionPda.toBase58());
      console.log("  Queue authority     :", queueAuthority.toBase58());
      console.log("  Seed                :", seed);

      const tx = await program.methods
        .schedule(
          userToSchedule.publicKey,
          JSON.stringify({
            recentTransactions: [
              {
                type: "swap",
                amount: 20,
                from: "SOL",
                to: "USDC",
                protocol: "Jupiter",
              },
            ],
            balances: { SOL: 10.0, USDC: 500 },
            walletAge: "3 months",
          }),
          taskId,
          new anchor.BN(seed)
        )
        .accountsPartial({
          payer: provider.publicKey,
          interaction: interactionPda,
          agent: agentPda,
          contextAccount: ctxPda,
          oracleProgram: ORACLE_PROGRAM_ID,
          analysisResult: analysisResultPda,
          systemProgram: SystemProgram.programId,
          taskQueue,
          taskQueueAuthority: taskQueueAuthorityPda,
          task: taskKey(taskQueue, taskId)[0],
          queueAuthority,
          tuktukProgram: tuktukProgram.programId,
        })
        .rpc({ skipPreflight: true });

      await confirmTx(tx);
      console.log("  Schedule tx:", tx);

      // Verify the analysis result account was created (init_if_needed)
      const analysisResult = await program.account.analysisResult.fetch(
        analysisResultPda
      );
      assert.ok(
        analysisResult.user.equals(userToSchedule.publicKey),
        "Analysis result user should match the scheduled user"
      );
      assert.isNumber(analysisResult.bump, "bump should be a number");
      assert.isAbove(analysisResult.bump, 0, "bump should be non-zero");
      console.log("  Analysis result account:", {
        user: analysisResult.user.toBase58(),
        analysis:
          analysisResult.analysis || "(empty — awaiting oracle callback)",
        timestamp: analysisResult.timestamp.toString(),
        bump: analysisResult.bump,
      });

      // Verify the task was queued in tuktuk
      const queuedTask = await (
        tuktukProgram.account as any
      ).taskV0.fetchNullable(taskKey(taskQueue, taskId)[0]);
      assert.isNotNull(queuedTask, "Task should be queued by schedule");
      console.log("  Task queued:", queuedTask ? "YES" : "NO");
    });

    it("fails to schedule with an invalid task queue", async () => {
      const tuktukProgram = await init(provider);

      const agentAccount = await program.account.agent.fetch(agentPda);
      const ctxPda = agentAccount.context;

      const userToSchedule = web3.Keypair.generate();
      const analysisResultPda = deriveAnalysisResultPda(
        userToSchedule.publicKey
      );
      const interactionPda = deriveInteractionPda(
        provider.wallet.publicKey,
        ctxPda
      );

      const seed = freshScheduleSeed();
      const fakeTaskQueue = web3.Keypair.generate().publicKey;
      const fakeTaskQueueAuthority = taskQueueAuthorityKey(
        fakeTaskQueue,
        queueAuthority
      )[0];

      try {
        await program.methods
          .schedule(
            userToSchedule.publicKey,
            "Some user data",
            0,
            new anchor.BN(seed)
          )
          .accountsPartial({
            payer: provider.publicKey,
            interaction: interactionPda,
            agent: agentPda,
            contextAccount: ctxPda,
            oracleProgram: ORACLE_PROGRAM_ID,
            analysisResult: analysisResultPda,
            systemProgram: SystemProgram.programId,
            taskQueue: fakeTaskQueue,
            taskQueueAuthority: fakeTaskQueueAuthority,
            task: taskKey(fakeTaskQueue, 0)[0],
            queueAuthority,
            tuktukProgram: tuktukProgram.programId,
          })
          .rpc();

        assert.fail("Should have thrown — invalid task queue");
      } catch (e: any) {
        console.log(
          "  Expected error (invalid task queue):",
          e.message?.substring(0, 160)
        );
        assert.ok(e, "Should fail with an invalid task queue");
      }
    });
  });

  // ============================================================
  // 6. ACCOUNT DATA INTEGRITY
  // ============================================================

  describe("6. Account Data Integrity", () => {
    it("Agent account stores a valid context reference", async () => {
      const agentAccount = await program.account.agent.fetch(agentPda);

      // Context pubkey should not be the default (all-zeros)
      assert.ok(
        !agentAccount.context.equals(web3.PublicKey.default),
        "Context pubkey should not be zero"
      );

      // The referenced context account must exist and be owned by the oracle
      const ctxInfo = await provider.connection.getAccountInfo(
        agentAccount.context
      );
      assert.isNotNull(ctxInfo, "Context account should exist on-chain");
      assert.ok(
        ctxInfo!.owner.equals(ORACLE_PROGRAM_ID),
        "Context account should be owned by the oracle program"
      );
    });

    it("Agent account is owned by our program", async () => {
      const info = await provider.connection.getAccountInfo(agentPda);
      assert.isNotNull(info, "Agent account should exist");
      assert.ok(
        info!.owner.equals(program.programId),
        "Agent should be owned by our program"
      );
    });

    it("Oracle counter was incremented after initialize", async () => {
      const counterAccount = await oracleProgram.account.counter.fetch(
        counterPda
      );
      assert.isAbove(
        counterAccount.count,
        0,
        "Counter should be > 0 after context creation"
      );
      console.log("  Oracle counter:", counterAccount.count);
    });

    it("lists all AnalysisResult accounts created by our program", async () => {
      const accounts = await program.account.analysisResult.all();
      console.log(`  Found ${accounts.length} AnalysisResult account(s):`);

      for (const acct of accounts) {
        console.log(`    PDA      : ${acct.publicKey.toBase58()}`);
        console.log(`    User     : ${acct.account.user.toBase58()}`);
        console.log(
          `    Analysis : ${
            acct.account.analysis || "(empty — awaiting callback)"
          }`
        );
        console.log(`    Timestamp: ${acct.account.timestamp.toString()}`);
        console.log(`    Bump     : ${acct.account.bump}`);
        console.log();
      }

      assert.isAbove(
        accounts.length,
        0,
        "Should have at least one AnalysisResult"
      );
    });
  });
});
