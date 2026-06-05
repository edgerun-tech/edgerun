#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/exchange-provider-status.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function providerCode(exports, memory, text) {
  return exports.exchange_provider_code(...put(memory, 1024, text));
}

function mapStatus(exports, memory, provider, status) {
  const p = put(memory, 1024, provider);
  const s = put(memory, 2048, status);
  return exports.exchange_map_provider_status(p[0], p[1], s[0], s[1]);
}

function hash(exports, memory, text, ptr = 3072) {
  const [p, len] = put(memory, ptr, text);
  return exports.exchange_hash_lower(p, len);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300095);

  assert.strictEqual(providerCode(exports, memory, "SIDESHIFT"), 1);
  assert.strictEqual(providerCode(exports, memory, "changenow"), 2);
  assert.strictEqual(providerCode(exports, memory, "FFIO"), 3);
  assert.strictEqual(providerCode(exports, memory, "unknown"), 0);

  assert.strictEqual(exports.exchange_provider_features(1), 0x307);
  assert.strictEqual(exports.exchange_provider_features(2), 0x20f);
  assert.strictEqual(exports.exchange_provider_features(3), 0x107);

  assert.strictEqual(mapStatus(exports, memory, "SIDESHIFT", "awaiting_deposit"), 4);
  assert.strictEqual(mapStatus(exports, memory, "SIDESHIFT", "deposit_received"), 5);
  assert.strictEqual(mapStatus(exports, memory, "SIDESHIFT", "refund_required"), 11);
  assert.strictEqual(mapStatus(exports, memory, "CHANGENOW", "waiting"), 4);
  assert.strictEqual(mapStatus(exports, memory, "CHANGENOW", "finished"), 9);
  assert.strictEqual(mapStatus(exports, memory, "FFIO", "processing"), 6);
  assert.strictEqual(mapStatus(exports, memory, "FFIO", "cancelled"), 9);
  assert.strictEqual(mapStatus(exports, memory, "SIDESHIFT", "brand_new_status"), 17);

  const usdt = hash(exports, memory, "USDT");
  const btc = hash(exports, memory, "btc");
  const sol = hash(exports, memory, "sol");
  assert.strictEqual(exports.exchange_supports_pair_hash(1, usdt, btc), 1);
  assert.strictEqual(exports.exchange_supports_pair_hash(1, usdt, sol), 0);
  assert.strictEqual(exports.exchange_supports_pair_hash(2, usdt, sol), 1);

  assert.strictEqual(exports.exchange_event_stream_type(1), 1);
  assert.strictEqual(exports.exchange_event_stream_type(2), 2);
  assert.strictEqual(exports.exchange_event_stream_type(3), 3);
  assert.strictEqual(exports.exchange_event_stream_type(8), 3);
  assert.strictEqual(exports.exchange_event_has_order_id(1), 0);
  assert.strictEqual(exports.exchange_event_has_order_id(2), 1);
  assert.strictEqual(exports.exchange_event_terminal(6), 1);
  assert.strictEqual(exports.exchange_event_terminal(7), 1);
  assert.strictEqual(exports.exchange_event_terminal(8), 1);
  assert.strictEqual(exports.exchange_event_terminal(5), 0);

  assert.strictEqual(exports.exchange_projection_terminal_status(9), 1);
  assert.strictEqual(exports.exchange_projection_terminal_status(14), 0);
  assert.strictEqual(exports.exchange_terminal_event_for_status(9), 6);
  assert.strictEqual(exports.exchange_terminal_event_for_status(15), 7);
  assert.strictEqual(exports.exchange_terminal_event_for_status(13), 0);
  assert.strictEqual(exports.exchange_provider_contradiction(1, 9, 7), 17);
  assert.strictEqual(exports.exchange_provider_contradiction(1, 9, 9), 9);
  assert.strictEqual(exports.exchange_provider_contradiction(0, 7, 8), 8);

  assert.strictEqual(
    exports.exchange_settlement_command_valid(1, 1, 42, 1, 1, 1, 1000n, 2000n),
    1,
  );
  assert.strictEqual(
    exports.exchange_settlement_command_valid(1, 1, 42, 1, 1, 1, 3000n, 2000n),
    0,
  );
  assert.strictEqual(
    exports.exchange_settlement_command_valid(0, 1, 42, 1, 1, 1, 1000n, 0n),
    0,
  );

  console.log(
    JSON.stringify({
      unit: "exchange-provider-status",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "provider_code_case_insensitive",
        "provider_feature_flags",
        "sideshift_status_mapping",
        "changenow_status_mapping",
        "ffio_status_mapping",
        "unknown_status_on_hold",
        "supported_pairs",
        "changenow_sol_extra_pair",
        "event_stream_type_mapping",
        "terminal_event_policy",
        "provider_terminal_contradiction",
        "settlement_command_validity",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
