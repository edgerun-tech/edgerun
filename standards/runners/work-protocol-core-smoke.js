#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/protocol-primitives/work-protocol-core.wat");
const wasm = path.join(os.tmpdir(), `work-protocol-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.work_wire_abi_version(), 1);
  assert.strictEqual(e.work_default_heartbeat_secs(), 10n);
  assert.strictEqual(e.work_max_frame_len(), 1048576);
  assert.strictEqual(e.work_max_relay_transit_bundle_hops(), 64);
  assert.strictEqual(e.work_node_role_valid(1), 1);
  assert.strictEqual(e.work_node_role_valid(9), 0);
  assert.strictEqual(e.work_type_department(1), 3);
  assert.strictEqual(e.work_type_department(2), 4);
  assert.strictEqual(e.work_type_department(3), 5);
  assert.strictEqual(e.work_type_department(12), 7);
  assert.strictEqual(e.work_packet_tag_valid(7), 1);
  assert.strictEqual(e.work_identity_result(4, 1), 0);
  assert.strictEqual(e.work_identity_result(9, 1), 1);
  assert.strictEqual(e.work_identity_result(4, 0), 2);

  assert.strictEqual(e.work_admission_result(1, 1, 1, 1, 4, 1, 50n, 100n, 90n, 100n), 0);
  assert.strictEqual(e.work_admission_result(0, 1, 1, 1, 4, 1, 50n, 100n, 90n, 100n), 1);
  assert.strictEqual(e.work_admission_result(1, 1, 0, 1, 4, 1, 50n, 100n, 90n, 100n), 3);
  assert.strictEqual(e.work_admission_result(1, 1, 1, 1, 3, 1, 50n, 100n, 90n, 100n), 4);
  assert.strictEqual(e.work_admission_result(1, 1, 1, 1, 4, 0, 50n, 100n, 90n, 100n), 5);
  assert.strictEqual(e.work_admission_result(1, 1, 1, 1, 4, 1, 150n, 100n, 90n, 100n), 6);
  assert.strictEqual(e.work_admission_result(1, 1, 1, 1, 4, 1, 50n, 100n, 101n, 100n), 7);
  assert.strictEqual(e.work_reserve_admission_result(1, 0, 1, 100n, 50n), 0);
  assert.strictEqual(e.work_reserve_admission_result(1, 0, 1, 10n, 50n), 4);

  assert.strictEqual(e.work_receipt_common_result(1, 1, 1, 1, 0, 1, 20n, 30n, 100n), 0);
  assert.strictEqual(e.work_receipt_common_result(1, 1, 1, 1, 1, 1, 20n, 30n, 100n), 4);
  assert.strictEqual(e.work_receipt_common_result(1, 1, 1, 1, 0, 0, 20n, 30n, 100n), 5);
  assert.strictEqual(e.work_receipt_common_result(1, 1, 1, 1, 0, 1, 90n, 30n, 100n), 6);
  assert.strictEqual(e.work_unchecked_receipt_evidence_result(1, 0), 7);
  assert.strictEqual(e.work_unchecked_receipt_evidence_result(2, 0), 0);
  assert.strictEqual(e.work_commit_settlement_spent_after(20n, 30n), 50n);
  assert.strictEqual(e.work_prune_refund(100n, 60n), 40n);

  assert.strictEqual(e.work_batch_result(2, 1, 0, 0, 0, 60n, 100n), 0);
  assert.strictEqual(e.work_batch_result(0, 1, 0, 0, 0, 0n, 100n), 1);
  assert.strictEqual(e.work_batch_result(2, 1, 1, 0, 0, 60n, 100n), 4);
  assert.strictEqual(e.work_batch_result(2, 1, 0, 0, 0, 160n, 100n), 6);
  assert.strictEqual(e.work_ordered_channel_result(1, 1, 1n, 1n, 1), 0);
  assert.strictEqual(e.work_ordered_channel_result(1, 1, 2n, 1n, 1), 3);

  assert.strictEqual(e.work_relay_delivery_evidence_result(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.work_relay_delivery_evidence_result(2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.work_relay_delivery_evidence_result(1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1), 3);
  assert.strictEqual(e.work_relay_delivery_evidence_result(1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1), 8);

  assert.strictEqual(e.work_relay_transit_builder_result(1, 0), 0);
  assert.strictEqual(e.work_relay_transit_builder_result(0, 64), 1);
  assert.strictEqual(e.work_relay_transit_builder_result(65, 64), 2);
  assert.strictEqual(e.work_relay_transit_hop_result(0, 64, 2, 1, 1, 1), 0);
  assert.strictEqual(e.work_relay_transit_hop_result(2, 64, 2, 1, 1, 1), 1);
  assert.strictEqual(e.work_relay_transit_settlement_result(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.work_relay_transit_settlement_result(1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1), 9);
  assert.strictEqual(e.work_custody_kind_valid(3), 1);
  assert.strictEqual(e.work_custody_kind_valid(4), 0);
  assert.strictEqual(e.work_custody_result(1, 3, 1, 1, 1), 0);
  assert.strictEqual(e.work_custody_result(0, 3, 1, 1, 1), 1);
  assert.strictEqual(e.work_custody_result(1, 3, 0, 1, 1), 2);

  assert.strictEqual(e.work_role_claim_result(1, 1, 1, 1n, 1, 1, 1, 0, 0), 0);
  assert.strictEqual(e.work_role_claim_result(1, 1, 1, 0n, 1, 1, 1, 0, 0), 1);
  assert.strictEqual(e.work_role_claim_result(1, 1, 1, 1n, 1, 0, 1, 0, 0), 3);
  assert.strictEqual(e.work_ranges_overlap(1n, 3n, 3n, 5n), 1);
  assert.strictEqual(e.work_ranges_overlap(1n, 2n, 3n, 5n), 0);

  assert.strictEqual(e.work_storage_role_accepts(4, 2), 1);
  assert.strictEqual(e.work_storage_role_accepts(5, 3), 1);
  assert.strictEqual(e.work_storage_payload_kind_valid(3), 1);
  assert.strictEqual(e.work_store_request_result(2, 1), 0);
  assert.strictEqual(e.work_store_request_result(3, 1), 1);
  assert.strictEqual(e.work_retrieve_response_result(1), 0);
  assert.strictEqual(e.work_retrieve_response_result(0), 1);

  assert.strictEqual(e.work_capability_kind_to_work_type(1), 11);
  assert.strictEqual(e.work_capability_kind_to_work_type(4), 14);
  assert.strictEqual(e.work_capability_operation_matches_content(1, 1), 1);
  assert.strictEqual(e.work_capability_operation_matches_content(10, 6), 1);
  assert.strictEqual(e.work_capability_operation_matches_content(20, 2), 1);
  assert.strictEqual(e.work_capability_operation_matches_content(30, 4), 1);
  assert.strictEqual(e.work_capability_operation_matches_content(40, 5), 1);
  assert.strictEqual(e.work_capability_operation_matches_content(40, 6), 0);
  assert.strictEqual(e.work_capability_message_result(7, 1, 3, 1, 1, 1, 1), 0);
  assert.strictEqual(e.work_capability_message_result(4, 1, 3, 1, 1, 1, 1), 1);
  assert.strictEqual(e.work_capability_message_result(7, 1, 9, 1, 1, 1, 1), 2);
  assert.strictEqual(e.work_capability_message_result(7, 1, 3, 1, 0, 1, 1), 3);

  assert.strictEqual(e.work_admitted_route_result(1, 1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.work_admitted_route_result(1, 1, 1, 1, 0, 1, 1, 1), 3);
  assert.strictEqual(e.work_message_against_route_result(1, 1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.work_message_against_route_result(1, 1, 1, 0, 1, 1, 1), 2);
  assert.strictEqual(e.work_receipt_against_route_result(1, 1, 1, 1, 1, 1, 10n, 20n), 0);
  assert.strictEqual(e.work_receipt_against_route_result(1, 1, 1, 1, 1, 1, 30n, 20n), 3);

  console.log("work protocol core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
