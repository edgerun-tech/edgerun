#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/ws-frame.wat";

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
    value: Number((packed >> 32n) & 0xffffffffn),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300004);

  const cases = [];
  function mark(id) {
    cases.push(id);
  }

  const outPtr = 2048;
  const headerPtr = 1024;
  let packed = unpack(exports.ws_write_server_frame_header(2, 3, 0, headerPtr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 2 });
  assert.deepStrictEqual(Array.from(memory.slice(headerPtr, headerPtr + 2)), [0x82, 0x03]);
  mark("server-binary-small-header");

  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 2);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 3);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 0);
  mark("decode-prefix-small-unmasked");

  assert.strictEqual(exports.ws_decode_payload_len(headerPtr + 1, 1, 3, 1024, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 3);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 0);
  mark("decode-payload-len-small");

  memory.set([0x83, 0x00], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 3);
  mark("reject-reserved-opcode");

  memory.set([0x09, 0x00], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 3);
  mark("reject-fragmented-control");

  memory.set([0x89, 0x7e], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 3);
  mark("reject-oversized-control-prefix");

  memory.set([0x82, 0x7e, 0x01, 0x00], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 126);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 2);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr + 2, 2, 126, 1024, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 256);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 2);
  mark("decode-extended-126");

  memory.set([0x00, 0x7d], headerPtr);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr, 2, 126, 1024, outPtr), 3);
  memory.set([0x00, 0x7e], headerPtr);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr, 2, 126, 124, outPtr), 4);
  mark("reject-nonminimal-and-over-max-126");

  memory.set([0x82, 0x7f, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 127);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 8);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr + 2, 8, 127, 0xffffffff, outPtr), 4);
  mark("decode-extended-127-over-max");

  memory.set([0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], headerPtr);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr, 8, 127, 0xffffffff, outPtr), 3);

  memory.set([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff], headerPtr);
  assert.strictEqual(exports.ws_decode_payload_len(headerPtr, 8, 127, 0xffffffff, outPtr), 3);
  mark("reject-invalid-127-lengths");

  memory.set([0x00, 0x00, 0x00, 0x00], headerPtr);
  assert.deepStrictEqual(unpack(exports.ws_write_server_frame_header(3, 0, 0, headerPtr, 16)), {
    status: 3,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.ws_write_server_frame_header(2, 0, 0x80000000, headerPtr, 16)), {
    status: 3,
    value: 0,
  });
  mark("reject-server-header-invalid");

  const payloadPtr = 4096;
  memory.set([9, 11, 10, 12], payloadPtr);
  assert.strictEqual(exports.ws_apply_mask_in_place(payloadPtr, 2, 0x01020304), 0);
  assert.deepStrictEqual(Array.from(memory.slice(payloadPtr, payloadPtr + 2)), [8, 9]);
  assert.strictEqual(exports.ws_apply_mask_in_place(payloadPtr, 2, 0x01020304), 0);
  assert.deepStrictEqual(Array.from(memory.slice(payloadPtr, payloadPtr + 4)), [9, 11, 10, 12]);
  mark("mask-transform-round-trip");

  memory.set([0x89, 0x82], headerPtr);
  assert.strictEqual(exports.ws_decode_prefix(headerPtr, 2, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 9);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 2);
  mark("decode-masked-ping-prefix");

  packed = unpack(exports.ws_write_frame_header(1, 0x80, 5, 0, 1, 0x11223344, headerPtr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 6 });
  assert.deepStrictEqual(Array.from(memory.slice(headerPtr, headerPtr + 6)), [
    0x81, 0x85, 0x11, 0x22, 0x33, 0x44,
  ]);
  assert.strictEqual(exports.ws_parse_header(headerPtr, 6, 1024, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 12, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 5);
  assert.strictEqual(view.getUint32(outPtr + 20, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 24, true), 6);
  assert.strictEqual(view.getUint32(outPtr + 28, true), 0x11223344);
  mark("write-and-parse-masked-client-small");

  packed = unpack(exports.ws_write_frame_header(2, 0x80, 126, 0, 1, 0x01020304, headerPtr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 8 });
  assert.deepStrictEqual(Array.from(memory.slice(headerPtr, headerPtr + 8)), [
    0x82, 0xfe, 0x00, 0x7e, 0x01, 0x02, 0x03, 0x04,
  ]);
  assert.strictEqual(exports.ws_parse_header(headerPtr, 7, 1024, outPtr), 1);
  assert.strictEqual(exports.ws_parse_header(headerPtr, 8, 1024, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 126);
  assert.strictEqual(view.getUint32(outPtr + 24, true), 8);
  mark("write-and-parse-masked-client-126");

  packed = unpack(exports.ws_write_frame_header(2, 0x80, 0x00010000, 0, 0, 0, headerPtr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 10 });
  assert.deepStrictEqual(Array.from(memory.slice(headerPtr, headerPtr + 10)), [
    0x82, 0x7f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
  ]);
  assert.strictEqual(exports.ws_parse_header(headerPtr, 10, 0x00010000, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr + 16, true), 0x00010000);
  assert.strictEqual(view.getUint32(outPtr + 20, true), 0);
  assert.strictEqual(view.getUint32(outPtr + 24, true), 10);
  mark("write-and-parse-127");

  assert.deepStrictEqual(unpack(exports.ws_write_frame_header(9, 0x00, 1, 0, 0, 0, headerPtr, 16)), {
    status: 3,
    value: 0,
  });
  assert.deepStrictEqual(unpack(exports.ws_write_frame_header(1, 0x80, 5, 0, 1, 0x11223344, headerPtr, 5)), {
    status: 2,
    value: 0,
  });
  mark("reject-general-header-invalid-and-short-output");

  const reason = Buffer.from("normal", "utf8");
  memory.set(reason, payloadPtr);
  packed = unpack(exports.ws_write_close_payload(1000, payloadPtr, reason.length, headerPtr, 16));
  assert.deepStrictEqual(packed, { status: 0, value: 8 });
  assert.deepStrictEqual(Array.from(memory.slice(headerPtr, headerPtr + 8)), [
    0x03, 0xe8, 0x6e, 0x6f, 0x72, 0x6d, 0x61, 0x6c,
  ]);
  assert.strictEqual(exports.ws_parse_close_payload(headerPtr, 8, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 1);
  assert.strictEqual(view.getUint32(outPtr + 4, true), 1000);
  assert.strictEqual(view.getUint32(outPtr + 8, true), 6);
  assert.strictEqual(exports.ws_parse_close_payload(headerPtr, 1, outPtr), 0);
  assert.strictEqual(view.getUint32(outPtr, true), 0);
  assert.deepStrictEqual(unpack(exports.ws_write_close_payload(1000, payloadPtr, 124, headerPtr, 128)), {
    status: 3,
    value: 0,
  });
  mark("close-payload-parse-and-write");

  console.log(
    JSON.stringify(
      {
        unit: "ws-frame",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        cases,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
