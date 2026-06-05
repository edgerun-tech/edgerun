#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/sdk-standards-seed.wat");

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

function writeBytes(memory, bytes, ptr) {
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 1));
  memory.set(bytes, ptr);
  return ptr;
}

function writeAscii(memory, value, ptr) {
  return writeBytes(memory, Buffer.from(value, "ascii"), ptr);
}

function readBytes(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

function shape32(bytes) {
  let s0 = 0x811c9dc5 >>> 0;
  let s1 = 0x9e3779b9 >>> 0;
  let s2 = 0x85ebca6b >>> 0;
  let s3 = 0xc2b2ae35 >>> 0;
  for (let i = 0; i < bytes.length; i += 1) {
    const b = bytes[i];
    s0 = (Math.imul(rotl32((s0 ^ b) >>> 0, 5), 16777619) + i) >>> 0;
    s1 = (rotl32((s1 + b) >>> 0, 7) + 0x7f4a7c15) >>> 0;
    s2 = (rotl32(s2, 11) ^ (((b << 16) + i) >>> 0)) >>> 0;
    s3 = (((s3 ^ Math.imul(b, 0x45d9f3b)) >>> 0) + rotl32(s0, 13)) >>> 0;
  }
  const words = [
    s0,
    s1,
    s2,
    s3,
    (s0 ^ s2) >>> 0,
    (s1 ^ s3) >>> 0,
    (rotl32(s0, 17) + s3) >>> 0,
    (rotl32(s1, 3) ^ s2) >>> 0,
  ];
  const out = Buffer.alloc(32);
  words.forEach((word, index) => out.writeUInt32LE(word >>> 0, index * 4));
  return out;
}

function rotl32(value, shift) {
  return ((value << shift) | (value >>> (32 - shift))) >>> 0;
}

function unitMeta(id) {
  const ids = [
    "udp-datagram-definition",
    "tftp-message-definition",
    "udp-rfc768-length-0001",
    "tftp-rfc1350-opcode-0001",
    "tftp-rfc1350-ack-length-0001",
    "tftp-rfc1350-data-length-0001",
  ];
  const index = ids.indexOf(id);
  assert.notStrictEqual(index, -1, id);
  const kind = index <= 1 ? 0 : 1;
  const standard = index === 0 || index === 2 ? 1 : 2;
  const wasmExport = index <= 1 ? 1 : 2;
  const masks = [0, 0, 1, 2, 4, 8];
  return { index, kind, standard, wasmExport, mask: masks[index] };
}

function expectedPreimage(id) {
  const meta = unitMeta(id);
  const prefix = Buffer.from("edgerun-sdk-standards-seed/v1\0", "ascii");
  const out = Buffer.alloc(39 + id.length);
  prefix.copy(out, 0);
  out[30] = meta.index;
  out[31] = meta.kind;
  out[32] = meta.standard;
  out[33] = meta.wasmExport;
  out.writeUInt32LE(meta.mask, 34);
  out[38] = id.length;
  out.write(id, 39, "ascii");
  return out;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300092);

  for (const id of [
    "udp-datagram-definition",
    "tftp-message-definition",
    "udp-rfc768-length-0001",
    "tftp-rfc1350-opcode-0001",
    "tftp-rfc1350-ack-length-0001",
    "tftp-rfc1350-data-length-0001",
  ]) {
    const ptr = writeAscii(memory, id, 1024);
    assert.equal(e.sdk_seed_id_valid(ptr, id.length), 0, id);
    assert.equal(e.sdk_seed_unit_lookup(ptr, id.length), 0, id);
  }

  for (const id of ["", "-bad", "bad-", "bad--id", "Bad", "bad_id"]) {
    const ptr = writeAscii(memory, id, 1024);
    assert.equal(e.sdk_seed_id_valid(ptr, id.length), 3, id);
  }

  for (const ns of ["input.bytes", "tftp.byte_len", "finding:tftp-rfc1350-opcode-0001"]) {
    const ptr = writeAscii(memory, ns, 2048);
    assert.equal(e.sdk_seed_namespace_valid(ptr, ns.length), 0, ns);
  }

  for (const ns of ["", ".input", "input.", "input..bytes", "Input.bytes", "bad space"]) {
    const ptr = writeAscii(memory, ns, 2048);
    assert.equal(e.sdk_seed_namespace_valid(ptr, ns.length), 3, ns);
  }

  let ptr = writeAscii(memory, "not-in-table", 1024);
  assert.equal(e.sdk_seed_unit_lookup(ptr, "not-in-table".length), 1);
  assert.equal(e.sdk_seed_unit_kind(ptr, "not-in-table".length), -1);

  ptr = writeAscii(memory, "tftp-message-definition", 1024);
  assert.equal(e.sdk_seed_unit_kind(ptr, "tftp-message-definition".length), 0);
  assert.equal(e.sdk_seed_graph_member(ptr, "tftp-message-definition".length), 0);

  ptr = writeAscii(memory, "udp-datagram-definition", 1024);
  assert.equal(e.sdk_seed_unit_kind(ptr, "udp-datagram-definition".length), 0);
  assert.equal(e.sdk_seed_graph_member(ptr, "udp-datagram-definition".length), 1);

  ptr = writeAscii(memory, "tftp-rfc1350-opcode-0001", 1024);
  assert.equal(e.sdk_seed_unit_kind(ptr, "tftp-rfc1350-opcode-0001".length), 1);
  assert.equal(e.sdk_seed_graph_member(ptr, "tftp-rfc1350-opcode-0001".length), 0);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-opcode-0001".length, 2, 4, 0, 0), 0);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-opcode-0001".length, 2, 9, 0, 0), 2);

  ptr = writeAscii(memory, "tftp-rfc1350-ack-length-0001", 1024);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-ack-length-0001".length, 4, 4, 0, 0), 0);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-ack-length-0001".length, 5, 4, 0, 0), 2);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-ack-length-0001".length, 5, 3, 0, 0), 0);

  ptr = writeAscii(memory, "tftp-rfc1350-data-length-0001", 1024);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-data-length-0001".length, 4, 3, 0, 0), 0);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "tftp-rfc1350-data-length-0001".length, 3, 3, 0, 0), 2);

  ptr = writeAscii(memory, "udp-rfc768-length-0001", 1024);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "udp-rfc768-length-0001".length, 0, 0, 12, 4), 0);
  assert.equal(e.sdk_seed_clause_table_status(ptr, "udp-rfc768-length-0001".length, 0, 0, 11, 4), 2);

  const id = "tftp-rfc1350-opcode-0001";
  const idPtr = writeAscii(memory, id, 1024);
  let outPtr = 4096;
  let result = packed(e.sdk_seed_unit_preimage(idPtr, id.length, outPtr, 256));
  const expected = expectedPreimage(id);
  assert.deepEqual(result, { status: 0, written: expected.length });
  assert.deepEqual(readBytes(memory, outPtr, result.written), expected);
  assert.deepEqual(packed(e.sdk_seed_unit_preimage(idPtr, id.length, outPtr, 8)), {
    status: 2,
    written: 0,
  });

  outPtr = 5120;
  result = packed(e.sdk_seed_shape32(idPtr, id.length, outPtr, 32));
  assert.deepEqual(result, { status: 0, written: 32 });
  assert.deepEqual(readBytes(memory, outPtr, 32), shape32(Buffer.from(id, "ascii")));
  assert.deepEqual(packed(e.sdk_seed_shape32(idPtr, id.length, outPtr, 31)), {
    status: 2,
    written: 0,
  });

  outPtr = 6144;
  result = packed(e.sdk_seed_unit_shape32(idPtr, id.length, outPtr, 32));
  assert.deepEqual(result, { status: 0, written: 32 });
  assert.deepEqual(readBytes(memory, outPtr, 32), shape32(expected));

  console.log(
    JSON.stringify({
      unit: "sdk-standards-seed",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "id_valid",
        "id_invalid",
        "namespace_valid",
        "namespace_invalid",
        "unit_lookup",
        "unit_kind",
        "graph_member",
        "clause_table_pass_reject",
        "unit_preimage",
        "unit_preimage_output_short",
        "shape32",
        "shape32_output_short",
        "unit_shape32",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
