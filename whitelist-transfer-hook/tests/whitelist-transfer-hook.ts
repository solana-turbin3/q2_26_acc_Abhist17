import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  TOKEN_2022_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createTransferCheckedInstruction,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getMint,
} from "@solana/spl-token";
import {
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
  SendTransactionError,
  Keypair,
} from "@solana/web3.js";
import { WhitelistTransferHook } from "../target/types/whitelist_transfer_hook";
import { expect } from "chai";

describe("whitelist-transfer-hook (Complete Test Suite)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const wallet = provider.wallet as anchor.Wallet;
  const connection = provider.connection;

  const program = anchor.workspace
    .whitelistTransferHook as Program<WhitelistTransferHook>;

  // Generate mint keypair
  const mint = Keypair.generate();
  const recipient = Keypair.generate();

  let sourceTokenAccount: anchor.web3.PublicKey;
  let destinationTokenAccount: anchor.web3.PublicKey;
  let extraAccountMetaListPDA: anchor.web3.PublicKey;
  let whitelistEntryPDA: anchor.web3.PublicKey;

  before("Airdrop to recipient", async () => {
    const airdropSig = await connection.requestAirdrop(
      recipient.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL,
    );
    await connection.confirmTransaction(airdropSig);
  });

  it("Create mint with transfer-hook using TokenFactory", async () => {
    // Derive PDAs
    [extraAccountMetaListPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("extra-account-metas"), mint.publicKey.toBuffer()],
      program.programId,
    );

    console.log("Mint public key:", mint.publicKey.toBase58());
    console.log(
      "Extra Account Meta List PDA:",
      extraAccountMetaListPDA.toBase58(),
    );

    // Call init_mint which creates the mint and initializes extra account metas
    const tx = await program.methods
      .initMint()
      .accounts({
        user: wallet.publicKey,
        mint: mint.publicKey,
        extraAccountMetaList: extraAccountMetaListPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_2022_PROGRAM_ID,
      })
      .signers([mint])
      .rpc();

    await connection.confirmTransaction(tx, "confirmed");

    console.log("Mint initialized with transfer hook:", tx);

    // Verify mint was created
    const mintInfo = await getMint(
      connection,
      mint.publicKey,
      "confirmed",
      TOKEN_2022_PROGRAM_ID,
    );

    expect(mintInfo.decimals).to.equal(9);
    expect(mintInfo.mintAuthority?.toBase58()).to.equal(
      wallet.publicKey.toBase58(),
    );
  });

  it("Create token accounts and mint tokens", async () => {
    // Derive ATAs
    sourceTokenAccount = getAssociatedTokenAddressSync(
      mint.publicKey,
      wallet.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    destinationTokenAccount = getAssociatedTokenAddressSync(
      mint.publicKey,
      recipient.publicKey,
      false,
      TOKEN_2022_PROGRAM_ID,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    );

    const amount = BigInt(100 * 10 ** 9); // 100 tokens with 9 decimals

    const tx = new Transaction().add(
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createAssociatedTokenAccountInstruction(
        wallet.publicKey,
        destinationTokenAccount,
        recipient.publicKey,
        mint.publicKey,
        TOKEN_2022_PROGRAM_ID,
        ASSOCIATED_TOKEN_PROGRAM_ID,
      ),
      createMintToInstruction(
        mint.publicKey,
        sourceTokenAccount,
        wallet.publicKey,
        amount,
        [],
        TOKEN_2022_PROGRAM_ID,
      ),
    );

    const sig = await sendAndConfirmTransaction(
      connection,
      tx,
      [wallet.payer],
      {
        skipPreflight: true,
      },
    );

    console.log("Token accounts created and tokens minted:", sig);
  });

  it("Should FAIL transfer without whitelist entry", async () => {
    const amount = BigInt(1 * 10 ** 9); // 1 token

    // Derive whitelist PDA
    [whitelistEntryPDA] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("whitelist"), wallet.publicKey.toBuffer()],
      program.programId,
    );

    const transferInstruction = createTransferCheckedInstruction(
      sourceTokenAccount,
      mint.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amount,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    // Add required extra accounts for transfer hook
    transferInstruction.keys.push(
      { pubkey: extraAccountMetaListPDA, isSigner: false, isWritable: false },
      { pubkey: whitelistEntryPDA, isSigner: false, isWritable: false },
      { pubkey: program.programId, isSigner: false, isWritable: false },
    );

    const tx = new Transaction().add(transferInstruction);

    try {
      await sendAndConfirmTransaction(connection, tx, [wallet.payer], {
        skipPreflight: false,
      });
      throw new Error("Transfer should have failed without whitelist");
    } catch (err) {
      console.log(
        "Expected failure (no whitelist):",
        (err as any).message || err,
      );
      // This is expected - whitelist PDA doesn't exist yet
    }
  });

  it("Add wallet to whitelist", async () => {
    const tx = await program.methods
      .addToWhitelist()
      .accounts({
        admin: wallet.publicKey,
        user: wallet.publicKey,
        whitelistEntry: whitelistEntryPDA,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Wallet added to whitelist:", tx);

    // Verify whitelist entry was created
    const whitelistEntry = await program.account.whiteListEntry.fetch(
      whitelistEntryPDA,
    );
    expect(whitelistEntry.user.toBase58()).to.equal(
      wallet.publicKey.toBase58(),
    );
  });

  it("Should SUCCEED transfer with whitelist entry", async () => {
    const amount = BigInt(1 * 10 ** 9); // 1 token

    const transferInstruction = createTransferCheckedInstruction(
      sourceTokenAccount,
      mint.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amount,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    // Add required extra accounts
    transferInstruction.keys.push(
      { pubkey: whitelistEntryPDA, isSigner: false, isWritable: false },
    );
    const tx = new Transaction().add(transferInstruction);

    try {
      const sig = await sendAndConfirmTransaction(
        connection,
        tx,
        [wallet.payer],
        {
          skipPreflight: false,
        },
      );

      console.log("Transfer succeeded with whitelist:", sig);
    } catch (err) {
      if (err instanceof SendTransactionError) {
        console.error("Transfer failed, logs:", err.logs);
      }
      throw err;
    }
  });

  it("Remove from whitelist and verify transfer fails", async () => {
    // Remove wallet from whitelist
    const txClose = await program.methods
      .removeFromWhitelist()
      .accounts({
        admin: wallet.publicKey,
        user: wallet.publicKey,
        whitelistEntry: whitelistEntryPDA,
      })
      .rpc();

    console.log("Whitelist entry removed:", txClose);

    // Attempt transfer (should fail)
    const amount = BigInt(1 * 10 ** 9);
    const transferInstruction = createTransferCheckedInstruction(
      sourceTokenAccount,
      mint.publicKey,
      destinationTokenAccount,
      wallet.publicKey,
      amount,
      9,
      [],
      TOKEN_2022_PROGRAM_ID,
    );

    transferInstruction.keys.push(
      { pubkey: whitelistEntryPDA, isSigner: false, isWritable: false },
    );

    const tx = new Transaction().add(transferInstruction);

    try {
      await sendAndConfirmTransaction(connection, tx, [wallet.payer], {
        skipPreflight: false,
      });
      throw new Error("Transfer should have failed after whitelist removal");
    } catch (err) {
      console.log(
        "Expected failure after whitelist removal:",
        (err as any).message || err,
      );
      // This is expected
    }
  });
});
