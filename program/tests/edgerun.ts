import * as anchor from "@coral-xyz/anchor";
import { expect } from "chai";
import {
  Ed25519Program,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import type { EdgerunProgram } from "../target/types/edgerun_program";

const PROGRAM_ID = new PublicKey("AgjxA2CoMmmWXrcsJtvvpmqdRHLVHrhYf6DAuBCL4s5T");

describe("edgerun", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.EdgerunProgram as anchor.Program<EdgerunProgram>;
  const schedulerAuthority = provider.wallet as anchor.Wallet;
  const worker0 = Keypair.generate();
  const worker1 = Keypair.generate();
  const worker2 = Keypair.generate();
  const lifecycleWorker0 = Keypair.generate();
  const lifecycleWorker1 = Keypair.generate();
  const lifecycleWorker2 = Keypair.generate();

  const jobId = random32();
  const bundleHash = random32();
  const runtimeId = random32();

  const [configPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    PROGRAM_ID
  );
  const [workerStake0Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), worker0.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [workerStake1Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), worker1.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [workerStake2Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), worker2.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [lifecycleWorkerStake0Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), lifecycleWorker0.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [lifecycleWorkerStake1Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), lifecycleWorker1.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [lifecycleWorkerStake2Pda] = PublicKey.findProgramAddressSync(
    [Buffer.from("worker_stake"), lifecycleWorker2.publicKey.toBuffer()],
    PROGRAM_ID
  );
  const [jobPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("job"), Buffer.from(jobId)],
    PROGRAM_ID
  );
  const [jobResult0Pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("job_result"),
      Buffer.from(jobId),
      worker0.publicKey.toBuffer(),
    ],
    PROGRAM_ID
  );
  const [jobResult1Pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("job_result"),
      Buffer.from(jobId),
      worker1.publicKey.toBuffer(),
    ],
    PROGRAM_ID
  );
  const [jobResult2Pda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("job_result"),
      Buffer.from(jobId),
      worker2.publicKey.toBuffer(),
    ],
    PROGRAM_ID
  );

  before(async () => {
    expect(program.programId.equals(PROGRAM_ID)).to.equal(true);
    await airdrop(worker0.publicKey, 3_000_000_000);
    await airdrop(worker1.publicKey, 3_000_000_000);
    await airdrop(worker2.publicKey, 3_000_000_000);
    await airdrop(lifecycleWorker0.publicKey, 4_000_000_000);
    await airdrop(lifecycleWorker1.publicKey, 4_000_000_000);
    await airdrop(lifecycleWorker2.publicKey, 4_000_000_000);

    await program.methods
      .initializeConfig({
        schedulerAuthority: schedulerAuthority.publicKey,
        minWorkerStakeLamports: new anchor.BN(500_000_000),
        protocolFeeBps: 100,
        challengeWindowSlots: new anchor.BN(100),
        maxMemoryBytes: 1_048_576,
        maxInstructions: new anchor.BN(500_000),
        paused: false,
      })
      .accounts({
        admin: schedulerAuthority.publicKey,
        config: configPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await registerAndFundWorker(worker0, workerStake0Pda, 1_000_000_000);
    await registerAndFundWorker(worker1, workerStake1Pda, 1_000_000_000);
    await registerAndFundWorker(worker2, workerStake2Pda, 1_000_000_000);
    await registerAndFundWorker(
      lifecycleWorker0,
      lifecycleWorkerStake0Pda,
      2_000_000_000
    );
    await registerAndFundWorker(
      lifecycleWorker1,
      lifecycleWorkerStake1Pda,
      2_000_000_000
    );
    await registerAndFundWorker(
      lifecycleWorker2,
      lifecycleWorkerStake2Pda,
      2_000_000_000
    );

    await program.methods
      .postJob(
        jobId,
        bundleHash,
        runtimeId,
        65_536,
        new anchor.BN(100_000),
        new anchor.BN(1_000_000_000)
      )
      .accounts({
        client: schedulerAuthority.publicKey,
        config: configPda,
        job: jobPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .assignWorkers([
        worker0.publicKey,
        worker1.publicKey,
        worker2.publicKey,
      ])
      .accounts({
        schedulerAuthority: schedulerAuthority.publicKey,
        config: configPda,
        job: jobPda,
        workerStake0: workerStake0Pda,
        workerStake1: workerStake1Pda,
        workerStake2: workerStake2Pda,
      })
      .rpc();
  });

  it("rejects submit_result without ed25519 pre-instruction", async () => {
    const outputHash = random32();
    const fakeSig = new Array<number>(64).fill(7);

    await expectFail(
      program.methods
        .submitResult(outputHash, fakeSig)
        .accounts({
          worker: worker1.publicKey,
          job: jobPda,
          jobResult: jobResult1Pda,
          instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .signers([worker1])
        .rpc()
    );
  });

  it("rejects submit_result when pre-instruction message does not match", async () => {
    const outputHash = random32();
    const wrongOutputHash = random32();
    const wrongMessage = buildAttestationMessage(jobId, worker2.publicKey, wrongOutputHash);
    const verifyIx = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: worker2.secretKey,
      message: wrongMessage,
    });
    const attestationSig = parseEd25519Signature(verifyIx);

    await expectFail(
      program.methods
        .submitResult(outputHash, attestationSig)
        .accounts({
          worker: worker2.publicKey,
          job: jobPda,
          jobResult: jobResult2Pda,
          instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
        })
        .preInstructions([verifyIx])
        .signers([worker2])
        .rpc()
    );
  });

  it("accepts submit_result with valid ed25519 pre-instruction", async () => {
    const outputHash = random32();
    const message = buildAttestationMessage(jobId, worker0.publicKey, outputHash);
    const verifyIx = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: worker0.secretKey,
      message,
    });
    const attestationSig = parseEd25519Signature(verifyIx);

    await program.methods
      .submitResult(outputHash, attestationSig)
      .accounts({
        worker: worker0.publicKey,
        job: jobPda,
        jobResult: jobResult0Pda,
        instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        systemProgram: SystemProgram.programId,
      })
      .preInstructions([verifyIx])
      .signers([worker0])
      .rpc();

    const stored = await program.account.jobResult.fetch(jobResult0Pda);
    expect(bytesEqual(stored.outputHash as number[], outputHash)).to.equal(true);
    expect(bytesEqual(stored.attestationSig as number[], attestationSig)).to.equal(true);
  });

  it("finalize_job unlocks winner stake and pays protocol + winners", async () => {
    const finalizeJobId = random32();
    const { jobPda: finalizeJobPda } = await createAssignedJob(
      finalizeJobId,
      900_000_000,
      [lifecycleWorker0, lifecycleWorker1, lifecycleWorker2],
      [lifecycleWorkerStake0Pda, lifecycleWorkerStake1Pda, lifecycleWorkerStake2Pda]
    );
    const beforeJob = await program.account.job.fetch(finalizeJobPda);
    const requiredLock = beforeJob.requiredLockLamports.toNumber();

    const stake0Before = await program.account.workerStake.fetch(lifecycleWorkerStake0Pda);
    const stake1Before = await program.account.workerStake.fetch(lifecycleWorkerStake1Pda);
    const stake2Before = await program.account.workerStake.fetch(lifecycleWorkerStake2Pda);
    const configLamportsBefore = await provider.connection.getBalance(configPda);
    const worker0LamportsBefore = await provider.connection.getBalance(
      lifecycleWorker0.publicKey
    );
    const worker1LamportsBefore = await provider.connection.getBalance(
      lifecycleWorker1.publicKey
    );
    const worker2LamportsBefore = await provider.connection.getBalance(
      lifecycleWorker2.publicKey
    );

    const winnerCount = 3;
    const protocolFee = Math.floor(beforeJob.escrowLamports.toNumber() / 100);
    const payoutEach = Math.floor((beforeJob.escrowLamports.toNumber() - protocolFee) / winnerCount);
    const payoutRemainder =
      beforeJob.escrowLamports.toNumber() - protocolFee - payoutEach * winnerCount;

    await program.methods
      .finalizeJob(random32(), winnerCount)
      .accounts({
        schedulerAuthority: schedulerAuthority.publicKey,
        config: configPda,
        job: finalizeJobPda,
        workerStake0: lifecycleWorkerStake0Pda,
        workerStake1: lifecycleWorkerStake1Pda,
        workerStake2: lifecycleWorkerStake2Pda,
      })
      .remainingAccounts([
        { pubkey: lifecycleWorker0.publicKey, isWritable: true, isSigner: false },
        { pubkey: lifecycleWorker1.publicKey, isWritable: true, isSigner: false },
        { pubkey: lifecycleWorker2.publicKey, isWritable: true, isSigner: false },
      ])
      .rpc();

    const afterJob = await program.account.job.fetch(finalizeJobPda);
    const stake0After = await program.account.workerStake.fetch(lifecycleWorkerStake0Pda);
    const stake1After = await program.account.workerStake.fetch(lifecycleWorkerStake1Pda);
    const stake2After = await program.account.workerStake.fetch(lifecycleWorkerStake2Pda);
    const configLamportsAfter = await provider.connection.getBalance(configPda);
    const worker0LamportsAfter = await provider.connection.getBalance(
      lifecycleWorker0.publicKey
    );
    const worker1LamportsAfter = await provider.connection.getBalance(
      lifecycleWorker1.publicKey
    );
    const worker2LamportsAfter = await provider.connection.getBalance(
      lifecycleWorker2.publicKey
    );

    expect(afterJob.status).to.equal(2); // Finalized
    expect(afterJob.escrowLamports.toNumber()).to.equal(0);
    expect(
      stake0Before.lockedStakeLamports.toNumber() - stake0After.lockedStakeLamports.toNumber()
    ).to.equal(requiredLock);
    expect(
      stake1Before.lockedStakeLamports.toNumber() - stake1After.lockedStakeLamports.toNumber()
    ).to.equal(requiredLock);
    expect(
      stake2Before.lockedStakeLamports.toNumber() - stake2After.lockedStakeLamports.toNumber()
    ).to.equal(requiredLock);
    expect(configLamportsAfter - configLamportsBefore).to.equal(protocolFee);
    expect(worker0LamportsAfter - worker0LamportsBefore).to.equal(payoutEach + payoutRemainder);
    expect(worker1LamportsAfter - worker1LamportsBefore).to.equal(payoutEach);
    expect(worker2LamportsAfter - worker2LamportsBefore).to.equal(payoutEach);
  });

  it("cancel_expired_job returns escrow to client after deadline", async () => {
    await program.methods
      .updateConfig({
        schedulerAuthority: schedulerAuthority.publicKey,
        minWorkerStakeLamports: new anchor.BN(500_000_000),
        protocolFeeBps: 100,
        challengeWindowSlots: new anchor.BN(0),
        maxMemoryBytes: 1_048_576,
        maxInstructions: new anchor.BN(500_000),
        paused: false,
      })
      .accounts({
        admin: schedulerAuthority.publicKey,
        config: configPda,
      })
      .rpc();

    const cancelJobId = random32();
    const { jobPda: cancelJobPda } = await createAssignedJob(
      cancelJobId,
      700_000_000,
      [lifecycleWorker0, lifecycleWorker1, lifecycleWorker2],
      [lifecycleWorkerStake0Pda, lifecycleWorkerStake1Pda, lifecycleWorkerStake2Pda]
    );
    const cancelJobBefore = await program.account.job.fetch(cancelJobPda);
    await waitForSlotAfter(cancelJobBefore.deadlineSlot.toNumber());

    const jobLamportsBefore = await provider.connection.getBalance(cancelJobPda);

    await program.methods
      .cancelExpiredJob()
      .accounts({
        caller: schedulerAuthority.publicKey,
        job: cancelJobPda,
        client: schedulerAuthority.publicKey,
      })
      .rpc();

    const cancelJobAfter = await program.account.job.fetch(cancelJobPda);
    const jobLamportsAfter = await provider.connection.getBalance(cancelJobPda);

    expect(cancelJobAfter.status).to.equal(3); // Cancelled
    expect(cancelJobAfter.escrowLamports.toNumber()).to.equal(0);
    expect(jobLamportsBefore - jobLamportsAfter).to.equal(700_000_000);
  });

  it("slash_worker burns required lock from stake and transfers to config", async () => {
    const slashJobId = random32();
    const { jobPda: slashJobPda } = await createAssignedJob(
      slashJobId,
      600_000_000,
      [lifecycleWorker0, lifecycleWorker1, lifecycleWorker2],
      [lifecycleWorkerStake0Pda, lifecycleWorkerStake1Pda, lifecycleWorkerStake2Pda]
    );
    const slashJob = await program.account.job.fetch(slashJobPda);
    const slashAmount = slashJob.requiredLockLamports.toNumber();

    const stakeBefore = await program.account.workerStake.fetch(lifecycleWorkerStake2Pda);
    const configLamportsBefore = await provider.connection.getBalance(configPda);

    await program.methods
      .slashWorker()
      .accounts({
        schedulerAuthority: schedulerAuthority.publicKey,
        config: configPda,
        job: slashJobPda,
        workerStake: lifecycleWorkerStake2Pda,
      })
      .rpc();

    const stakeAfter = await program.account.workerStake.fetch(lifecycleWorkerStake2Pda);
    const configLamportsAfter = await provider.connection.getBalance(configPda);

    expect(
      stakeBefore.totalStakeLamports.toNumber() - stakeAfter.totalStakeLamports.toNumber()
    ).to.equal(slashAmount);
    expect(
      stakeBefore.lockedStakeLamports.toNumber() - stakeAfter.lockedStakeLamports.toNumber()
    ).to.equal(slashAmount);
    expect(configLamportsAfter - configLamportsBefore).to.equal(slashAmount);
  });

  async function registerAndFundWorker(
    worker: Keypair,
    workerStakePda: PublicKey,
    stakeLamports: number
  ) {
    await program.methods
      .registerWorkerStake()
      .accounts({
        worker: worker.publicKey,
        workerStake: workerStakePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([worker])
      .rpc();

    await program.methods
      .depositStake(new anchor.BN(stakeLamports))
      .accounts({
        worker: worker.publicKey,
        workerStake: workerStakePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([worker])
      .rpc();
  }

  async function airdrop(pubkey: PublicKey, lamports: number) {
    const sig = await provider.connection.requestAirdrop(pubkey, lamports);
    await provider.connection.confirmTransaction(sig, "confirmed");
  }

  async function createAssignedJob(
    jobIdBytes: number[],
    escrowLamports: number,
    workers: [Keypair, Keypair, Keypair],
    workerStakes: [PublicKey, PublicKey, PublicKey]
  ) {
    const [jobPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("job"), Buffer.from(jobIdBytes)],
      PROGRAM_ID
    );

    await program.methods
      .postJob(
        jobIdBytes,
        random32(),
        random32(),
        65_536,
        new anchor.BN(100_000),
        new anchor.BN(escrowLamports)
      )
      .accounts({
        client: schedulerAuthority.publicKey,
        config: configPda,
        job: jobPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    await program.methods
      .assignWorkers([
        workers[0].publicKey,
        workers[1].publicKey,
        workers[2].publicKey,
      ])
      .accounts({
        schedulerAuthority: schedulerAuthority.publicKey,
        config: configPda,
        job: jobPda,
        workerStake0: workerStakes[0],
        workerStake1: workerStakes[1],
        workerStake2: workerStakes[2],
      })
      .rpc();

    return { jobPda };
  }

  async function waitForSlotAfter(target: number) {
    for (let i = 0; i < 50; i++) {
      const slot = await provider.connection.getSlot("processed");
      if (slot > target) {
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 200));
    }
    expect.fail(`slot did not advance beyond ${target}`);
  }
});

function parseEd25519Signature(ix: TransactionInstruction): number[] {
  const data = ix.data;
  const signatureOffset = data.readUInt16LE(2);
  return Array.from(data.slice(signatureOffset, signatureOffset + 64));
}

function buildAttestationMessage(
  jobId: number[],
  worker: PublicKey,
  outputHash: number[]
): Buffer {
  return Buffer.concat([
    Buffer.from("ER"),
    Buffer.from(jobId),
    worker.toBuffer(),
    Buffer.from(outputHash),
  ]);
}

function random32(): number[] {
  return Array.from(anchor.web3.Keypair.generate().publicKey.toBytes());
}

function bytesEqual(a: number[], b: number[]): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

async function expectFail(promise: Promise<string>) {
  try {
    await promise;
    expect.fail("expected transaction to fail");
  } catch (_) {
    // expected
  }
}
