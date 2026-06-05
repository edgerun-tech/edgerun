#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/x509-name-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function len(n) {
  if (n < 0x80) return [n];
  if (n <= 0xff) return [0x81, n];
  return [0x82, (n >>> 8) & 0xff, n & 0xff];
}

function tlv(tag, value) {
  return [tag, ...len(value.length), ...value];
}

function seq(...children) {
  return tlv(0x30, children.flat());
}

function set(...children) {
  return tlv(0x31, children.flat());
}

function oid(bytes) {
  return tlv(0x06, bytes);
}

function utf8(value) {
  return tlv(0x0c, [...Buffer.from(value, "utf8")]);
}

function printable(value) {
  return tlv(0x13, [...Buffer.from(value, "ascii")]);
}

function attr(oidBytes, valueTlv) {
  return seq(oid(oidBytes), valueTlv);
}

function name(...rdns) {
  return seq(...rdns);
}

function write(memory, data, ptr = 1024) {
  memory.fill(0, ptr, ptr + 4096);
  memory.set(data, ptr);
  return ptr;
}

function u32(memory, offset) {
  return (
    memory[offset] |
    (memory[offset + 1] << 8) |
    (memory[offset + 2] << 16) |
    (memory[offset + 3] << 24)
  ) >>> 0;
}

function bytes(memory, ptr, len) {
  return Array.from(memory.slice(ptr, ptr + len));
}

function record(memory, out = 8192) {
  return {
    nameBodyPtr: u32(memory, out),
    nameBodyLen: u32(memory, out + 4),
    nameHeaderLen: u32(memory, out + 8),
    nameTotalLen: u32(memory, out + 12),
    rdnCount: u32(memory, out + 16),
    attributeCount: u32(memory, out + 20),
    commonNameValuePtr: u32(memory, out + 24),
    commonNameValueLen: u32(memory, out + 28),
    commonNameValueTag: u32(memory, out + 32),
    commonNameTlvTotalLen: u32(memory, out + 36),
    organizationValuePtr: u32(memory, out + 40),
    organizationValueLen: u32(memory, out + 44),
    organizationValueTag: u32(memory, out + 48),
    organizationTlvTotalLen: u32(memory, out + 52),
  };
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300030);

  const cn = "edge.example";
  const org = "EdgeRun";
  const fixture = name(
    set(attr([0x55, 0x04, 0x03], utf8(cn))),
    set(
      attr([0x55, 0x04, 0x0a], printable(org)),
      attr([0x55, 0x04, 0x0b], utf8("Protocol"))
    )
  );
  let ptr = write(memory, fixture);
  assert.strictEqual(exports.x509_name_scan(ptr, fixture.length, 8192), 0);
  let out = record(memory);
  assert.strictEqual(out.nameBodyPtr, ptr + out.nameHeaderLen);
  assert.strictEqual(out.nameTotalLen, fixture.length);
  assert.strictEqual(out.rdnCount, 2);
  assert.strictEqual(out.attributeCount, 3);
  assert.strictEqual(out.commonNameValueTag, 0x0c);
  assert.strictEqual(out.organizationValueTag, 0x13);
  assert.deepStrictEqual(bytes(memory, out.commonNameValuePtr, out.commonNameValueLen), [
    ...Buffer.from(cn, "utf8"),
  ]);
  assert.deepStrictEqual(bytes(memory, out.organizationValuePtr, out.organizationValueLen), [
    ...Buffer.from(org, "ascii"),
  ]);
  assert.strictEqual(out.commonNameTlvTotalLen, cn.length + 2);
  assert.strictEqual(out.organizationTlvTotalLen, org.length + 2);

  const empty = name();
  ptr = write(memory, empty);
  assert.strictEqual(exports.x509_name_scan(ptr, empty.length, 8192), 0);
  out = record(memory);
  assert.strictEqual(out.rdnCount, 0);
  assert.strictEqual(out.attributeCount, 0);
  assert.strictEqual(out.commonNameValueLen, 0);
  assert.strictEqual(out.organizationValueLen, 0);

  const repeated = name(
    set(attr([0x55, 0x04, 0x03], utf8("first"))),
    set(attr([0x55, 0x04, 0x03], utf8("second")))
  );
  ptr = write(memory, repeated);
  assert.strictEqual(exports.x509_name_scan(ptr, repeated.length, 8192), 0);
  out = record(memory);
  assert.deepStrictEqual(bytes(memory, out.commonNameValuePtr, out.commonNameValueLen), [
    ...Buffer.from("first", "utf8"),
  ]);

  assert.strictEqual(exports.x509_name_scan(write(memory, [0x30, 0x80, 0x00, 0x00]), 4, 8192), 3);
  assert.strictEqual(exports.x509_name_scan(write(memory, fixture), fixture.length - 1, 8192), 1);
  assert.strictEqual(exports.x509_name_scan(write(memory, [...fixture, 0x00]), fixture.length + 1, 8192), 3);
  assert.strictEqual(exports.x509_name_scan(write(memory, [0x31, 0x00]), 2, 8192), 3);
  assert.strictEqual(exports.x509_name_scan(write(memory, name(set())), name(set()).length, 8192), 3);

  const missingValue = name(set(seq(oid([0x55, 0x04, 0x03]))));
  assert.strictEqual(exports.x509_name_scan(write(memory, missingValue), missingValue.length, 8192), 3);

  const trailingInAttribute = name(set(seq(oid([0x55, 0x04, 0x03]), utf8("x"), [0x05, 0x00])));
  assert.strictEqual(
    exports.x509_name_scan(write(memory, trailingInAttribute), trailingInAttribute.length, 8192),
    3
  );

  const badOidTag = name(set(seq(tlv(0x05, []), utf8("x"))));
  assert.strictEqual(exports.x509_name_scan(write(memory, badOidTag), badOidTag.length, 8192), 3);

  const badOidRoot = name(set(attr([0x78], utf8("x"))));
  assert.strictEqual(exports.x509_name_scan(write(memory, badOidRoot), badOidRoot.length, 8192), 3);

  const unterminatedOidArc = name(set(attr([0x55, 0x04, 0x80], utf8("x"))));
  assert.strictEqual(
    exports.x509_name_scan(write(memory, unterminatedOidArc), unterminatedOidArc.length, 8192),
    1
  );

  const nonMinimalOidArc = name(set(attr([0x55, 0x04, 0x80, 0x00], utf8("x"))));
  assert.strictEqual(
    exports.x509_name_scan(write(memory, nonMinimalOidArc), nonMinimalOidArc.length, 8192),
    3
  );

  const unsupportedValue = name(set(attr([0x55, 0x04, 0x03], tlv(0x05, []))));
  assert.strictEqual(
    exports.x509_name_scan(write(memory, unsupportedValue), unsupportedValue.length, 8192),
    3
  );

  console.log(JSON.stringify({
    unit: "x509-name-core",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
