#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/browser-core.wat");
const wasm = path.join(os.tmpdir(), `browser-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.browser_wire_constant(1), 2);
  assert.strictEqual(e.browser_wire_constant(6), 7);
  assert.strictEqual(e.browser_artifact_kind(0, 0, 1, 0, 0), 3);
  assert.strictEqual(e.browser_artifact_kind(0, 0, 0, 1, 0), 4);
  assert.strictEqual(e.browser_artifact_path_valid(8, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.browser_artifact_path_valid(8, 1, 0, 0, 0, 0), 0);
  assert.strictEqual(e.browser_code_hash_artifact_included(1), 0);
  assert.strictEqual(e.browser_code_hash_artifact_included(5), 1);
  assert.strictEqual(e.browser_verify_package_result(1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.browser_verify_package_result(1, 1, 0, 1, 1, 1, 1), 3);
  assert.strictEqual(e.browser_signature_verify_result(2, 1, 32, 1), 1);
  assert.strictEqual(e.browser_signature_verify_result(1, 1, 32, 1), 0);

  assert.strictEqual(e.browser_first_run_event_count(2), 4);
  assert.strictEqual(e.browser_first_run_event_count(1), 3);
  assert.strictEqual(e.browser_first_run_cache_required(2), 1);
  assert.strictEqual(e.browser_first_run_installs_app(3), 0);
  assert.strictEqual(e.browser_decision_valid(3), 1);
  assert.strictEqual(e.browser_decision_valid(4), 0);
  assert.strictEqual(e.browser_runtime_first_run_result(0, 1, 0, 0), 1);
  assert.strictEqual(e.browser_runtime_first_run_result(1, 2, 0, 0), 2);
  assert.strictEqual(e.browser_runtime_first_run_result(1, 1, 1, 1), 3);
  assert.strictEqual(e.browser_runtime_first_run_result(1, 2, 1, 1), 0);

  assert.strictEqual(e.browser_route_is_live(10n, 20n, 10n), 1);
  assert.strictEqual(e.browser_route_is_live(10n, 20n, 21n), 0);
  assert.strictEqual(e.browser_grant_valid(1, 1, 1, 1), 1);
  assert.strictEqual(e.browser_grant_valid(1, 1, 0, 1), 0);
  assert.strictEqual(e.browser_binding_matches_grant(1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.browser_session_valid(1, 1, 1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.browser_storage_envelope_supported(1, 1, 1, 2), 1);
  assert.strictEqual(e.browser_storage_envelope_supported(1, 1, 1, 9), 0);
  assert.strictEqual(e.browser_envelope_operation_matches_request(1, 6), 1);
  assert.strictEqual(e.browser_envelope_operation_matches_request(2, 7), 1);
  assert.strictEqual(e.browser_storage_request_bound(1, 1, 1, 1), 1);
  assert.strictEqual(e.browser_storage_response_status(0, 7, 1, 1), 1);
  assert.strictEqual(e.browser_storage_response_status(1, 7, 0, 1), 2);
  assert.strictEqual(e.browser_storage_response_status(1, 6, 1, 0), 2);
  assert.strictEqual(e.browser_storage_response_status(1, 6, 1, 1), 0);
  assert.strictEqual(e.browser_event_chain_step_valid(2n, 2n, 1, 1, 1), 1);
  assert.strictEqual(e.browser_event_chain_step_valid(2n, 3n, 1, 1, 1), 0);

  assert.strictEqual(e.browser_retrieval_policy_valid_at(10n, 20n, 15n), 1);
  assert.strictEqual(e.browser_retrieval_policy_valid_at(10n, 20n, 9n), 0);
  assert.strictEqual(e.browser_retrieval_cost(5n, 2n, 20n, 0n, 3n, 4n, 5n), 29n);
  assert.strictEqual(e.browser_retrieval_cost(5n, 2n, 40n, 50n, 3n, 4n, 5n), 40n);
  assert.strictEqual(e.browser_retrieval_cost(5n, 10n, 0n, 50n, 3n, 4n, 5n), 50n);
  assert.strictEqual(e.browser_policy_hash_source(0), 1);
  assert.strictEqual(e.browser_policy_hash_source(8), 2);
  assert.strictEqual(e.browser_work_request_valid_until(100n), 60100n);
  assert.strictEqual(e.browser_admitted_budget(0n), 1n);
  assert.strictEqual(e.browser_admitted_budget(7n), 7n);

  assert.strictEqual(e.browser_host_error_status(1), 1);
  assert.strictEqual(e.browser_host_error_status(2), 2);
  assert.strictEqual(e.browser_host_invoke_operation(6), 1);
  assert.strictEqual(e.browser_host_invoke_operation(7), 2);
  assert.strictEqual(e.browser_host_invoke_operation(9), 0);
  assert.strictEqual(e.browser_ffi_slice_valid(0, 8), 0);
  assert.strictEqual(e.browser_ffi_slice_valid(1, 8), 1);
  assert.strictEqual(e.browser_wasm_pages_needed(65536, 1), 0);
  assert.strictEqual(e.browser_wasm_pages_needed(65537, 1), 1);

  console.log("browser core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
