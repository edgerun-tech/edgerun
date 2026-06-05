#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/dhcp-message-core.wat";

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;
const STATUS_INVALID_COOKIE = 4;
const STATUS_INVALID_OPTION_LENGTH = 5;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    value: Number((packed >> 32n) & 0xffffffffn) >>> 0,
  };
}

function u32(view, offset) {
  return view.getUint32(offset, true);
}

function optionRecord(view, outPtr) {
  return {
    code: u32(view, outPtr),
    klass: u32(view, outPtr + 4),
    valueOff: u32(view, outPtr + 8),
    valueLen: u32(view, outPtr + 12),
    totalLen: u32(view, outPtr + 16),
    lengthStatus: u32(view, outPtr + 20),
  };
}

function writeMessage(memory, ptr, overrides = {}) {
  memory.fill(0, ptr, ptr + 320);
  memory[ptr] = overrides.op ?? 1;
  memory[ptr + 1] = overrides.htype ?? 1;
  memory[ptr + 2] = overrides.hlen ?? 6;
  memory[ptr + 3] = overrides.hops ?? 0;
  memory.set(overrides.xid ?? [0x12, 0x34, 0x56, 0x78], ptr + 4);
  memory.set(overrides.secs ?? [0x00, 0x2a], ptr + 8);
  memory.set(overrides.flags ?? [0x80, 0x00], ptr + 10);
  memory.set(overrides.ciaddr ?? [0, 0, 0, 0], ptr + 12);
  memory.set(overrides.yiaddr ?? [192, 168, 1, 100], ptr + 16);
  memory.set(overrides.siaddr ?? [192, 168, 1, 1], ptr + 20);
  memory.set(overrides.giaddr ?? [10, 0, 0, 1], ptr + 24);
  memory.set(overrides.chaddr ?? [0xde, 0xad, 0xbe, 0xef, 0x00, 0x01], ptr + 28);
  memory.set(overrides.cookie ?? [99, 130, 83, 99], ptr + 236);
  memory.set(overrides.options ?? [53, 1, 1, 55, 3, 1, 3, 6, 255], ptr + 240);
  return 240 + (overrides.options ?? [53, 1, 1, 55, 3, 1, 3, 6, 255]).length;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300102);
  assert.strictEqual(e.dhcp_magic_cookie(), 0x63825363);

  assert.strictEqual(e.dhcp_op_classify(1), 1);
  assert.strictEqual(e.dhcp_op_classify(2), 2);
  assert.strictEqual(e.dhcp_op_classify(9), 0);
  assert.strictEqual(e.dhcp_op_status(1), STATUS_OK);
  assert.strictEqual(e.dhcp_op_status(9), STATUS_INVALID);

  assert.strictEqual(e.dhcp_htype_classify(1, 6), 1);
  assert.strictEqual(e.dhcp_htype_classify(2, 8), 2);
  assert.strictEqual(e.dhcp_htype_classify(0, 6), 0);
  assert.strictEqual(e.dhcp_htype_classify(1, 17), 0);

  for (let mt = 1; mt <= 8; mt += 1) {
    assert.strictEqual(e.dhcp_message_type_classify(mt), mt);
    assert.strictEqual(e.dhcp_message_type_status(mt), STATUS_OK);
  }
  assert.strictEqual(e.dhcp_message_type_classify(9), 0);
  assert.strictEqual(e.dhcp_message_type_status(9), STATUS_INVALID);

  assert.strictEqual(e.dhcp_option_classify(0), 0);
  assert.strictEqual(e.dhcp_option_classify(255), 1);
  assert.strictEqual(e.dhcp_option_classify(1), 2);
  assert.strictEqual(e.dhcp_option_classify(6), 3);
  assert.strictEqual(e.dhcp_option_classify(51), 4);
  assert.strictEqual(e.dhcp_option_classify(53), 5);
  assert.strictEqual(e.dhcp_option_classify(55), 6);
  assert.strictEqual(e.dhcp_option_classify(66), 7);
  assert.strictEqual(e.dhcp_option_classify(93), 8);
  assert.strictEqual(e.dhcp_option_classify(94), 9);
  assert.strictEqual(e.dhcp_option_classify(97), 10);
  assert.strictEqual(e.dhcp_option_classify(43), 11);
  assert.strictEqual(e.dhcp_option_classify(61), 12);
  assert.strictEqual(e.dhcp_option_classify(222), 255);

  assert.strictEqual(e.dhcp_option_length_status(53, 1), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(53, 2), STATUS_INVALID_OPTION_LENGTH);
  assert.strictEqual(e.dhcp_option_length_status(1, 4), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(1, 3), STATUS_INVALID_OPTION_LENGTH);
  assert.strictEqual(e.dhcp_option_length_status(6, 8), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(6, 6), STATUS_INVALID_OPTION_LENGTH);
  assert.strictEqual(e.dhcp_option_length_status(51, 4), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(93, 2), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(94, 3), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(97, 17), STATUS_OK);
  assert.strictEqual(e.dhcp_option_length_status(97, 16), STATUS_INVALID_OPTION_LENGTH);
  assert.strictEqual(e.dhcp_option_length_status(12, 0), STATUS_OK);

  assert.strictEqual(e.dhcp_ipv4_pack(192, 168, 1, 1) >>> 0, 0xc0a80101);
  assert.strictEqual(e.dhcp_ipv4_pack(256, 511, 1, 300) >>> 0, 0x00ff012c);

  const len = writeMessage(memory, inPtr);
  assert.strictEqual(e.dhcp_cookie_status(inPtr, len), STATUS_OK);
  assert.strictEqual(e.dhcp_header_decode(inPtr, len, outPtr), STATUS_OK);
  assert.strictEqual(u32(view, outPtr), 1);
  assert.strictEqual(u32(view, outPtr + 4), 1);
  assert.strictEqual(u32(view, outPtr + 20), 0x12345678);
  assert.strictEqual(u32(view, outPtr + 24), 42);
  assert.strictEqual(u32(view, outPtr + 28), 0x8000);
  assert.strictEqual(u32(view, outPtr + 32), 1);
  assert.strictEqual(u32(view, outPtr + 36), 0x00000000);
  assert.strictEqual(u32(view, outPtr + 40), 0xc0a80164);
  assert.strictEqual(u32(view, outPtr + 44), 0xc0a80101);
  assert.strictEqual(u32(view, outPtr + 48), 0x0a000001);
  assert.strictEqual(u32(view, outPtr + 52), 0x63825363);

  assert.deepStrictEqual(unpack(e.dhcp_ipv4_field(inPtr, len, 1)), {
    status: STATUS_OK,
    value: 0xc0a80164,
  });
  assert.deepStrictEqual(unpack(e.dhcp_ipv4_field(inPtr, len, 4)), {
    status: STATUS_INVALID,
    value: 0,
  });

  let next = unpack(e.dhcp_option_next(inPtr + 240, len - 240, 0, outPtr));
  assert.deepStrictEqual(next, { status: STATUS_OK, value: 3 });
  assert.deepStrictEqual(optionRecord(view, outPtr), {
    code: 53,
    klass: 5,
    valueOff: 2,
    valueLen: 1,
    totalLen: 3,
    lengthStatus: STATUS_OK,
  });
  assert.strictEqual(memory[inPtr + 240 + optionRecord(view, outPtr).valueOff], 1);

  next = unpack(e.dhcp_option_next(inPtr + 240, len - 240, next.value, outPtr));
  assert.deepStrictEqual(next, { status: STATUS_OK, value: 8 });
  assert.deepStrictEqual(optionRecord(view, outPtr), {
    code: 55,
    klass: 6,
    valueOff: 5,
    valueLen: 3,
    totalLen: 5,
    lengthStatus: STATUS_OK,
  });

  next = unpack(e.dhcp_option_next(inPtr + 240, len - 240, next.value, outPtr));
  assert.deepStrictEqual(next, { status: STATUS_OK, value: 9 });
  assert.deepStrictEqual(optionRecord(view, outPtr), {
    code: 255,
    klass: 1,
    valueOff: 9,
    valueLen: 0,
    totalLen: 1,
    lengthStatus: STATUS_OK,
  });

  memory.set([0, 53, 2, 1, 2], inPtr);
  next = unpack(e.dhcp_option_next(inPtr, 5, 0, outPtr));
  assert.deepStrictEqual(next, { status: STATUS_OK, value: 1 });
  assert.deepStrictEqual(optionRecord(view, outPtr), {
    code: 0,
    klass: 0,
    valueOff: 1,
    valueLen: 0,
    totalLen: 1,
    lengthStatus: STATUS_OK,
  });
  next = unpack(e.dhcp_option_next(inPtr, 5, next.value, outPtr));
  assert.deepStrictEqual(next, { status: STATUS_INVALID_OPTION_LENGTH, value: 5 });
  assert.deepStrictEqual(optionRecord(view, outPtr), {
    code: 53,
    klass: 5,
    valueOff: 3,
    valueLen: 2,
    totalLen: 4,
    lengthStatus: STATUS_INVALID_OPTION_LENGTH,
  });

  memory.set([6, 4, 1, 1], inPtr);
  assert.deepStrictEqual(unpack(e.dhcp_option_next(inPtr, 4, 0, outPtr)), {
    status: STATUS_INPUT_SHORT,
    value: 4,
  });

  writeMessage(memory, inPtr, { op: 9 });
  assert.strictEqual(e.dhcp_header_decode(inPtr, 240, outPtr), STATUS_INVALID);
  writeMessage(memory, inPtr, { cookie: [0, 0, 0, 0] });
  assert.strictEqual(e.dhcp_cookie_status(inPtr, 240), STATUS_INVALID_COOKIE);
  assert.strictEqual(e.dhcp_header_decode(inPtr, 240, outPtr), STATUS_INVALID_COOKIE);
  assert.strictEqual(e.dhcp_cookie_status(inPtr, 239), STATUS_INPUT_SHORT);
  assert.strictEqual(e.dhcp_header_decode(inPtr, 239, outPtr), STATUS_INPUT_SHORT);
  assert.deepStrictEqual(unpack(e.dhcp_option_next(inPtr, 1, 0, 0)), {
    status: STATUS_OUTPUT_SHORT,
    value: 0,
  });

  console.log(
    JSON.stringify(
      {
        unit: "dhcp-message-core",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
