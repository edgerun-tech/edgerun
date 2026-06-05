#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tls-core-state.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, text, ptr = 1024) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + Math.max(128, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function text(e, memory, fn, value) {
  return e[fn](...put(memory, value));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300110);

  assert.strictEqual(e.tls_record_content_type(22), 3);
  assert.strictEqual(e.tls_record_content_type(23), 4);
  assert.strictEqual(e.tls_record_header_status(22, 0x0303, 32, 37), 0);
  assert.strictEqual(e.tls_record_header_status(99, 0x0303, 32, 37), 1);
  assert.strictEqual(e.tls_record_header_status(22, 0x0301, 32, 37), 2);
  assert.strictEqual(e.tls_record_header_status(22, 0x0303, 17000, 17005), 3);
  assert.strictEqual(e.tls_record_header_status(22, 0x0303, 32, 20), 4);

  assert.strictEqual(e.tls_alert_level(1), 1);
  assert.strictEqual(e.tls_alert_level(2), 2);
  assert.strictEqual(e.tls_alert_level(3), 0);
  assert.strictEqual(e.tls_alert_description(40), 2);
  assert.strictEqual(e.tls_alert_description(48), 3);
  assert.strictEqual(e.tls_alert_description(120), 9);
  assert.strictEqual(e.tls_alert_message_status(2, 40, 2), 0);
  assert.strictEqual(e.tls_alert_message_status(3, 40, 2), 2);
  assert.strictEqual(e.tls_alert_message_status(2, 255, 2), 3);

  assert.strictEqual(e.tls_handshake_type(1), 1);
  assert.strictEqual(e.tls_handshake_type(2), 2);
  assert.strictEqual(e.tls_handshake_type(20), 7);
  assert.strictEqual(e.tls_extension_type(0), 1);
  assert.strictEqual(e.tls_extension_type(16), 4);
  assert.strictEqual(e.tls_extension_type(51), 9);
  assert.strictEqual(e.tls_named_group(29), 3);

  assert.strictEqual(text(e, memory, "tls_hkdf_label", "key"), 1);
  assert.strictEqual(text(e, memory, "tls_hkdf_label", "c hs traffic"), 5);
  assert.strictEqual(text(e, memory, "tls_hkdf_label", "res master"), 10);
  assert.strictEqual(text(e, memory, "tls_alpn_protocol", "h3"), 1);
  assert.strictEqual(text(e, memory, "tls_alpn_protocol", "acme-tls/1"), 2);

  assert.strictEqual(e.tls_cert_list_status(0, 10, 14), 0);
  assert.strictEqual(e.tls_cert_list_status(3, 10, 6), 1);
  assert.strictEqual(e.tls_cert_list_status(0, 10, 12), 2);
  assert.strictEqual(e.tls_cert_valid_at(100n, 200n, 150n), 1);
  assert.strictEqual(e.tls_cert_valid_at(100n, 200n, 250n), 0);
  assert.strictEqual(text(e, memory, "tls_hostname_wildcard_shape", "*.example.com"), 1);
  assert.strictEqual(text(e, memory, "tls_hostname_wildcard_shape", "example.com"), 0);

  console.log(
    JSON.stringify({
      unit: "tls-core-state",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "record_content_type",
        "record_header_status",
        "alert_level_description",
        "handshake_extension_groups",
        "hkdf_label_mapping",
        "alpn_mapping",
        "certificate_list_status",
        "certificate_time_validity",
        "wildcard_hostname_shape",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
