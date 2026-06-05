#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/protocol-primitives/core-validator-core.wat");
const wasm = path.join(os.tmpdir(), `core-validator-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.core_validator_abi_version(), 1);
  assert.strictEqual(e.core_validator_record_version_result(1), 0);
  assert.strictEqual(e.core_validator_record_version_result(2), 1);
  assert.strictEqual(e.core_validator_timestamp_result(999999999), 0);
  assert.strictEqual(e.core_validator_timestamp_result(1000000000), 1);
  assert.strictEqual(e.core_validator_digest_result(1, 1, 32), 0);
  assert.strictEqual(e.core_validator_digest_result(0, 1, 32), 1);
  assert.strictEqual(e.core_validator_digest_result(1, 9, 32), 2);
  assert.strictEqual(e.core_validator_digest_result(1, 1, 20), 3);

  assert.strictEqual(e.core_validator_identity_record_result(1, 3, 1, 1, 64, 1, 0, 1, 1, 1, 1, 1, 64, 1, 1), 0);
  assert.strictEqual(e.core_validator_identity_record_result(1, 0, 1, 1, 64, 1, 0, 1, 1, 1, 1, 1, 64, 1, 1), 2);
  assert.strictEqual(e.core_validator_identity_record_result(1, 3, 1, 2, 64, 1, 0, 1, 1, 1, 1, 1, 64, 1, 1), 4);
  assert.strictEqual(e.core_validator_identity_record_result(1, 3, 1, 1, 64, 1, 1000000000, 1, 1, 1, 1, 1, 64, 1, 1), 5);
  assert.strictEqual(e.core_validator_identity_record_result(1, 3, 1, 1, 64, 1, 0, 1, 1, 1, 1, 1, 64, 0, 1), 8);

  assert.strictEqual(e.core_validator_stream_heads_proof_result(4, 1, 1), 0);
  assert.strictEqual(e.core_validator_stream_heads_proof_result(0, 1, 1), 1);
  assert.strictEqual(e.core_validator_snapshot_set_proof_result(4, 0, 1), 2);
  assert.strictEqual(e.core_validator_event_set_proof_result(4, 2, 0, 1), 3);
  assert.strictEqual(e.core_validator_object_assertion_proof_result(4, 1, 1, 0), 4);
  assert.strictEqual(e.core_validator_aggregate_summary_result(4, 1, 1, 1), 3);
  assert.strictEqual(e.core_validator_trust_policy_proof_result(4, 0, 0, 1, 1), 2);

  assert.strictEqual(e.core_validator_proof_bundle_result(1, 1, 0, 2, 1, 8, 1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.core_validator_proof_bundle_result(2, 1, 0, 2, 1, 8, 1, 1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.core_validator_proof_bundle_result(1, 1, 0, 2, 0, 8, 1, 1, 1, 1, 1, 1, 1), 3);
  assert.strictEqual(e.core_validator_proof_bundle_result(1, 1, 0, 2, 1, 8, 1, 1, 1, 1, 1, 0, 1), 8);
  assert.strictEqual(e.core_validator_federated_aggregate_result(1, 1, 4, 1, 1, 1, 1, 2, 1, 1, 1, 1), 0);
  assert.strictEqual(e.core_validator_federated_aggregate_result(1, 1, 4, 1, 1, 1, 1, 0, 1, 1, 1, 1), 7);

  assert.strictEqual(e.core_validator_route_trust_assignments_result(3, 1, 1), 0);
  assert.strictEqual(e.core_validator_route_trust_assignments_result(3, 0, 1), 2);
  assert.strictEqual(e.core_validator_aggregate_trust_policy_result(3, 1, 1, -1), 3);
  assert.strictEqual(e.core_validator_route_candidate_score(50, 20, 5, 1), 1065);
  assert.strictEqual(e.core_validator_route_candidate_allowed(20, 10, 5, 10, 1, 1, 1, 1, 50, 40), 1);
  assert.strictEqual(e.core_validator_route_candidate_allowed(20, 10, 5, 10, 1, 0, 1, 1, 50, 40), 0);
  assert.strictEqual(e.core_validator_route_replace_best(100, 1, 90, 0, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.core_validator_route_replace_best(100, 1, 100, 0, 0, 0, 0, 0, 1), 1);

  assert.strictEqual(e.core_validator_session_hello_result(4, 1, 1, 1, 16, 2, 0, 1, 1, 1), 0);
  assert.strictEqual(e.core_validator_session_hello_result(4, 1, 1, 1, 16, 2, 1, 1, 1, 1), 5);
  assert.strictEqual(e.core_validator_session_accept_result(4, 1, 1, 16, 1, 1, 1, 1), 0);
  assert.strictEqual(e.core_validator_session_accept_result(4, 1, 1, 16, 0, 1, 1, 1), 3);
  assert.strictEqual(e.core_validator_route_advertisement_result(4, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.core_validator_route_advertisement_result(4, 4, 1, 1, 1, 1, 1, 0, 1, 1, 1), 5);
  assert.strictEqual(e.core_validator_relay_envelope_result(4, 4, 1, 1, 1, 1, 0, 1, 0, 1, 1), 0);
  assert.strictEqual(e.core_validator_relay_envelope_result(4, 4, 1, 1, 1, 1, 1, 1, 1, 1, 1), 5);

  assert.strictEqual(e.core_validator_object_descriptor_result(4, 1, 1, 1, 2, 32, 1, 10n, 1, 1), 0);
  assert.strictEqual(e.core_validator_object_descriptor_result(4, 1, 1, 1, 2, 32, 1, -1n, 1, 1), 4);
  assert.strictEqual(e.core_validator_object_header_result(1, 4, 4, 1, 1, 1, 1, 10n, 1, 0), 6);
  assert.strictEqual(e.core_validator_chunk_manifest_result(1, 4, 1, 1, 2, 2, 20n, 1, 20n, 1, 1), 0);
  assert.strictEqual(e.core_validator_chunk_manifest_result(1, 4, 1, 1, 2, 2, 20n, 1, 19n, 1, 1), 7);
  assert.strictEqual(e.core_validator_representation_result(1, 0, 1, 0, 1), 1);
  assert.strictEqual(e.core_validator_representation_result(0, 0, 1, 1, 0), 4);

  assert.strictEqual(e.core_validator_command_result(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1), 0);
  assert.strictEqual(e.core_validator_command_result(1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 1), 5);
  assert.strictEqual(e.core_validator_command_result(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1), 9);
  assert.strictEqual(e.core_validator_fixture_signature_result(100, 0, 1), 0);
  assert.strictEqual(e.core_validator_fixture_signature_result(99, 0, 1), 1);
  assert.strictEqual(e.core_validator_fixture_signature_result(100, 1, 1), 2);
  assert.strictEqual(e.core_native_generated_record_code(13), 13);
  assert.strictEqual(e.core_native_generated_record_code(14), 0);

  console.log("core validator core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
