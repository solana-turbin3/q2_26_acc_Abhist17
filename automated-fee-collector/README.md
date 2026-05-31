# Automated Fee Collector

A Token-2022 transfer fee token on Solana with automated fee collection powered by [TukTuk](https://tuktuk.fun).

Users transfer tokens and a 5% fee is automatically withheld on the recipient's account. TukTuk's permissionless cranker network periodically harvests these fees and deposits them into a treasury - no manual intervention needed.

## How It Works

```
Alice sends 100 tokens to Bob
  └─> Bob receives 95 tokens
  └─> 5 tokens withheld as fee

TukTuk cranker executes scheduled collection
  └─> Fees harvested from all accounts
  └─> Treasury receives collected fees
```

The program uses two PDAs:
- **Fee Authority** (`seeds = [b"fee_authority"]`) - Signs fee withdrawal from mint to treasury
- **Queue Authority** (`seeds = [b"queue_authority"]`) - Registered with TukTuk to schedule tasks

## Devnet Deployment

| | Address |
|---|---------|
| Program ID | `8Axod2sBdMn6b7Nwicbtujwj6Gsv3wfBh6NjDtPPVKAh` |
| Task Queue | `BPWhNdmPi5T6uhnH3DHoAakAZ3V4ciicQtj6PF8eoL5F` |

## Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) 1.18+
- [Anchor](https://www.anchor-lang.com/docs/installation) 0.30+
- [Node.js](https://nodejs.org/) 18+

## Setup

### 1. Clone & Install

```bash
git clone https://github.com/your-username/automated-fee-collector.git
cd automated-fee-collector
yarn install
```

### 2. Configure Solana

```bash
solana config set --url devnet
solana-keygen new -o ~/.config/solana/id.json
solana airdrop 2
```

### 3. Build & Deploy

```bash
anchor build
solana address -k target/deploy/automated_fee_collector-keypair.json
```

Update `Anchor.toml` and `lib.rs` with your program ID, then:

```bash
anchor deploy --provider.cluster devnet
```

### 4. Install TukTuk CLI

```bash
cargo install tuktuk-cli
```

### 5. Create Task Queue

```bash
tuktuk -u https://api.devnet.solana.com -w ~/.config/solana/id.json task-queue create \
  --name "fee-collector" \
  --capacity 100 \
  --min-crank-reward 1000000 \
  --funding-amount 500000000
```

Save the task queue address from the output.

### 6. Get PDAs

```bash
npx ts-node scripts/get-pdas.ts
```

Save the Queue Authority PDA.

### 7. Register Queue Authority

```bash
tuktuk -u https://api.devnet.solana.com -w ~/.config/solana/id.json task-queue add-queue-authority \
  --task-queue <TASK_QUEUE_ADDRESS> \
  --queue-authority <QUEUE_AUTHORITY_PDA>
```

### 8. Update Constants

Edit `tests/constants.ts` with your program ID and task queue address.

## Test

```bash
yarn test:devnet
```

Expected output:

```
  automated-fee-collector
    ✓ 1. Initialize mint with transfer fee
    ✓ 2. Initialize treasury
    ✓ 3. Mint tokens to Alice
    ✓ 4. Mint tokens to Bob
    ✓ 5. Alice → Bob transfer (fees accumulate)
    ✓ 6. Bob → Alice transfer (more fees)
    ✓ 7. Collect fees to treasury
    ✓ 8. Schedule automated collection via TukTuk

  8 passing
```

## Flow

```
INIT:
  init_mint()     -> Creates Token-2022 mint with 5% transfer fee
  init_treasury() -> Creates treasury account for collected fees

USAGE:
  mint_to()       -> Mint tokens to users
  transfer()      -> Transfer tokens (fee withheld on recipient)

COLLECTION:
  manual_collect()  -> Harvest fees to treasury (anyone can call)
  schedule()        -> Queue automated collection via TukTuk
```

## Monitoring

```bash
# Check task queue
tuktuk -u https://api.devnet.solana.com task-queue show --task-queue <ADDRESS>

# Check task status  
tuktuk -u https://api.devnet.solana.com task show --task <ADDRESS>
```

## License

MIT