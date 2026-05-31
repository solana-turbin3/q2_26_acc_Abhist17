
import * as anchor from "@coral-xyz/anchor";
import { init as initTuktuk, taskQueueAuthorityKey } from "@helium/tuktuk-sdk";
import { PublicKey } from "@solana/web3.js";

type ParsedArgs = {
    taskQueue: string;
    runOnce: boolean;
    staleSeconds: number;
    batchSize: number;
    weekSeconds: number;
};

const DEFAULT_TASK_QUEUE = "CJv1jLvFSLsV7X1UGq6bHr6XHacbJAfq7Tio8iqpEK6b";
const ONE_WEEK_SECONDS = 7 * 24 * 60 * 60;

function parseArgs(): ParsedArgs {
    const args = process.argv.slice(2);
    const map = new Map<string, string>();

    for (let index = 0; index < args.length; index += 1) {
        const token = args[index];
        if (!token.startsWith("--")) continue;
        const key = token.slice(2);
        const next = args[index + 1];
        if (!next || next.startsWith("--")) {
            map.set(key, "true");
            continue;
        }
        map.set(key, next);
        index += 1;
    }

    const taskQueueArg = map.get("taskQueue") ?? map.get("queue") ?? map.get("queueName") ?? DEFAULT_TASK_QUEUE;

    return {
        taskQueue: taskQueueArg,
        runOnce: (map.get("runOnce") ?? "false") === "true",
        staleSeconds: Number(map.get("staleSeconds") ?? ONE_WEEK_SECONDS),
        batchSize: Number(map.get("batchSize") ?? 100),
        weekSeconds: Number(map.get("weekSeconds") ?? ONE_WEEK_SECONDS),
    };
}

function nowUnix(): number {
    return Math.floor(Date.now() / 1000);
}

function getTriggerTimestamp(taskAccount: any): number {
    const trigger = taskAccount?.trigger;
    if (!trigger || typeof trigger !== "object") {
        return 0;
    }

    if ("timestamp" in trigger) {
        const raw = (trigger as { timestamp: any }).timestamp;
        if (typeof raw === "number") return raw;
        if (raw?.toNumber) return raw.toNumber();
        if (raw?.toString) return Number(raw.toString());
    }

    if ("now" in trigger) {
        return 0;
    }

    return 0;
}

async function cleanupTaskQueue(args: ParsedArgs): Promise<void> {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);
    const wallet = provider.wallet as anchor.Wallet;
    const tuktukProgram = await initTuktuk(provider);
    const taskQueue = new PublicKey(args.taskQueue);
    const queueAuthority = wallet.publicKey;
    const startedAt = new Date().toISOString();

    console.log("---------------- TukTuk Weekly Cleanup Run ----------------");
    console.log("Started At (UTC):", startedAt);
    console.log("Wallet:", wallet.publicKey.toBase58());
    console.log("Task Queue:", taskQueue.toBase58());
    console.log("Queue Authority:", queueAuthority.toBase58());

    const taskQueueAuthority = taskQueueAuthorityKey(taskQueue, queueAuthority)[0];
    const taskQueueAuthorityAccount = await (tuktukProgram.account as any).taskQueueAuthorityV0.fetchNullable(taskQueueAuthority);
    if (!taskQueueAuthorityAccount) {
        throw new Error(
            `taskQueueAuthority ${taskQueueAuthority.toBase58()} not found for queue ${taskQueue.toBase58()} and authority ${queueAuthority.toBase58()}. ` +
                "Add this authority to the queue first (or run with the correct authority wallet)."
        );
    }

    const allQueueTasks = await (tuktukProgram.account as any).taskV0.all([
        {
            memcmp: {
                offset: 8,
                bytes: taskQueue.toBase58(),
            },
        },
    ]);

    const currentTs = nowUnix();
    const staleBefore = currentTs - args.staleSeconds;
    console.log("Current Unix Time:", currentTs);
    console.log("Stale Threshold (trigger <=):", staleBefore);

    const completedOrStaleTasks = allQueueTasks.filter((task: any) => {
        const triggerTs = getTriggerTimestamp(task.account);
        return triggerTs <= staleBefore;
    });

    console.log("Total tasks in queue:", allQueueTasks.length);
    console.log("Eligible for dequeue (executed/stale):", completedOrStaleTasks.length);

    const selectedTasks = completedOrStaleTasks.slice(0, args.batchSize);
    if (selectedTasks.length === 0) {
        console.log("No eligible tasks found. Nothing to dequeue.");
        return;
    }

    for (const task of selectedTasks) {
        const taskPubkey = task.publicKey as PublicKey;
        const rentRefund = task.account.rentRefund as PublicKey;
        const taskId = task.account.id;

        try {
            const signature = await tuktukProgram.methods
                .dequeueTaskV0()
                .accountsPartial({
                    queueAuthority,
                    rentRefund,
                    taskQueueAuthority,
                    taskQueue,
                    task: taskPubkey,
                })
                .rpc();

            console.log(`Dequeued task id=${taskId} pubkey=${taskPubkey.toBase58()} tx=${signature}`);
        } catch (error) {
            console.error(`Failed to dequeue task id=${taskId} pubkey=${taskPubkey.toBase58()}:`, error);
        }
    }

    console.log("Cleanup run completed.");
}

async function main() {
    const argv = parseArgs();

    console.log("================ TukTuk Cleanup Worker ================");
    console.log("Mode:", argv.runOnce ? "run-once" : "weekly-daemon");
    console.log("Task Queue:", argv.taskQueue);
    console.log("Batch Size:", argv.batchSize);
    console.log("Stale Seconds:", argv.staleSeconds);
    console.log("Week Seconds:", argv.weekSeconds);
    console.log("Worker Started At (UTC):", new Date().toISOString());
    console.log("=======================================================");

    if (argv.runOnce) {
        console.log("Running single cleanup pass and exiting...");
        await cleanupTaskQueue(argv);
        console.log("Run-once cleanup finished.");
        return;
    }

    await cleanupTaskQueue(argv);
    console.log(`Starting scheduler loop. Next cleanup every ${argv.weekSeconds} seconds.`);

    setInterval(async () => {
        try {
            await cleanupTaskQueue(argv);
        } catch (error) {
            console.error("Scheduled cleanup failed:", error);
        }
    }, argv.weekSeconds * 1000);
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});