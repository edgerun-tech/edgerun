import * as anchor from "@coral-xyz/anchor";

describe("edgerun_program", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  it("initialize_config", async () => {
    // TODO: derive config PDA, invoke initialize_config, assert fields.
  });

  it("register_worker_stake + deposit/withdraw", async () => {
    // TODO: create worker stake PDA and verify stake accounting rules.
  });

  it("post_job", async () => {
    // TODO: derive job PDA, post a job, assert limits/deadline/posted status.
  });

  it("assign_workers", async () => {
    // TODO: lock stake for assigned workers and assert assigned status.
  });

  it("submit_result", async () => {
    // TODO: create JobResult PDA, assert worker assignment and stored hash/sig.
  });

  it("finalize_job", async () => {
    // TODO: assert quorum behavior and finalized state.
  });

  it("cancel_expired_job", async () => {
    // TODO: advance slot/time and assert cancellation behavior.
  });

  it("slash_worker", async () => {
    // TODO: slash losing worker lock and assert stake reductions.
  });
});
