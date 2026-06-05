#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/apps-core.wat");
const wasm = path.join(os.tmpdir(), `apps-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.oauth_default_scope_count(), 3);
  assert.strictEqual(e.oauth_default_timeout_secs(), 300n);
  assert.strictEqual(e.oauth_default_path_code(1), 1);
  assert.strictEqual(e.oauth_default_path_code(3), 3);
  assert.strictEqual(e.oauth_credentials_expired(0, 100n, 0n, 200n), 1);
  assert.strictEqual(e.oauth_credentials_expired(1, 100n, 30n, 130n), 1);
  assert.strictEqual(e.oauth_credentials_valid(1, 1, 100n, 0n, 200n), 1);
  assert.strictEqual(e.oauth_credentials_valid(0, 1, 100n, 0n, 200n), 0);
  assert.strictEqual(e.oauth_token_request_field_count(1, 0, 1, 1, 1, 0, 0), 6);
  assert.strictEqual(e.oauth_token_response_field_count(1, 1, 1, 1, 1, 1, 0, 0), 6);
  assert.strictEqual(e.oauth_device_response_result(1, 1, 1, 1), 0);
  assert.strictEqual(e.oauth_device_response_result(1, 0, 1, 1), 2);
  assert.strictEqual(e.oauth_device_response_default(1), 600n);
  assert.strictEqual(e.oauth_device_response_default(2), 5n);
  assert.strictEqual(e.oauth_poll_error_action(1, 5n), 5n);
  assert.strictEqual(e.oauth_poll_error_action(2, 5n), 7n);
  assert.strictEqual(e.oauth_auth_url_field_count(), 7);
  assert.strictEqual(e.oauth_json_parse_object_result(1, 1, 0, 0), 0);
  assert.strictEqual(e.oauth_json_parse_object_result(0, 1, 0, 0), 1);
  assert.strictEqual(e.oauth_json_escape_code(34), 1);
  assert.strictEqual(e.oauth_json_escape_code(10), 3);
  assert.strictEqual(e.oauth_jwt_parse_result(3, 1, 1), 0);
  assert.strictEqual(e.oauth_jwt_parse_result(2, 1, 1), 1);
  assert.strictEqual(e.oauth_jwt_payload_expired(100n, 30n, 130n), 1);
  assert.strictEqual(e.oauth_jwt_aud_valid(0, 1), 1);
  assert.strictEqual(e.oauth_jwt_nonce_valid(1, 0, 0), 0);
  assert.strictEqual(e.oauth_jwt_at_hash_valid(0, 0), 1);
  assert.strictEqual(e.oauth_jwt_verify_dispatch(1, 1, 1), 0);
  assert.strictEqual(e.oauth_jwt_verify_dispatch(1, 2, 1), 1);
  assert.strictEqual(e.oauth_jwk_verifier_result(1, 1, 1, 1, 0, 0, 0), 0);
  assert.strictEqual(e.oauth_jwk_verifier_result(2, 0, 0, 0, 1, 0, 0), 0);
  assert.strictEqual(e.oauth_jwk_verifier_result(3, 0, 0, 0, 0, 0, 1), 0);
  assert.strictEqual(e.oauth_constant_time_eq(32, 32, 0), 1);
  assert.strictEqual(e.oauth_secret_namespace_code(1, 1), 1);

  assert.strictEqual(e.exchange_id_valid(1, 32, 1), 1);
  assert.strictEqual(e.exchange_id_valid(1, 31, 1), 0);
  assert.strictEqual(e.exchange_mode_code(2), 2);
  assert.strictEqual(e.exchange_mode_code(9), 1);
  assert.strictEqual(e.exchange_amount_side_code(2), 2);
  assert.strictEqual(e.exchange_quote_input_result(1, 1, 1, 1, 1, 1), 0);
  assert.strictEqual(e.exchange_quote_input_result(1, 1, 1, 1, 0, 1), 5);
  assert.strictEqual(e.exchange_quote_amounts_present(1, 1), 1);
  assert.strictEqual(e.exchange_quote_amounts_present(2, 1), 2);
  assert.strictEqual(e.exchange_payment_request_input_result(1, 1, 1, 1), 0);
  assert.strictEqual(e.exchange_payment_request_input_result(1, 1, 1, 0), 4);
  assert.strictEqual(e.exchange_payment_quote_result(1, 0, 1), 0);
  assert.strictEqual(e.exchange_payment_quote_result(1, 1, 1), 2);
  assert.strictEqual(e.exchange_order_input_result(1, 1, 1), 0);
  assert.strictEqual(e.exchange_order_input_result(1, 0, 1), 2);
  assert.strictEqual(e.exchange_order_status_str_code(9), 9);
  assert.strictEqual(e.exchange_order_status_str_code(20), 0);
  assert.strictEqual(e.exchange_record_provider_status_result(1, 1, 1, 0, 1, 1), 6);
  assert.strictEqual(e.exchange_record_provider_status_result(0, 1, 1, 0, 1, 0), 1);
  assert.strictEqual(e.exchange_route_code(2, 1, 9), 1);
  assert.strictEqual(e.exchange_route_code(1, 8, 7), 8);
  assert.strictEqual(e.exchange_assets_catalog_count(), 5);
  assert.strictEqual(e.exchange_health_status(), 2);

  assert.strictEqual(e.tor_cell_len(), 514);
  assert.strictEqual(e.tor_relay_payload_len(), 498);
  assert.strictEqual(e.tor_relay_cell_payload_len(), 509);
  assert.strictEqual(e.tor_command_code(1), 5);
  assert.strictEqual(e.tor_command_code(9), 131);
  assert.strictEqual(e.tor_relay_command_code(3), 14);
  assert.strictEqual(e.tor_relay_command_code(7), 37);
  assert.strictEqual(e.tor_fixed_cell_len_for_body(509), 514);
  assert.strictEqual(e.tor_fixed_cell_len_for_body(510), -1);
  assert.strictEqual(e.tor_var_cell_len_v0(6), 11);
  assert.strictEqual(e.tor_var_cell_len_v3(6), 13);
  assert.strictEqual(e.tor_read_any_cell_body_kind(7), 1);
  assert.strictEqual(e.tor_read_any_cell_body_kind(3), 2);
  assert.strictEqual(e.tor_parse_versions_result(6, 5), 5);
  assert.strictEqual(e.tor_parse_versions_result(4, 0), 2);
  assert.strictEqual(e.tor_base64_decode_result(1, 0), 1);
  assert.strictEqual(e.tor_base32_decode_result(0, 35, 1), 0);
  assert.strictEqual(e.tor_base32_decode_result(0, 34, 1), 2);
  assert.strictEqual(e.tor_consensus_line_action(4), 4);
  assert.strictEqual(e.tor_consensus_relay_accept(20, 9001, 1), 1);
  assert.strictEqual(e.tor_relay_cell_data_len(600), 498);
  assert.strictEqual(e.tor_decrypt_relay_result(1, 498, 1, 2, 2), 2);
  assert.strictEqual(e.tor_decrypt_relay_result(1, 100, 1, 2, 2), 0);
  assert.strictEqual(e.tor_authenticate_body_len(), 356);
  assert.strictEqual(e.tor_percentile_index(101, 9500), 95);

  console.log("apps core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
