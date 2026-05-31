import { PublicKey } from "@solana/web3.js";

// Replace with YOUR program ID from Step 1
const PROGRAM_ID = new PublicKey("8Axod2sBdMn6b7Nwicbtujwj6Gsv3wfBh6NjDtPPVKAh");

const [feeAuthority, feeAuthorityBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("fee_authority")],
  PROGRAM_ID
);

const [queueAuthority, queueAuthorityBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("queue_authority")],
  PROGRAM_ID
);

console.log("===========================================");
console.log("Program ID:       ", PROGRAM_ID.toBase58());
console.log("Fee Authority:    ", feeAuthority.toBase58());
console.log("Queue Authority:  ", queueAuthority.toBase58());
console.log("===========================================");
console.log("\nSave the Queue Authority address - you need it for Step 5!");