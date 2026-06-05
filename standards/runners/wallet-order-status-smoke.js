#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/wallet-order-status.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function packed(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function readAscii(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("ascii");
}

function expectLabel(exports, memory, id, label) {
  const outPtr = 2048;
  memory.fill(0, outPtr, outPtr + 64);
  const result = packed(exports.wallet_status_label(id, outPtr, label.length));
  assert.deepStrictEqual(result, { status: 0, written: label.length });
  assert.strictEqual(readAscii(memory, outPtr, label.length), label);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300085);

  for (let id = 1; id <= 19; id += 1) {
    assert.strictEqual(exports.wallet_status_valid(id), 1);
  }
  assert.strictEqual(exports.wallet_status_valid(0), 0);
  assert.strictEqual(exports.wallet_status_valid(20), 0);

  for (const id of [9, 13, 14, 15, 16, 18]) {
    assert.strictEqual(exports.wallet_status_terminal(id), 1);
  }
  assert.strictEqual(exports.wallet_status_terminal(3), 0);
  assert.strictEqual(exports.wallet_status_terminal(7), 0);

  assert.strictEqual(exports.wallet_status_can_transition(3, 4), 1);
  assert.strictEqual(exports.wallet_status_can_transition(4, 5), 1);
  assert.strictEqual(exports.wallet_status_can_transition(7, 8), 1);
  assert.strictEqual(exports.wallet_status_can_transition(8, 9), 1);
  assert.strictEqual(exports.wallet_status_can_transition(3, 9), 0);
  assert.strictEqual(exports.wallet_status_can_transition(9, 7), 0);
  assert.strictEqual(exports.wallet_status_can_transition(9, 9), 1);
  assert.strictEqual(exports.wallet_status_can_transition(7, 7), 1);

  expectLabel(exports, memory, 17, "ON_HOLD");
  expectLabel(exports, memory, 19, "PARTIAL_DEPOSITS");

  assert.deepStrictEqual(packed(exports.wallet_status_label(17, 2048, 6)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(packed(exports.wallet_status_label(0, 2048, 32)), {
    status: 3,
    written: 0,
  });

  console.log(
    JSON.stringify({
      unit: "wallet-order-status",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "terminal_completed_expired_failed_refunded_rejected_canceled",
        "created_to_awaiting_deposit",
        "awaiting_deposit_to_deposit_seen",
        "exchanging_to_sending",
        "sending_to_completed",
        "invalid_created_to_completed",
        "terminal_completed_to_exchanging_false",
        "same_state_true",
        "label_on_hold",
        "label_partial_deposits",
        "invalid_id",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
