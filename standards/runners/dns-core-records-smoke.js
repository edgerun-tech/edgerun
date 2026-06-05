#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dns-core-records.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function unpack(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function writeBytes(memory, offset, bytes) {
  memory.fill(0, offset, offset + Math.max(bytes.length, 256));
  memory.set(Uint8Array.from(bytes), offset);
  return bytes.length;
}

function u16(v) {
  return [(v >> 8) & 0xff, v & 0xff];
}

function u32(v) {
  return [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
}

function wireName(labels) {
  const out = [];
  for (const label of labels) {
    const bytes = Buffer.from(label, "ascii");
    out.push(bytes.length, ...bytes);
  }
  out.push(0);
  return out;
}

function readU32(view, offset) {
  return view.getUint32(offset, true);
}

function words(view, offset, count) {
  return Array.from({ length: count }, (_, i) => readU32(view, offset + i * 4));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300101);

  for (const type of [
    1, 2, 5, 6, 12, 13, 15, 16, 17, 18, 28, 29, 33, 35, 41,
    43, 46, 47, 48, 50, 52, 64, 65, 99, 250, 252, 255, 256, 257,
  ]) {
    assert.equal(e.dns_record_type_status(type), 0, `known type ${type}`);
  }
  assert.equal(e.dns_record_type_status(9999), 3);
  assert.equal(e.dns_record_type_family(1), 1);
  assert.equal(e.dns_record_type_family(5), 2);
  assert.equal(e.dns_record_type_family(16), 3);
  assert.equal(e.dns_record_type_family(65), 4);
  assert.equal(e.dns_record_type_family(48), 5);
  assert.equal(e.dns_record_type_family(41), 6);
  assert.equal(e.dns_record_type_family(6), 7);
  assert.equal(e.dns_record_type_family(9999), 0);

  assert.equal(e.dns_class_status(1, 1), 0);
  assert.equal(e.dns_class_status(255, 1), 0);
  assert.equal(e.dns_class_status(3, 1), 3);
  assert.equal(e.dns_class_status(4096, 41), 0);
  assert.equal(e.dns_class_status(511, 41), 3);
  assert.equal(e.dns_opcode_status(5), 0);
  assert.equal(e.dns_opcode_status(3), 3);
  assert.equal(e.dns_rcode_status(10), 0);
  assert.equal(e.dns_rcode_status(11), 3);

  let packed = unpack(e.dns_header_flags_pack(1, 0, 1, 0, 1, 1, 3));
  assert.deepEqual(packed, { status: 0, value: 0x8583 });
  assert.equal(e.dns_header_flags_unpack(0x8583, outPtr, 28), 0);
  assert.deepEqual(words(view, outPtr, 7), [1, 0, 1, 0, 1, 1, 3]);
  assert.deepEqual(unpack(e.dns_header_flags_pack(1, 3, 0, 0, 0, 0, 0)), {
    status: 3,
    value: 0,
  });
  assert.equal(e.dns_header_flags_unpack(0x1800, outPtr, 28), 3);
  assert.equal(e.dns_header_flags_unpack(0x8583, outPtr, 24), 2);

  let len = writeBytes(memory, inPtr, Buffer.from("Edge_01", "ascii"));
  assert.equal(e.dns_label_validate(inPtr, len), 0);
  len = writeBytes(memory, inPtr, Buffer.from("-bad", "ascii"));
  assert.equal(e.dns_label_validate(inPtr, len), 3);
  len = writeBytes(memory, inPtr, Buffer.from("bad-", "ascii"));
  assert.equal(e.dns_label_validate(inPtr, len), 3);
  len = writeBytes(memory, inPtr, Buffer.from("bad.name", "ascii"));
  assert.equal(e.dns_label_validate(inPtr, len), 3);
  len = writeBytes(memory, inPtr, Buffer.from("a".repeat(64), "ascii"));
  assert.equal(e.dns_label_validate(inPtr, len), 3);

  len = writeBytes(memory, inPtr, [0]);
  assert.equal(e.dns_name_scan(inPtr, len, outPtr, 64), 0);
  assert.deepEqual(words(view, outPtr, 3), [1, 0, 0]);
  len = writeBytes(memory, inPtr, wireName(["WWW", "Example", "COM"]));
  assert.equal(e.dns_name_scan(inPtr, len, outPtr, 64), 0);
  assert.deepEqual(words(view, outPtr, 9), [
    len, 3, 15,
    1, 3,
    5, 7,
    13, 3,
  ]);
  len = writeBytes(memory, inPtr, [0xc0, 0x0c]);
  assert.equal(e.dns_name_scan(inPtr, len, outPtr, 64), 3);
  len = writeBytes(memory, inPtr, [3, 0x62, 0x61]);
  assert.equal(e.dns_name_scan(inPtr, len, outPtr, 64), 1);
  len = writeBytes(memory, inPtr, wireName(Array.from({ length: 128 }, () => "a")));
  assert.equal(e.dns_name_scan(inPtr, len, outPtr, 2048), 6);

  assert.equal(e.dnssec_algorithm_status(8), 0);
  assert.equal(e.dnssec_algorithm_status(13), 0);
  assert.equal(e.dnssec_algorithm_status(15), 0);
  assert.equal(e.dnssec_algorithm_status(16), 0);
  assert.equal(e.dnssec_algorithm_status(253), 6);
  assert.equal(e.dnssec_digest_status(1), 0);
  assert.equal(e.dnssec_digest_status(2), 0);
  assert.equal(e.dnssec_digest_status(4), 0);
  assert.equal(e.dnssec_digest_status(3), 5);
  for (const status of [0, 1, 2, 3, 4, 5, 6]) {
    assert.equal(e.dnssec_result_status(status), 0);
  }
  assert.equal(e.dnssec_result_status(7), 3);
  assert.equal(e.dnssec_dnskey_header_status(3, 13, 64), 0);
  assert.equal(e.dnssec_dnskey_header_status(2, 13, 64), 3);
  assert.equal(e.dnssec_dnskey_header_status(3, 13, 0), 3);
  assert.equal(e.dnssec_dnskey_header_status(3, 99, 64), 6);
  assert.equal(e.dnssec_ds_digest_length_status(1, 20), 0);
  assert.equal(e.dnssec_ds_digest_length_status(2, 32), 0);
  assert.equal(e.dnssec_ds_digest_length_status(4, 48), 0);
  assert.equal(e.dnssec_ds_digest_length_status(2, 31), 3);
  assert.equal(e.dnssec_ds_digest_length_status(3, 32), 5);

  const rrStart = 5;
  len = writeBytes(memory, inPtr, [
    ...wireName(["www"]),
    ...u16(1),
    ...u16(1),
    ...u32(300),
    ...u16(4),
    192, 0, 2, 1,
  ]);
  assert.equal(e.dns_rr_trailer_decode(inPtr, len, rrStart, outPtr, 32), 0);
  assert.deepEqual(words(view, outPtr, 8), [1, 1, 300, 4, 15, 19, 0, 0]);
  assert.equal(e.dns_rr_trailer_decode(inPtr, len - 1, rrStart, outPtr, 32), 1);
  assert.equal(e.dns_rr_trailer_decode(inPtr, len, rrStart, outPtr, 28), 2);

  len = writeBytes(memory, inPtr, [
    ...wireName(["www"]),
    ...u16(9999),
    ...u16(1),
    ...u32(0xffffffff),
    ...u16(0),
  ]);
  assert.equal(e.dns_rr_trailer_decode(inPtr, len, rrStart, outPtr, 32), 3);
  assert.deepEqual(words(view, outPtr, 8), [9999, 1, 0xffffffff, 0, 15, 15, 3, 0]);

  len = writeBytes(memory, inPtr, [
    0,
    ...u16(41),
    ...u16(1232),
    ...u32(0x8000),
    ...u16(0),
  ]);
  assert.equal(e.dns_rr_trailer_decode(inPtr, len, 1, outPtr, 32), 0);
  assert.deepEqual(words(view, outPtr, 8), [41, 1232, 0x8000, 0, 11, 11, 0, 0]);

  console.log(JSON.stringify({
    unit: "dns-core-records",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
