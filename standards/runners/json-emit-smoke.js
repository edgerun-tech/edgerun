#!/usr/bin/env node
const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/json-emit.wat");
const wasm = path.join(os.tmpdir(), `json-emit-${process.pid}.wasm`);

function run(cmd, args) {
  const result = spawnSync(cmd, args, { encoding: "utf8" });
  if (result.status !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    process.exit(result.status ?? 1);
  }
}

function unpack(value) {
  const n = BigInt.asUintN(64, value);
  return {
    status: Number(n & 0xffffffffn),
    value: Number((n >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, bytes) {
  new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
}

function read(memory, ptr, len) {
  return Buffer.from(new Uint8Array(memory.buffer, ptr, len)).toString("utf8");
}

function readBytes(memory, ptr, len) {
  return Buffer.from(new Uint8Array(memory.buffer, ptr, len));
}

(async () => {
  run("wat2wasm", [wat, "-o", wasm]);
  run("wasm-validate", [wasm]);

  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm));
  const e = instance.exports;
  const memory = e.memory;
  const inPtr = 1024;
  const outPtr = 2048;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300043);

  function emitString(input, cap = 256) {
    const bytes = Buffer.from(input, "utf8");
    write(memory, inPtr, bytes);
    const result = unpack(e.json_emit_string(inPtr, bytes.length, outPtr, cap));
    return { ...result, text: read(memory, outPtr, result.value) };
  }

  assert.deepEqual(emitString("plain"), { status: 0, value: 7, text: "\"plain\"" });
  assert.deepEqual(emitString("hello\t\"world\"\n\u0007"), {
    status: 0,
    value: 26,
    text: "\"hello\\t\\\"world\\\"\\n\\u0007\"",
  });
  assert.deepEqual(emitString("slash\\path"), {
    status: 0,
    value: 13,
    text: "\"slash\\\\path\"",
  });
  assert.deepEqual(emitString("snow \u2603"), {
    status: 0,
    value: 10,
    text: "\"snow \u2603\"",
  });

  let short = emitString("abc", 4);
  assert.equal(short.status, 2);
  assert.equal(short.value, 4);

  function emitLiteral(kind) {
    const result = unpack(e.json_emit_literal(kind, outPtr, 32));
    return { ...result, text: read(memory, outPtr, result.value) };
  }
  assert.deepEqual(emitLiteral(0), { status: 0, value: 4, text: "null" });
  assert.deepEqual(emitLiteral(1), { status: 0, value: 5, text: "false" });
  assert.deepEqual(emitLiteral(2), { status: 0, value: 4, text: "true" });
  assert.deepEqual(unpack(e.json_emit_literal(9, outPtr, 32)), { status: 3, value: 0 });
  assert.deepEqual(unpack(e.json_emit_literal(1, outPtr, 4)), { status: 2, value: 4 });

  function emitI64(value) {
    const result = unpack(e.json_emit_i64(BigInt(value), outPtr, 64));
    return { ...result, text: read(memory, outPtr, result.value) };
  }
  assert.deepEqual(emitI64(0), { status: 0, value: 1, text: "0" });
  assert.deepEqual(emitI64(-42), { status: 0, value: 3, text: "-42" });
  assert.deepEqual(emitI64("-9223372036854775808"), {
    status: 0,
    value: 20,
    text: "-9223372036854775808",
  });

  function emitU64(value) {
    const result = unpack(e.json_emit_u64(BigInt(value), outPtr, 64));
    return { ...result, text: read(memory, outPtr, result.value) };
  }
  assert.deepEqual(emitU64(0), { status: 0, value: 1, text: "0" });
  assert.deepEqual(emitU64("18446744073709551615"), {
    status: 0,
    value: 20,
    text: "18446744073709551615",
  });
  assert.deepEqual(unpack(e.json_emit_u64(12345n, outPtr, 3)), { status: 2, value: 3 });

  for (const [kind, token] of [
    [0, "["],
    [1, "]"],
    [2, "{"],
    [3, "}"],
    [4, ","],
    [5, ":"],
  ]) {
    const result = unpack(e.json_emit_token(kind, outPtr, 1));
    assert.deepEqual({ ...result, text: read(memory, outPtr, result.value) }, {
      status: 0,
      value: 1,
      text: token,
    });
  }
  assert.deepEqual(unpack(e.json_emit_token(99, outPtr, 1)), { status: 3, value: 0 });

  write(memory, inPtr, Buffer.from("a:b\n"));
  const key = unpack(e.json_emit_key(inPtr, 4, outPtr, 32));
  assert.deepEqual({ ...key, text: read(memory, outPtr, key.value) }, {
    status: 0,
    value: 8,
    text: "\"a:b\\n\":",
  });

  const raw = Buffer.from([0, 1, 31]);
  write(memory, inPtr, raw);
  const ctrl = unpack(e.json_emit_string(inPtr, raw.length, outPtr, 32));
  assert.equal(ctrl.status, 0);
  assert.deepEqual(readBytes(memory, outPtr, ctrl.value), Buffer.from("\"\\u0000\\u0001\\u001f\""));

  fs.rmSync(wasm, { force: true });
  console.log(JSON.stringify({ ok: true, standard_id: e.proto_standard_id() }));
})().catch((error) => {
  fs.rmSync(wasm, { force: true });
  console.error(error);
  process.exit(1);
});
