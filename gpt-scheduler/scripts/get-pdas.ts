import { PublicKey } from "@solana/web3.js";

const PROGRAM_ID = new PublicKey("77gWVyXhRufiXYer1jF47dCySoScpobpDpZNE3FbDfT7");

const [gptScheduler, gptSchedulerBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("gpt_scheduler")],
  PROGRAM_ID
);

const [queueAuthority, queueAuthorityBump] = PublicKey.findProgramAddressSync(
  [Buffer.from("queue_authority")],
  PROGRAM_ID
);

console.log("===========================================");
console.log("Program ID:       ", PROGRAM_ID.toBase58());
console.log("GPT Scheduler:    ", gptScheduler.toBase58());
console.log("Queue Authority:  ", queueAuthority.toBase58());
console.log("===========================================");
console.log("\nSave the Queue Authority for TukTuk registration!");