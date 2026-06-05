#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/wayland-wire-core.wat";

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;
const STATUS_INCOMPLETE = 4;
const STATUS_TOO_LARGE = 5;
const STATUS_UNALIGNED = 6;

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

function writeHeader(memory, ptr, objectId, opcode, size) {
  const view = new DataView(memory.buffer);
  view.setUint32(ptr, objectId >>> 0, true);
  view.setUint16(ptr + 4, opcode, true);
  view.setUint16(ptr + 6, size, true);
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300108);

  assert.strictEqual(e.wayland_align4(0), 0);
  assert.strictEqual(e.wayland_align4(1), 4);
  assert.strictEqual(e.wayland_align4(5), 8);
  assert.strictEqual(e.wayland_align4(8), 8);

  assert.strictEqual(e.wayland_object_id_status(1, 0), STATUS_OK);
  assert.strictEqual(e.wayland_object_id_status(0, 1), STATUS_OK);
  assert.strictEqual(e.wayland_object_id_status(0, 0), STATUS_INVALID);

  writeHeader(memory, inPtr, 42, 3, 16);
  assert.strictEqual(e.wayland_header_decode(inPtr, 16, outPtr), STATUS_OK);
  assert.strictEqual(u32(view, outPtr), 42);
  assert.strictEqual(u32(view, outPtr + 4), 3);
  assert.strictEqual(u32(view, outPtr + 8), 16);
  assert.strictEqual(u32(view, outPtr + 12), 8);
  assert.strictEqual(u32(view, outPtr + 16), 0x00100003);
  assert.strictEqual(u32(view, outPtr + 20), STATUS_OK);
  assert.strictEqual(u32(view, outPtr + 24), STATUS_OK);
  assert.strictEqual(u32(view, outPtr + 28), 1);

  assert.strictEqual(e.wayland_header_decode(inPtr, 7, outPtr), STATUS_INPUT_SHORT);
  writeHeader(memory, inPtr, 1, 0, 7);
  assert.strictEqual(e.wayland_header_decode(inPtr, 8, outPtr), STATUS_INVALID);
  writeHeader(memory, inPtr, 1, 0, 12);
  assert.strictEqual(e.wayland_header_decode(inPtr, 8, outPtr), STATUS_INCOMPLETE);
  writeHeader(memory, inPtr, 1, 0, 10);
  assert.strictEqual(e.wayland_header_decode(inPtr, 10, outPtr), STATUS_UNALIGNED);
  writeHeader(memory, inPtr, 0, 0, 8);
  assert.strictEqual(e.wayland_header_decode(inPtr, 8, outPtr), STATUS_INVALID);

  let packed = unpack(e.wayland_header_encode(7, 5, 20, outPtr, 8));
  assert.deepStrictEqual(packed, { status: STATUS_OK, value: 8 });
  assert.strictEqual(u32(view, outPtr), 7);
  assert.strictEqual(view.getUint16(outPtr + 4, true), 5);
  assert.strictEqual(view.getUint16(outPtr + 6, true), 20);
  assert.deepStrictEqual(unpack(e.wayland_header_encode(7, 5, 20, outPtr, 7)), {
    status: STATUS_OUTPUT_SHORT,
    value: 0,
  });
  assert.deepStrictEqual(unpack(e.wayland_header_encode(0, 5, 20, outPtr, 8)), {
    status: STATUS_INVALID,
    value: 0,
  });
  assert.deepStrictEqual(unpack(e.wayland_header_encode(7, 5, 10, outPtr, 8)), {
    status: STATUS_UNALIGNED,
    value: 0,
  });

  assert.strictEqual(e.wayland_interface_classify(1), 1);
  assert.strictEqual(e.wayland_interface_classify(11), 11);
  assert.strictEqual(e.wayland_interface_classify(99), 0);
  assert.strictEqual(e.wayland_message_classify(1, 0, 1), 0x00010001);
  assert.strictEqual(e.wayland_message_classify(1, 1, 1), 0x00010101);
  assert.strictEqual(e.wayland_message_classify(4, 0, 3), 0x00040003);
  assert.strictEqual(e.wayland_message_classify(4, 1, 1), 0x00040101);
  assert.strictEqual(e.wayland_message_classify(6, 1, 8), 0x00060108);
  assert.strictEqual(e.wayland_message_classify(11, 1, 5), 0x000b0105);
  assert.strictEqual(e.wayland_message_classify(13, 0, 6), 0x000d0006);
  assert.strictEqual(e.wayland_message_classify(13, 1, 5), 0x000d0105);
  assert.strictEqual(e.wayland_message_status(12, 1, 0), STATUS_INVALID);
  assert.strictEqual(e.wayland_message_status(12, 0, 1), STATUS_OK);
  assert.strictEqual(e.wayland_message_status(5, 1, 6), STATUS_INVALID);

  assert.strictEqual(e.wayland_seat_capabilities_status(1 | 2 | 4), STATUS_OK);
  assert.strictEqual(e.wayland_seat_capabilities_status(8), STATUS_INVALID);
  assert.strictEqual(e.wayland_seat_capabilities_classify(1 | 4), 5);
  assert.strictEqual(e.wayland_seat_capabilities_classify(16), 0);
  assert.strictEqual(e.wayland_key_state_status(0), STATUS_OK);
  assert.strictEqual(e.wayland_key_state_status(1), STATUS_OK);
  assert.strictEqual(e.wayland_key_state_status(2), STATUS_INVALID);
  assert.strictEqual(e.wayland_button_state_status(1), STATUS_OK);
  assert.strictEqual(e.wayland_axis_source_status(3), STATUS_OK);
  assert.strictEqual(e.wayland_axis_source_status(4), STATUS_INVALID);
  assert.strictEqual(e.wayland_dnd_action_status(1 | 2 | 4), STATUS_OK);
  assert.strictEqual(e.wayland_dnd_action_status(8), STATUS_INVALID);
  assert.strictEqual(e.wayland_dnd_preferred_action_status(4), STATUS_OK);
  assert.strictEqual(e.wayland_dnd_preferred_action_status(3), STATUS_INVALID);
  assert.strictEqual(e.wayland_text_change_cause_status(0), STATUS_OK);
  assert.strictEqual(e.wayland_text_change_cause_status(1), STATUS_OK);
  assert.strictEqual(e.wayland_text_change_cause_status(2), STATUS_INVALID);

  memory.fill(0, inPtr, inPtr + 64);
  view.setUint32(inPtr, 6, true);
  memory.set([0x68, 0x65, 0x6c, 0x6c, 0x6f, 0], inPtr + 4);
  assert.strictEqual(e.wayland_string_arg_status(inPtr, 12, 0), STATUS_OK);
  memory[inPtr + 9] = 0x21;
  assert.strictEqual(e.wayland_string_arg_status(inPtr, 12, 0), STATUS_INVALID);
  memory[inPtr + 9] = 0;
  assert.strictEqual(e.wayland_string_arg_status(inPtr, 8, 0), STATUS_INPUT_SHORT);
  view.setUint32(inPtr, 0, true);
  assert.strictEqual(e.wayland_string_arg_status(inPtr, 4, 0), STATUS_OK);

  view.setUint32(inPtr, 5, true);
  memory.set([1, 2, 3, 4, 5], inPtr + 4);
  assert.strictEqual(e.wayland_array_arg_status(inPtr, 12, 0), STATUS_OK);
  assert.strictEqual(e.wayland_array_arg_status(inPtr, 10, 0), STATUS_INPUT_SHORT);
  assert.strictEqual(e.wayland_array_arg_status(inPtr, 3, 0), STATUS_INPUT_SHORT);

  console.log(JSON.stringify({
    unit: "wayland-wire-core",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
