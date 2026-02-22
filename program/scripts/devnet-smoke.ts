// SPDX-License-Identifier: Apache-2.0
import * as anchor from "@coral-xyz/anchor";
import {
  Ed25519Program,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { blake3 } from "@noble/hashes/blake3";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";

const RPC_URL = process.env.ANCHOR_PROVIDER_URL || "https://api.devnet.solana.com";
const PROGRAM_ID = new PublicKey(
  process.env.EDGERUN_PROGRAM_ID || "A2ac8yDnTXKfZCHWqcJVYFfR2jv65kezW95XTgrrdbtG"
);
const WALLET_PATH = process.env.ANCHOR_WALLET || path.join(homedir(), ".config/solana/id.json");

const walletBytes = JSON.parse(readFileSync(WALLET_PATH, "utf8")) as number[];
const payer = Keypair.fromSecretKey(Uint8Array.from(walletBytes));
const connection = new anchor.web3.Connection(RPC_URL, "confirmed");
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(payer), {
  commitment: "confirmed",
  preflightCommitment: "confirmed",
});
anchor.setProvider(provider);

const idl = JSON.parse(
  readFileSync(new URL("../target/idl/edgerun_program.json", import.meta.url), "utf8")
);
idl.address = PROGRAM_ID.toBase58();
const program = new anchor.Program(idl, provider);

function random32(): number[] {
  return Array.from(Keypair.generate().publicKey.toBytes());
}

function writableAccounts(pubkeys: readonly PublicKey[]) {
  return pubkeys.map((pubkey) => ({
    pubkey,
    isWritable: true,
    isSigner: false,
  }));
}

function parseEd25519Signature(ix: TransactionInstruction): number[] {
  const data = ix.data;
  const signatureOffset = data.readUInt16LE(2);
  return Array.from(data.slice(signatureOffset, signatureOffset + 64));
}

function buildResultDigest(
  jobId: number[],
  bundleHash: number[],
  outputHash: number[],
  runtimeId: number[]
): Buffer {
  const preimage = Buffer.concat([
    Buffer.from(jobId),
    Buffer.from(bundleHash),
    Buffer.from(outputHash),
    Buffer.from(runtimeId),
  ]);
  return Buffer.from(blake3(preimage));
}

async function ensureConfig(configPda: PublicKey): Promise<void> {
  const info = await connection.getAccountInfo(configPda, "confirmed");
  if (info) {
    return;
  }
  const sig = await program.methods
    .initializeConfig({
      schedulerAuthority: payer.publicKey,
      randomnessAuthority: payer.publicKey,
      daWindowSlots: new anchor.BN(64),
      nonResponseSlashLamports: new anchor.BN(50_000),
      committeeTieringEnabled: false,
      maxCommitteeSize: 9,
      minWorkerStakeLamports: new anchor.BN(100_000_000),
      protocolFeeBps: 100,
      challengeWindowSlots: new anchor.BN(64),
      maxMemoryBytes: 1_048_576,
      maxInstructions: new anchor.BN(500_000),
      allowedRuntimeRoot: new Array<number>(32).fill(0),
      paused: false,
    })
    .accountsStrict({
      admin: payer.publicKey,
      config: configPda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("initialize_config:", sig);
}

async function main() {
  console.log("RPC:", RPC_URL);
  console.log("Program:", PROGRAM_ID.toBase58());
  console.log("Payer:", payer.publicKey.toBase58());

  const [configPda] = PublicKey.findProgramAddressSync([Buffer.from("config")], PROGRAM_ID);
  await ensureConfig(configPda);

  const updateSig = await program.methods
    .updateConfig({
      schedulerAuthority: payer.publicKey,
      randomnessAuthority: payer.publicKey,
      daWindowSlots: new anchor.BN(64),
      nonResponseSlashLamports: new anchor.BN(50_000),
      committeeTieringEnabled: false,
      maxCommitteeSize: 9,
      minWorkerStakeLamports: new anchor.BN(100_000_000),
      protocolFeeBps: 100,
      challengeWindowSlots: new anchor.BN(64),
      maxMemoryBytes: 1_048_576,
      maxInstructions: new anchor.BN(500_000),
      allowedRuntimeRoot: new Array<number>(32).fill(0),
      paused: false,
    })
    .accountsStrict({
      admin: payer.publicKey,
      config: configPda,
    })
    .rpc();
  console.log("update_config:", updateSig);

  const workers = [Keypair.generate(), Keypair.generate(), Keypair.generate()];
  const workerStakePdas = workers.map((w) =>
    PublicKey.findProgramAddressSync([Buffer.from("worker_stake"), w.publicKey.toBuffer()], PROGRAM_ID)[0]
  );

  const fundTx = new Transaction();
  for (const worker of workers) {
    fundTx.add(
      SystemProgram.transfer({
        fromPubkey: payer.publicKey,
        toPubkey: worker.publicKey,
        lamports: 300_000_000,
      })
    );
  }
  const fundSig = await provider.sendAndConfirm(fundTx);
  console.log("fund_workers:", fundSig);

  for (let i = 0; i < workers.length; i += 1) {
    const worker = workers[i];
    const workerStake = workerStakePdas[i];
    const registerSig = await program.methods
      .registerWorkerStake()
      .accountsStrict({
        worker: worker.publicKey,
        workerStake,
        systemProgram: SystemProgram.programId,
      })
      .signers([worker])
      .rpc();
    console.log(`register_worker_${i}:`, registerSig);

    const depositSig = await program.methods
      .depositStake(new anchor.BN(200_000_000))
      .accountsStrict({
        worker: worker.publicKey,
        workerStake,
        systemProgram: SystemProgram.programId,
      })
      .signers([worker])
      .rpc();
    console.log(`deposit_stake_${i}:`, depositSig);
  }

  const jobId = random32();
  const bundleHash = random32();
  const runtimeId = random32();
  const [jobPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("job"), Buffer.from(jobId)],
    PROGRAM_ID
  );
  const jobResultPdas = workers.map((w) =>
    PublicKey.findProgramAddressSync(
      [Buffer.from("job_result"), Buffer.from(jobId), w.publicKey.toBuffer()],
      PROGRAM_ID
    )[0]
  );
  const [outputPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("output"), Buffer.from(jobId)],
    PROGRAM_ID
  );

  const postSig = await program.methods
    .postJob(
      jobId,
      bundleHash,
      runtimeId,
      [],
      65_536,
      new anchor.BN(100_000),
      new anchor.BN(150_000_000)
    )
    .accountsStrict({
      client: payer.publicKey,
      config: configPda,
      job: jobPda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("post_job:", postSig);

  const seedSig = await program.methods
    .setJobSeed(random32())
    .accountsStrict({
      randomnessAuthority: payer.publicKey,
      config: configPda,
      job: jobPda,
    })
    .rpc();
  console.log("set_job_seed:", seedSig);

  const assignSig = await program.methods
    .assignWorkers(workers.map((w) => w.publicKey))
    .accountsStrict({
      schedulerAuthority: payer.publicKey,
      config: configPda,
      job: jobPda,
    })
    .remainingAccounts(writableAccounts(workerStakePdas))
    .rpc();
  console.log("assign_workers:", assignSig);

  const winningHash = random32();
  const losingHash = random32();
  for (let i = 0; i < workers.length; i += 1) {
    const worker = workers[i];
    const outputHash = i < 2 ? winningHash : losingHash;
    const digest = buildResultDigest(jobId, bundleHash, outputHash, runtimeId);
    const verifyIx = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: worker.secretKey,
      message: digest,
    });
    const attestationSig = parseEd25519Signature(verifyIx);
    const submitSig = await program.methods
      .submitResult(outputHash, attestationSig)
      .accountsStrict({
        worker: worker.publicKey,
        job: jobPda,
        jobResult: jobResultPdas[i],
        instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([verifyIx])
      .signers([worker])
      .rpc();
    console.log(`submit_result_${i}:`, submitSig);
  }

  const quorumSig = await program.methods
    .reachQuorum()
    .accountsStrict({
      caller: payer.publicKey,
      config: configPda,
      job: jobPda,
    })
    .remainingAccounts(
      jobResultPdas.map((pubkey) => ({ pubkey, isWritable: false, isSigner: false }))
    )
    .rpc();
  console.log("reach_quorum:", quorumSig);

  const declareSig = await program.methods
    .declareOutput(winningHash, Buffer.from("ipfs://devnet-smoke/phase2"))
    .accountsStrict({
      publisher: workers[0].publicKey,
      job: jobPda,
      jobResult: jobResultPdas[0],
      outputAvailability: outputPda,
      systemProgram: SystemProgram.programId,
    })
    .signers([workers[0]])
    .rpc();
  console.log("declare_output:", declareSig);

  const finalizeSig = await program.methods
    .finalizeJob()
    .accountsStrict({
      caller: payer.publicKey,
      config: configPda,
      job: jobPda,
      outputAvailability: outputPda,
    })
    .remainingAccounts([
      ...jobResultPdas.map((pubkey) => ({ pubkey, isWritable: false, isSigner: false })),
      ...writableAccounts(workerStakePdas),
      { pubkey: workers[0].publicKey, isWritable: true, isSigner: false },
      { pubkey: workers[1].publicKey, isWritable: true, isSigner: false },
    ])
    .rpc();
  console.log("finalize_job:", finalizeSig);

  const job = await program.account.job.fetch(jobPda);
  console.log("job_status:", job.status, "(2 means Finalized)");
  console.log("job_pda:", jobPda.toBase58());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
