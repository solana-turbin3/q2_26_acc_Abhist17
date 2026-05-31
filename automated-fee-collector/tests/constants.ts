import { PublicKey } from "@solana/web3.js";

// ========== FILL THESE IN ==========
export const PROGRAM_ID = new PublicKey("8Axod2sBdMn6b7Nwicbtujwj6Gsv3wfBh6NjDtPPVKAh");
export const TASK_QUEUE = new PublicKey("BPWhNdmPi5T6uhnH3DHoAakAZ3V4ciicQtj6PF8eoL5F");
// ===================================

// PDAs derived from program ID
export const [FEE_AUTHORITY] = PublicKey.findProgramAddressSync(
  [Buffer.from("fee_authority")],
  PROGRAM_ID
);

export const [QUEUE_AUTHORITY] = PublicKey.findProgramAddressSync(
  [Buffer.from("queue_authority")],
  PROGRAM_ID
);