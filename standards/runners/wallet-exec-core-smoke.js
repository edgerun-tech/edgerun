#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/wallet-exec-core.wat");
const wasm = path.join(os.tmpdir(), `wallet-exec-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.wallet_exec_abi_version(), 1);
  assert.strictEqual(e.wallet_core_abi_version(), 1);
  assert.strictEqual(e.exec_runner_magic() >>> 0, 0x52585245);
  assert.strictEqual(e.exec_max_program_size(), 536870912n);
  assert.strictEqual(e.wallet_chain_family_valid(5), 1);
  assert.strictEqual(e.wallet_chain_family_valid(6), 0);
  assert.strictEqual(e.wallet_is_edgerun_chain(1), 1);
  assert.strictEqual(e.wallet_amount_is_zero(0n, 0n), 1);
  assert.strictEqual(e.canonical_asset_id_parts(0), 2);
  assert.strictEqual(e.canonical_asset_id_parts(1), 3);
  assert.strictEqual(e.wallet_event_apply_result(4, 0, 0, 0, 0, 0), 0);
  assert.strictEqual(e.wallet_event_apply_result(4, 0, 1, 0, 0, 0), 1);
  assert.strictEqual(e.wallet_event_apply_result(2, 0, 0, 1, 0, 0), 2);
  assert.strictEqual(e.wallet_event_apply_result(5, 0, 0, 0, 0, 0), 3);
  assert.strictEqual(e.wallet_credit_result(1, 0), 5);
  assert.strictEqual(e.wallet_debit_result(0, 1, 0), 4);
  assert.strictEqual(e.wallet_emission_evidence_result(0, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.wallet_emission_evidence_result(0, 1, 1, 1, 0, 1, 1), 4);
  assert.strictEqual(e.wallet_preimage_domain_code(7), 7);

  assert.strictEqual(e.exec_msg_valid(24), 1);
  assert.strictEqual(e.exec_msg_valid(9), 0);
  assert.strictEqual(e.exec_read_frame_result(1, 1), 0);
  assert.strictEqual(e.exec_read_frame_result(0, 1), 1);
  assert.strictEqual(e.exec_read_frame_result(1, 2), 2);
  assert.strictEqual(e.exec_begin_result(84, 536870912n, 0), 0);
  assert.strictEqual(e.exec_begin_result(83, 1n, 0), 1);
  assert.strictEqual(e.exec_begin_result(84, 536870913n, 0), 2);
  assert.strictEqual(e.exec_argv_count_after_parse(0), 1);
  assert.strictEqual(e.exec_job_chunk_result(3, 8n, 10n), 0);
  assert.strictEqual(e.exec_job_chunk_result(2, 8n, 10n), 1);
  assert.strictEqual(e.exec_job_chunk_result(3, 11n, 10n), 2);
  assert.strictEqual(e.exec_job_end_result(4), 0);
  assert.strictEqual(e.exec_exit_code(1, 7, 0, 0), 7);
  assert.strictEqual(e.exec_exit_code(0, 0, 9, 0), 137);
  assert.strictEqual(e.exec_exit_code(0, 0, 0, 1), 124);
  assert.strictEqual(e.exec_pipe_msg_type(1), 11);

  assert.strictEqual(e.cas_put_begin_result(44, 1n), 0);
  assert.strictEqual(e.cas_put_begin_result(44, 536870913n), 2);
  assert.strictEqual(e.cas_put_result(1, 0), 25);
  assert.strictEqual(e.cas_put_result(0, 0), 13);
  assert.strictEqual(e.cas_put_result(0, 1), 23);
  assert.strictEqual(e.cas_put_chunk_result(21, 4n, 4n), 0);
  assert.strictEqual(e.cas_put_end_result(22), 0);
  assert.strictEqual(e.cas_exec_result(0), 13);
  assert.strictEqual(e.hex32_result(64, 1), 0);
  assert.strictEqual(e.hex32_result(63, 1), 1);
  assert.strictEqual(e.hex32_result(64, 0), 2);

  console.log("wallet exec core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
