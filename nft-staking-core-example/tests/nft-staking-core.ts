import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NftStakingCore } from "../target/types/nft_staking_core";
import { SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { MPL_CORE_PROGRAM_ID } from "@metaplex-foundation/mpl-core";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

const POINTS_PER_STAKED_NFT_PER_DAY = 10_000_000;
const FREEZE_PERIOD_IN_DAYS = 1;
const MPL_CORE_ID = new anchor.web3.PublicKey(MPL_CORE_PROGRAM_ID);

describe("nft-staking-core", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.nftStakingCore as Program<NftStakingCore>;

  const collectionKeypair = anchor.web3.Keypair.generate();
  const updateAuthority = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("update_authority"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];
  const config = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("config"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];
  const rewardsMint = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("rewards"), config.toBuffer()],
    program.programId
  )[0];
  const oracleAccount = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("oracle"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];
  const crankVault = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("crank_vault"), collectionKeypair.publicKey.toBuffer()],
    program.programId
  )[0];

  const nftForClaim = anchor.web3.Keypair.generate();
  const nftForBurn = anchor.web3.Keypair.generate();
  const nftForUnstake = anchor.web3.Keypair.generate();
  const nftForTransfer = anchor.web3.Keypair.generate();
  const newOwner = anchor.web3.Keypair.generate();
  let userRewardsAta: anchor.web3.PublicKey;

  // Track the last known timestamp to ensure we always go forward
  let lastKnownTimestamp: number = 0;

  async function getCurrentTimestamp(): Promise<number> {
    const slot = await provider.connection.getSlot();
    const ts = (await provider.connection.getBlockTime(slot))!;
    // Always use the max of what we know and what blockchain reports
    lastKnownTimestamp = Math.max(lastKnownTimestamp, ts);
    return lastKnownTimestamp;
  }

  async function timeTravel(seconds: number): Promise<void> {
    // Get the latest known timestamp
    await getCurrentTimestamp();
    
    // Calculate target ensuring we go forward
    const targetTs = lastKnownTimestamp + seconds + 120; // Add buffer
    lastKnownTimestamp = targetTs; // Update our tracking

    const res = await fetch(provider.connection.rpcEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: 1,
        method: "surfnet_timeTravel",
        params: [{ absoluteTimestamp: targetTs * 1000 }],
      }),
    });
    const result = (await res.json()) as { error?: any };
    if (result.error)
      throw new Error(`Time travel failed: ${JSON.stringify(result.error)}`);
    await new Promise((r) => setTimeout(r, 1000));
  }

  // Ensure we're in the allowed window (9 AM - 5 PM UTC)
  async function ensureInAllowedWindow(): Promise<void> {
    const currentTs = await getCurrentTimestamp();
    const secSinceMidnight = ((currentTs % 86400) + 86400) % 86400;
    const currentHour = Math.floor(secSinceMidnight / 3600);

    console.log(`Current hour (UTC): ${currentHour}`);

    if (currentHour >= 9 && currentHour < 17) {
      console.log("Already in allowed window");
      return;
    }

    // Calculate seconds to travel to reach 10 AM
    let secsToTravel: number;
    if (currentHour < 9) {
      // Before 9 AM, travel to 10 AM same day
      secsToTravel = (10 * 3600) - secSinceMidnight;
    } else {
      // After 5 PM, travel to 10 AM next day
      secsToTravel = (86400 - secSinceMidnight) + (10 * 3600);
    }

    console.log(`Traveling ${secsToTravel} seconds to reach 10 AM UTC`);
    await timeTravel(secsToTravel);
  }

  // Ensure we're outside the allowed window (before 9 AM or after 5 PM UTC)
  async function ensureOutsideAllowedWindow(): Promise<void> {
    const currentTs = await getCurrentTimestamp();
    const secSinceMidnight = ((currentTs % 86400) + 86400) % 86400;
    const currentHour = Math.floor(secSinceMidnight / 3600);

    console.log(`Current hour (UTC): ${currentHour}`);

    if (currentHour < 9 || currentHour >= 17) {
      console.log("Already outside allowed window");
      return;
    }

    // Calculate seconds to travel to reach 11 PM (23:00)
    const secsToTravel = (23 * 3600) - secSinceMidnight;

    console.log(`Traveling ${secsToTravel} seconds to reach 11 PM UTC`);
    await timeTravel(secsToTravel);
  }

  async function mintNft(kp: anchor.web3.Keypair, name: string) {
    await program.methods
      .mintNft(name, "https://example.com")
      .accountsPartial({
        user: provider.wallet.publicKey,
        nft: kp.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_ID,
      })
      .signers([kp])
      .rpc();
  }

  async function stakeNft(kp: anchor.web3.Keypair) {
    await program.methods
      .stake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        nft: kp.publicKey,
        collection: collectionKeypair.publicKey,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_ID,
      })
      .rpc();
  }

  it("Create collection, config, mint & stake NFTs", async () => {
    // Initialize our timestamp tracking
    await getCurrentTimestamp();

    await program.methods
      .createCollection("Collection", "https://example.com")
      .accountsPartial({
        payer: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        systemProgram: SystemProgram.programId,
        mplCoreProgram: MPL_CORE_ID,
      })
      .signers([collectionKeypair])
      .rpc();

    await program.methods
      .initializeConfig(POINTS_PER_STAKED_NFT_PER_DAY, FREEZE_PERIOD_IN_DAYS)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    userRewardsAta = getAssociatedTokenAddressSync(
      rewardsMint,
      provider.wallet.publicKey,
      false,
      TOKEN_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID
    );

    await mintNft(nftForClaim, "Claim NFT");
    await mintNft(nftForBurn, "Burn NFT");
    await mintNft(nftForUnstake, "Unstake NFT");
    await mintNft(nftForTransfer, "Transfer NFT");

    await stakeNft(nftForClaim);
    await stakeNft(nftForBurn);
    await stakeNft(nftForUnstake);
    console.log("Setup complete: 3 NFTs staked");
  });

  it("Advance time 2 days and claim rewards (NFT stays staked)", async () => {
    await timeTravel(2 * 86400);

    await program.methods
      .claimRewards()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftForClaim.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
    const bal = await provider.connection.getTokenAccountBalance(userRewardsAta);
    console.log("Claimed rewards:", bal.value.uiAmount);
  });

  it("Burn staked NFT for 10x bonus rewards", async () => {
    await program.methods
      .burnStakedNft()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftForBurn.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
    const bal = await provider.connection.getTokenAccountBalance(userRewardsAta);
    console.log("Balance after burn:", bal.value.uiAmount);
    const nftAccount = await provider.connection.getAccountInfo(nftForBurn.publicKey);
    console.log("NFT destroyed:", nftAccount === null);
  });

  it("Unstake NFT with rewards", async () => {
    await timeTravel(2 * 86400);
    await program.methods
      .unstake()
      .accountsPartial({
        user: provider.wallet.publicKey,
        updateAuthority,
        config,
        rewardsMint,
        userRewardsAta,
        nft: nftForUnstake.publicKey,
        collection: collectionKeypair.publicKey,
        mplCoreProgram: MPL_CORE_ID,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      })
      .rpc();
    const bal = await provider.connection.getTokenAccountBalance(userRewardsAta);
    console.log("Balance after unstake:", bal.value.uiAmount);
  });

  it("Initialize oracle and fund crank vault", async () => {
    await program.methods
      .initializeOracle()
      .accountsPartial({
        admin: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        oracleAccount,
        crankVault,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    await program.methods
      .fundCrankVault(new anchor.BN(0.1 * LAMPORTS_PER_SOL))
      .accountsPartial({
        funder: provider.wallet.publicKey,
        collection: collectionKeypair.publicKey,
        crankVault,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    const state = await program.account.oracleState.fetch(oracleAccount);
    console.log("Oracle initialized, transfer_allowed:", state.transferAllowed);
  });

  it("Transfer NFT during allowed window (9AM-5PM UTC)", async () => {
    // Ensure we're in the allowed window (9 AM - 5 PM UTC)
    await ensureInAllowedWindow();

    // Check oracle state and crank if needed
    let state = await program.account.oracleState.fetch(oracleAccount);
    console.log("Oracle transfer_allowed before crank:", state.transferAllowed);
    
    if (!state.transferAllowed) {
      try {
        await program.methods
          .crankOracle()
          .accountsPartial({
            cranker: provider.wallet.publicKey,
            collection: collectionKeypair.publicKey,
            oracleAccount,
            crankVault,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        state = await program.account.oracleState.fetch(oracleAccount);
        console.log("Oracle transfer_allowed after crank:", state.transferAllowed);
      } catch (e) {
        console.log("Crank failed (may already be in correct state):", e);
      }
    }

    await program.methods
      .transferNft()
      .accountsPartial({
        owner: provider.wallet.publicKey,
        newOwner: newOwner.publicKey,
        nft: nftForTransfer.publicKey,
        collection: collectionKeypair.publicKey,
        updateAuthority,
        oracleAccount,
        mplCoreProgram: MPL_CORE_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts([
        { pubkey: oracleAccount, isSigner: false, isWritable: false },
      ])
      .rpc();
    console.log("NFT transferred during allowed window");
  });

  it("Transfer fails outside window (11PM UTC)", async () => {
    const extraNft = anchor.web3.Keypair.generate();
    await mintNft(extraNft, "Blocked NFT");

    // Ensure we're outside the allowed window
    await ensureOutsideAllowedWindow();

    // Check oracle state and crank if needed
    let state = await program.account.oracleState.fetch(oracleAccount);
    console.log("Oracle transfer_allowed before crank:", state.transferAllowed);
    
    if (state.transferAllowed) {
      try {
        await program.methods
          .crankOracle()
          .accountsPartial({
            cranker: provider.wallet.publicKey,
            collection: collectionKeypair.publicKey,
            oracleAccount,
            crankVault,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        state = await program.account.oracleState.fetch(oracleAccount);
        console.log("Oracle transfer_allowed after crank:", state.transferAllowed);
      } catch (e) {
        console.log("Crank failed (may already be in correct state):", e);
      }
    }

    try {
      await program.methods
        .transferNft()
        .accountsPartial({
          owner: provider.wallet.publicKey,
          newOwner: newOwner.publicKey,
          nft: extraNft.publicKey,
          collection: collectionKeypair.publicKey,
          updateAuthority,
          oracleAccount,
          mplCoreProgram: MPL_CORE_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts([
          { pubkey: oracleAccount, isSigner: false, isWritable: false },
        ])
        .rpc();
      throw new Error("Should have failed");
    } catch (err: any) {
      if (err.message === "Should have failed") throw err;
      console.log("Transfer correctly blocked outside window");
    }
  });
});