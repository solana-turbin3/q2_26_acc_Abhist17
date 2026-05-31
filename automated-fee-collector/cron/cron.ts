import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { createCronJob, 
    cronJobTransactionKey, 
    getCronJobForName, 
    init as initCron 
} from "@helium/cron-sdk";
import {
  compileTransaction,
  init,
  taskQueueAuthorityKey
} from "@helium/tuktuk-sdk";
import { 
    LAMPORTS_PER_SOL, 
    SystemProgram, 
    TransactionInstruction 
} from "@solana/web3.js";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import { sendInstructions } from "@helium/spl-utils";
import { TuktukCounter } from "../target/types/tuktuk_counter";


const counterProgram = anchor.workspace.tuktukCounter as Program<TuktukCounter>;

const counter = anchor.web3.PublicKey.findProgramAddressSync([Buffer.from("counter")], counterProgram.programId)[0];

async function main() {
    const argv = await yargs(hideBin(process.argv))
        .options({
        cronName: {
            type: "string",
            description: "The name of the cron job to create",
            demandOption: true,
        },
        queueName: {
            type: "string",
            description: "The name of the task queue to use",
            demandOption: true,
        },
        walletPath: {
            type: "string",
            description: "Path to the wallet keypair",
            demandOption: true,
        },
        rpcUrl: {
            type: "string",
            description: "Your Solana RPC URL",
            demandOption: true,
        },
        message: {
            type: "string",
            description: "Message to write in the memo",
            default: "This will create a cron job for the Turbin3 Accel tuktuk counter program!",
        },
        fundingAmount: {
            type: "number",
            description: "Amount of SOL to fund the cron job with (in lamports)",
            default: 0.01 * LAMPORTS_PER_SOL,
        },
        })
        .help()
        .alias("help", "h").argv;

    // Setup connection and provider
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const wallet = provider.wallet as anchor.Wallet;

    console.log("Using wallet:", wallet.publicKey.toBase58());
    console.log("RPC URL:", argv.rpcUrl);
    console.log("Message:", argv.message);

    // Initialize TukTuk program
    const program = await init(provider);
    const cronProgram = await initCron(provider);
    const taskQueue = new anchor.web3.PublicKey("CMreFdKxT5oeZhiX8nWTGz9PtXM1AMYTh6dGR2UzdtrA");

    // Check if task_queue_authority exists for this wallet, if not create it
    const taskQueueAuthorityPda = taskQueueAuthorityKey(taskQueue, wallet.publicKey)[0];
    const taskQueueAuthorityInfo = await provider.connection.getAccountInfo(taskQueueAuthorityPda);
    
    if (!taskQueueAuthorityInfo) {
        console.log("Initializing task queue authority for wallet...");
        await program.methods
            .addQueueAuthorityV0()
            .accounts({
                payer: wallet.publicKey,
                queueAuthority: wallet.publicKey,
                taskQueue,
            })
            .rpc({ skipPreflight: true });
        console.log("Task queue authority initialized!");
    } else {
        console.log("Task queue authority already exists");
    }

    // Check if cron job already exists
    let cronJob = await getCronJobForName(cronProgram, argv.cronName);
    console.log("Cron Job:", cronJob);
    if (!cronJob) {
        console.log("Creating new cron job...");
        const { pubkeys: { cronJob: cronJobPubkey } } = await (await createCronJob(cronProgram, {
            tuktukProgram: program,
            taskQueue,
            args: {
                name: argv.cronName,
                schedule: "0 * * * * *", // Run every minute
                // How many "free" tasks to allocate to this cron job per transaction (whitout paying crank fee)
                // The increment transaction doesn't need to schedule more transactions, so we set this to 0
                freeTasksPerTransaction: 0,
                // We just have one transaction to queue for each cron job, so we set this to 1
                numTasksPerQueueCall: 1,
            }
        }))
        .rpcAndKeys({ skipPreflight: false });
        cronJob = cronJobPubkey;
        console.log("Funding cron job with", argv.fundingAmount / LAMPORTS_PER_SOL, "SOL");
        await sendInstructions(provider, [
        SystemProgram.transfer({
            fromPubkey: provider.publicKey,
            toPubkey: cronJob,
            lamports: argv.fundingAmount,
        }),
        ]);
        // Create a simple increment instruction
        const counterInstruction = new TransactionInstruction({
            keys: [
                { pubkey: counter, isSigner: false, isWritable: true }
            ],
            data: counterProgram.coder.instruction.encode("increment", {}),
            programId: counterProgram.programId,
        });

        // Compile the instruction
        console.log("Compiling instructions...");
        const { transaction, remainingAccounts } = compileTransaction(
            [counterInstruction],
            []
        );

        // Adding increment to the cron job
        await cronProgram.methods
        .addCronTransactionV0({
            index: 0,
            transactionSource: {
            compiledV0: [transaction],
            },
        })
        .accounts({
            payer: provider.publicKey,
            cronJob,
            cronJobTransaction: cronJobTransactionKey(cronJob, 0)[0],
        })
        .remainingAccounts(remainingAccounts)
        .rpc({ skipPreflight: true });
        console.log(`Cron job created!`);
    } else {
        console.log("Cron job already exists");
    }

    console.log("Cron job address:", cronJob.toBase58());
    console.log(`\nYour Counter Increment Instruction will be posted every minute. Watch for transactions on task queue ${taskQueue.toBase58()}. To stop the cron job, use the tuktuk-cli:`);
    console.log(`tuktuk -u ${argv.rpcUrl} -w ${argv.walletPath} cron-transaction close --cron-name ${argv.cronName} --id 0`);
    console.log(`tuktuk -u ${argv.rpcUrl} -w ${argv.walletPath} cron close --cron-name ${argv.cronName}`);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  }); 