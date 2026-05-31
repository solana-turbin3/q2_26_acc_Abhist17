import { PublicKey } from "@solana/web3.js";

// Replace these after deploying
export const PROGRAM_ID = new PublicKey("77gWVyXhRufiXYer1jF47dCySoScpobpDpZNE3FbDfT7");
export const TASK_QUEUE = new PublicKey("EyBYdsGtjkphgmjKiyqe2DKEDP1v6okAMJAXeaqjWmAE");

// PDAs
export const [GPT_SCHEDULER] = PublicKey.findProgramAddressSync(
  [Buffer.from("gpt_scheduler")],
  PROGRAM_ID
);

export const [QUEUE_AUTHORITY] = PublicKey.findProgramAddressSync(
  [Buffer.from("queue_authority")],
  PROGRAM_ID
);

// Oracle constants
export const ORACLE_PROGRAM_ID = new PublicKey(
  "LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab"
);