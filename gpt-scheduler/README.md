# GPT Scheduler

A Solana program that schedules automated GPT queries using [MagicBlock's Solana GPT Oracle](https://github.com/magicblock-labs/super-smart-contracts) and [TukTuk](https://tuktuk.fun) crank network.

Your program sends queries to the GPT Oracle, and TukTuk's permissionless crankers execute scheduled queries automatically - no manual intervention needed.


## Architecture

**GPT Scheduler PDA** (`seeds = [b"gpt_scheduler"]`)
- Stores the LLM context reference
- Stores query and last response
- Tracks query count

**Queue Authority PDA** (`seeds = [b"queue_authority"]`)
- Registered with TukTuk to schedule tasks
- Signs task queue operations

## Devnet Deployment

| | Address |
|---|---------|
| Program ID | `77gWVyXhRufiXYer1jF47dCySoScpobpDpZNE3FbDfT7` |
| Task Queue | `EyBYdsGtjkphgmjKiyqe2DKEDP1v6okAMJAXeaqjWmAE` |
| GPT Oracle | `LLMrieZMpbJFwN52WgmBNMxYojrpRVYXdC1RCweEbab` |

## Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) 1.18+
- [Anchor](https://www.anchor-lang.com/docs/installation) 0.30+
- [Node.js](https://nodejs.org/) 18+

## Setup

### 1. Clone & Install

```bash
git clone https://github.com/your-username/gpt-scheduler.git
cd gpt-scheduler
yarn install
```

### 2. Clone GPT Oracle (dependency)

```bash
cd ..
git clone https://github.com/magicblock-labs/super-smart-contracts.git
cd gpt-scheduler
```

### 3. Configure Solana

```bash
solana config set --url devnet
solana-keygen new -o ~/.config/solana/id.json
solana airdrop 2
```

### 4. Build & Deploy

```bash
anchor build
solana address -k target/deploy/gpt_scheduler-keypair.json
```

Update program ID in `lib.rs`, `Anchor.toml`, and `tests/constants.ts`, then:

```bash
anchor deploy --provider.cluster devnet
```

### 5. Install TukTuk CLI

```bash
cargo install tuktuk-cli
```

### 6. Create Task Queue

```bash
tuktuk -u https://api.devnet.solana.com -w ~/.config/solana/id.json task-queue create \
  --name "gpt-scheduler" \
  --capacity 100 \
  --min-crank-reward 1000000 \
  --funding-amount 500000000 \
  --stale-task-age 60400
```

Save the task queue address.

### 7. Get Queue Authority PDA

```bash
npx ts-node -e "
const { PublicKey } = require('@solana/web3.js');
const PROGRAM_ID = new PublicKey('YOUR_PROGRAM_ID');
const [qa] = PublicKey.findProgramAddressSync([Buffer.from('queue_authority')], PROGRAM_ID);
console.log('Queue Authority:', qa.toBase58());
"
```

### 8. Register Queue Authority

```bash
tuktuk -u https://api.devnet.solana.com -w ~/.config/solana/id.json task-queue add-queue-authority \
  --task-queue-id <TASK_QUEUE_ID> \
  --queue-authority <QUEUE_AUTHORITY_PDA>
```

### 9. Update Constants

Edit `tests/constants.ts`:

```typescript
export const PROGRAM_ID = new PublicKey("YOUR_PROGRAM_ID");
export const TASK_QUEUE = new PublicKey("YOUR_TASK_QUEUE");
```

## Testing

```bash
npm run test:devnet
```

Expected output:

```
  gpt-scheduler
    ✔ 1. Initialize GPT Scheduler (skip if exists)
    ✔ 2. Query GPT
    ✔ 3. Schedule automated GPT query via TukTuk
    ✔ 4. Check scheduler state

  4 passing
```

## Check Response

After a query, the Oracle processes it off-chain and calls back. Check the response:

```bash
npx ts-node scripts/check-response.ts
```

## Flow

```
SETUP:
  initialize()
    └─> Creates LLM context via Oracle CPI
    └─> Stores context reference in GptScheduler

MANUAL QUERY:
  query_gpt()
    └─> Sends query to Oracle via interact_with_llm CPI
    └─> Oracle processes off-chain
    └─> callback_from_gpt() receives response

AUTOMATED QUERY:
  schedule_query()
    └─> CPI to TukTuk queue_task_v0
    └─> TukTuk cranker executes query_gpt
    └─> Oracle processes and calls back
```

## Monitoring

```bash
# Check task queue
tuktuk -u https://api.devnet.solana.com task-queue show --task-queue <ADDRESS>

# Check specific task
tuktuk -u https://api.devnet.solana.com task show --task <TASK_ADDRESS>
```

## License

MIT