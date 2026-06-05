#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/network-primitives/mesh-core.wat");
const wasm = path.join(os.tmpdir(), `mesh-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.mesh_node_id_len(), 64);
  assert.strictEqual(e.mesh_signature_len(), 64);
  assert.strictEqual(e.mesh_header_len(), 130);
  assert.strictEqual(e.mesh_min_wire_len(), 194);
  assert.strictEqual(e.mesh_max_payload_len(), 65536);
  assert.strictEqual(e.mesh_default_ttl(), 16);
  assert.strictEqual(e.mesh_frame_type_code(7), 7);
  assert.strictEqual(e.mesh_frame_type_code(8), -1);
  assert.strictEqual(e.mesh_frame_type_admissible(6), 1);
  assert.strictEqual(e.mesh_frame_type_admissible(9), 0);
  assert.strictEqual(e.mesh_wire_len(256), 450);
  assert.strictEqual(e.mesh_payload_len_from_wire(193), -1);
  assert.strictEqual(e.mesh_payload_len_from_wire(194), 0);
  assert.strictEqual(e.mesh_from_wire_result(194, 0), 0);
  assert.strictEqual(e.mesh_from_wire_result(193, 0), 1);
  assert.strictEqual(e.mesh_from_wire_result(65731, 0), 2);
  assert.strictEqual(e.mesh_inspect_result(194, 0, 1), 0);
  assert.strictEqual(e.mesh_inspect_result(194, 8, 1), 3);
  assert.strictEqual(e.mesh_inspect_result(194, 0, 0), 4);
  assert.strictEqual(e.mesh_policy_result(0, 0, 1, 1, 0, 0, 65536), 0);
  assert.strictEqual(e.mesh_policy_result(0, 0, 0, 0, 0, 0, 65536), 5);
  assert.strictEqual(e.mesh_public_policy_allows(0, 0), 1);
  assert.strictEqual(e.mesh_local_only_policy_allows(0, 1), 1);
  assert.strictEqual(e.mesh_local_only_policy_allows(0, 0), 0);
  assert.strictEqual(e.mesh_signed_preimage_len(20), 150);
  assert.strictEqual(e.mesh_signature_domain_code(), 1);

  assert.strictEqual(e.mesh_route_update_action(0, 0, 5), 1);
  assert.strictEqual(e.mesh_route_update_action(1, 4, 3), 2);
  assert.strictEqual(e.mesh_route_update_action(1, 3, 3), 0);
  assert.strictEqual(e.mesh_route_lookup_prefers_new(5, 4), 1);
  assert.strictEqual(e.mesh_route_remove_via_keeps(1, 1), 0);
  assert.strictEqual(e.mesh_route_remove_via_keeps(1, 0), 1);
  assert.strictEqual(e.mesh_peer_dead(2), 0);
  assert.strictEqual(e.mesh_peer_dead(3), 1);
  assert.strictEqual(e.mesh_heartbeat_after_tick(2), 3);
  assert.strictEqual(e.mesh_heartbeat_went_dead(2), 1);
  assert.strictEqual(e.mesh_process_discovery_route_action(0, 0, 2, 0, 0), 1);
  assert.strictEqual(e.mesh_process_discovery_route_action(0, 0, 2, 1, 10), 2);
  assert.strictEqual(e.mesh_process_discovery_route_action(1, 0, 2, 0, 0), 0);
  assert.strictEqual(e.mesh_direct_peer_route_action(1, 2), 2);
  assert.strictEqual(e.mesh_next_hop_kind(0, 1, 0), 1);
  assert.strictEqual(e.mesh_next_hop_kind(0, 1, 1), 2);
  assert.strictEqual(e.mesh_next_hop_kind(1, 1, 1), 0);
  assert.strictEqual(e.mesh_should_forward_result(0, 16, 1), 15);
  assert.strictEqual(e.mesh_should_forward_result(0, 0, 1), -1);

  assert.strictEqual(e.mesh_discovery_max_routes(), 50);
  assert.strictEqual(e.mesh_discovery_encoded_len(2), 135);
  assert.strictEqual(e.mesh_discovery_encoded_len(70), 3255);
  assert.strictEqual(e.mesh_discovery_decode_result(135, 2), 0);
  assert.strictEqual(e.mesh_discovery_decode_result(134, 2), 2);
  assert.strictEqual(e.mesh_next_sequence(41), 42);

  assert.strictEqual(e.mesh_session_constant(1), 65n);
  assert.strictEqual(e.mesh_session_constant(2), 129n);
  assert.strictEqual(e.mesh_session_constant(5), 1000000n);
  assert.strictEqual(e.mesh_session_constant(6), 300n);
  assert.strictEqual(e.mesh_handshake_decode_result(129, 1), 0);
  assert.strictEqual(e.mesh_handshake_decode_result(128, 1), 1);
  assert.strictEqual(e.mesh_handshake_decode_result(129, 0), 2);
  assert.strictEqual(e.mesh_encrypt_wire_len(0), 28);
  assert.strictEqual(e.mesh_encrypt_wire_len(20), 48);
  assert.strictEqual(e.mesh_decrypt_result(11, 0, 0n, 0n, 1), 5);
  assert.strictEqual(e.mesh_decrypt_result(40, 1, 5n, 5n, 1), 6);
  assert.strictEqual(e.mesh_decrypt_result(40, 0, 6n, 5n, 0), 4);
  assert.strictEqual(e.mesh_decrypt_result(40, 1, 6n, 5n, 1), 0);
  assert.strictEqual(e.mesh_needs_rekey(999999n, 0n, 299n), 0);
  assert.strictEqual(e.mesh_needs_rekey(1000000n, 0n, 1n), 1);
  assert.strictEqual(e.mesh_needs_rekey(1n, 0n, 300n), 1);
  assert.strictEqual(e.mesh_session_manager_encrypt_result(0, 0), 1);
  assert.strictEqual(e.mesh_session_manager_encrypt_result(1, 1), 2);
  assert.strictEqual(e.mesh_replay_clock_action(0), 7);

  assert.strictEqual(e.mesh_link_constant(1), 0x88b5);
  assert.strictEqual(e.mesh_link_constant(2), 47080);
  assert.strictEqual(e.mesh_link_constant(3), 47079);
  assert.strictEqual(e.mesh_send_transport_choice(1, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.mesh_send_transport_choice(0, 1, 0, 0, 1, 1), 2);
  assert.strictEqual(e.mesh_send_transport_choice(0, 0, 1, 0, 1, 0), 3);
  assert.strictEqual(e.mesh_send_transport_choice(0, 0, 0, 1, 1, 0), 4);
  assert.strictEqual(e.mesh_send_transport_choice(0, 0, 0, 0, 1, 0), 5);
  assert.strictEqual(e.mesh_send_transport_choice(0, 0, 0, 0, 0, 1), 6);
  assert.strictEqual(e.mesh_inbound_action(0, 1, 0, -1), 0);
  assert.strictEqual(e.mesh_inbound_action(1, 1, 1, -1), 1);
  assert.strictEqual(e.mesh_inbound_action(1, 1, 0, -1), 2);
  assert.strictEqual(e.mesh_inbound_action(1, 0, 0, 15), 3);
  assert.strictEqual(e.mesh_admitted_frame_result(1, 0, 0, 1), 1);
  assert.strictEqual(e.mesh_admitted_frame_result(1, 0, 0, 0), 0);

  console.log("mesh core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
