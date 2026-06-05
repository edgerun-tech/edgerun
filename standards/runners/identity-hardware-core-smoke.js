#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/identity-primitives/identity-hardware-core.wat");
const wasm = path.join(os.tmpdir(), `identity-hardware-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.identity_mesh_public_key_len(), 64);
  assert.strictEqual(e.identity_mesh_signature_len(), 64);
  assert.strictEqual(e.identity_algorithm_is_mesh(3), 1);
  assert.strictEqual(e.identity_algorithm_is_mesh(1), 0);
  assert.strictEqual(e.identity_assurance_strength(4), 4);
  assert.strictEqual(e.identity_assurance_strength(9), 0);
  assert.strictEqual(e.identity_assurance_at_least(4, 2), 1);
  assert.strictEqual(e.identity_node_id_result(3, 64), 0);
  assert.strictEqual(e.identity_node_id_result(2, 64), 1);
  assert.strictEqual(e.identity_node_id_result(3, 65), 2);
  assert.strictEqual(e.identity_validate_key_info(1, 1, 1, 1, 1, 1, 1, 64, 1, 1), 0);
  assert.strictEqual(e.identity_validate_key_info(0, 1, 0, 0, 0, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.identity_validate_key_info(1, 1, 0, 0, 1, 0, 0, 0, 0, 0), 4);
  assert.strictEqual(e.identity_signature_input_len(10, 32), 43);
  assert.strictEqual(e.identity_provider_code(3), 3);
  assert.strictEqual(e.identity_provider_code(4), 0);
  assert.strictEqual(e.identity_map_tpm_algorithm(6), 6);
  assert.strictEqual(e.identity_map_tpm_algorithm(9), 7);
  assert.strictEqual(e.identity_map_tpm_assurance(2), 2);
  assert.strictEqual(e.identity_map_tpm_assurance(4), 4);
  assert.strictEqual(e.identity_map_yubikey_assurance(3), 2);
  assert.strictEqual(e.identity_map_yubikey_assurance(4), 4);
  assert.strictEqual(e.identity_yubikey_default_algorithm_allowed_count(0), 5);
  assert.strictEqual(e.identity_yubikey_hardware_algorithm_result(5), -1);

  assert.strictEqual(e.identity_tpm_command_code(1), 0x15d);
  assert.strictEqual(e.identity_tpm_command_code(6), 0x176);
  assert.strictEqual(e.identity_tpm_name_algorithm(2), 0x000b);
  assert.strictEqual(e.identity_tpm_name_algorithm(9), 0x0010);
  assert.strictEqual(e.identity_tpm_map_public_type(0x0023), 2);
  assert.strictEqual(e.identity_tpm_map_public_type(0x9999), 0);
  assert.strictEqual(e.identity_tpm_map_ecc_curve(0x0003), 1);
  assert.strictEqual(e.identity_tpm_auth_command_len(3, 4), 16);
  assert.strictEqual(e.identity_tpm_auth_value_area_len(8), 17);
  assert.strictEqual(e.identity_tpm_sign_command_len(32, 0, 0), 60);
  assert.strictEqual(e.identity_tpm_sign_command_len(32, 0, 17), 81);
  assert.strictEqual(e.identity_tpm_hash_command_len(16), 34);
  assert.strictEqual(e.identity_tpm_fixed_command_len(1), 12);
  assert.strictEqual(e.identity_tpm_fixed_command_len(2), 14);
  assert.strictEqual(e.identity_tpm_response_header_result(10, 10, 0), 0);
  assert.strictEqual(e.identity_tpm_response_header_result(9, 9, 0), 1);
  assert.strictEqual(e.identity_tpm_response_header_result(10, 11, 0), 2);
  assert.strictEqual(e.identity_tpm_response_header_result(10, 10, 1), 3);
  assert.strictEqual(e.identity_tpm_parse_sign_result(0, 0x0018, 0), 0);
  assert.strictEqual(e.identity_tpm_parse_sign_result(0, 0x9999, 0), 5);
  assert.strictEqual(e.identity_tpm_public_area_result(0x0023, 64, 0x0003), 1);
  assert.strictEqual(e.identity_tpm_public_area_result(0x0023, 32, 0x0003), 2);
  assert.strictEqual(e.identity_tpm_public_area_result(0x0001, 256, 0), 3);
  assert.strictEqual(e.identity_tpm_strip_ecdsa_result(72, 32, 32, 1), 64);
  assert.strictEqual(e.identity_tpm_strip_ecdsa_result(7, 32, 32, 1), -1);

  assert.strictEqual(e.identity_yubikey_product_known(0x0407), 1);
  assert.strictEqual(e.identity_yubikey_product_known(0x9999), 0);
  assert.strictEqual(e.identity_yubikey_slot_key_ref(1), 0x9a);
  assert.strictEqual(e.identity_yubikey_slot_key_ref(2), 0x9c);
  assert.strictEqual(e.identity_yubikey_apdu_len(1, 11), 16);
  assert.strictEqual(e.identity_yubikey_apdu_len(2, 0), 4);
  assert.strictEqual(e.identity_yubikey_apdu_len(3, 3), 10);
  assert.strictEqual(e.identity_yubikey_apdu_len(4, 0), 13);
  assert.strictEqual(e.identity_yubikey_pin_result(5), 1);
  assert.strictEqual(e.identity_yubikey_pin_result(6), 0);
  assert.strictEqual(e.identity_yubikey_pin_result(8), 0);
  assert.strictEqual(e.identity_yubikey_piv_algorithm_id(1), 0x07);
  assert.strictEqual(e.identity_yubikey_piv_algorithm_id(3), 0x11);
  assert.strictEqual(e.identity_yubikey_piv_algorithm_id(4), 0x14);
  assert.strictEqual(e.identity_yubikey_digest_len(3), 32);
  assert.strictEqual(e.identity_yubikey_digest_len(4), 48);
  assert.strictEqual(e.identity_yubikey_general_auth_apdu_len(32), 43);
  assert.strictEqual(e.identity_yubikey_parse_apdu_response(2, 0x9000), 0);
  assert.strictEqual(e.identity_yubikey_parse_apdu_response(1, 0x9000), 1);
  assert.strictEqual(e.identity_yubikey_parse_apdu_response(2, 0x6a82), 2);
  assert.strictEqual(e.identity_yubikey_parse_version(3), 0);
  assert.strictEqual(e.identity_yubikey_parse_version(2), 1);
  assert.strictEqual(e.identity_yubikey_parse_piv_algorithm(0x07), 1);
  assert.strictEqual(e.identity_yubikey_parse_piv_algorithm(0x11), 3);
  assert.strictEqual(e.identity_yubikey_parse_piv_algorithm(0x14), 4);

  console.log("identity hardware core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
