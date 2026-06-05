#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/protocol-core-validation.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr = 4096) {
  memory.fill(0, ptr, ptr + Math.max(256, value.length + 1));
  memory.set(Buffer.from(value, "ascii"), ptr);
  return ptr;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const str = (fn, value) => e[fn](writeAscii(memory, value), value.length);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300116);

  assert.equal(str("core_value_kind_code", "null"), 1);
  assert.equal(str("core_value_kind_code", "map"), 6);
  assert.equal(str("core_verdict_code", "ACCEPT"), 1);
  assert.equal(str("core_verdict_code", "DUPLICATE"), 4);
  assert.equal(str("core_reason_code", "POLICY_DENIED"), 7);
  assert.equal(str("core_reason_code", "REPRESENTATION_INVALID"), 17);

  assert.equal(e.core_fixed_from_ratio(3n, 2n), 98304);
  assert.equal(e.core_fixed_from_int(-5), 0);
  assert.equal(Number(e.core_fixed_mul_u64(98304, 1000n)), 1500);
  assert.equal(e.core_fixed_mul_fp(98304, 131072), 196608);
  assert.equal(e.core_fixed_div_fp(196608, 131072), 98304);
  assert.equal(e.core_fixed_sub(10, 20), 0);
  assert.equal(e.core_fixed_frac4(98304), 5000);

  assert.equal(e.core_timestamp_shape(1n, 999999999), 0);
  assert.equal(e.core_timestamp_shape(1n, 1000000000), 1);

  assert.equal(e.core_command_required_capability(1), 1);
  assert.equal(e.core_command_required_capability(4), 3);
  assert.equal(e.core_command_required_capability(5), 4);
  assert.equal(e.core_command_required_capability(12), 6);
  assert.equal(e.core_command_required_capability(14), 7);
  assert.equal(e.core_command_required_capability(1001), 8);
  assert.equal(e.core_command_required_capability(1019), 1);
  assert.equal(e.core_command_required_action(14), 14);
  assert.equal(e.core_command_required_action(1001), 16);
  assert.equal(e.core_command_required_action(1019), 33);

  assert.equal(e.core_command_policy_verdict(1, 0, 0, 0, 0), 2);
  assert.equal(e.core_command_policy_verdict(0, 0, 1, 0, 0), 2);
  assert.equal(e.core_command_policy_verdict(1, 1, 1, 0, 1), 1);

  assert.equal(str("core_sig_hash_pair_code", "edgerun:v0:sig:command-envelope"), 2);
  assert.equal(str("core_sig_hash_pair_code", "edgerun:v0:sig:assurance-claim"), 12);

  console.log(JSON.stringify({ unit: "protocol-core-validation", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
