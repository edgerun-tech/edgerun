#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/der-tlv.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  return execFileSync("wat2wasm", [file, "-o", "-"]);
}

function low16(value) {
  return Number(value & 0xffffn);
}

function derLenParts(value) {
  return {
    status: Number(value & 0xffffn),
    consumed: Number((value >> 16n) & 0xffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function status16(value) {
  return Number(value & 0xffffn);
}

function low32High32(value) {
  return {
    status: Number(value & 0xffffffffn),
    written: Number((value >> 32n) & 0xffffffffn),
  };
}

function derHeaderParts(value) {
  return {
    status: Number(value & 0xffffn),
    consumed: Number((value >> 16n) & 0xffffn),
    tag: Number((value >> 32n) & 0xffn),
    length: Number((value >> 40n) & 0xffffffn),
  };
}

function write(memory, bytes, ptr = 1024) {
  memory.fill(0, ptr, ptr + 16);
  memory.set(bytes, ptr);
  return ptr;
}

function expectLenDecode(exports, memory, bytes, expected) {
  const ptr = write(memory, bytes);
  assert.deepStrictEqual(derLenParts(exports.der_len_decode(ptr, bytes.length)), expected);
}

function expectLenStatus(exports, memory, bytes, status) {
  const ptr = write(memory, bytes);
  assert.strictEqual(status16(exports.der_len_decode(ptr, bytes.length)), status);
}

function expectHeaderStatus(exports, memory, bytes, status) {
  const ptr = write(memory, bytes);
  assert.strictEqual(status16(exports.der_header_decode(ptr, bytes.length)), status);
}

function expectEncode(exports, memory, value, expected) {
  const packed = exports.der_len_encode(value, 2048, 8);
  assert.deepStrictEqual(low32High32(packed), {
    status: 0,
    written: expected.length,
  });
  assert.deepStrictEqual(Array.from(memory.slice(2048, 2048 + expected.length)), expected);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300005);

  expectLenStatus(exports, memory, [], 1);
  expectLenStatus(exports, memory, [0x81], 1);
  expectLenStatus(exports, memory, [0x82, 0x01], 1);
  expectLenStatus(exports, memory, [0x80], 3);
  expectLenStatus(exports, memory, [0x85, 0x01, 0x00, 0x00, 0x00, 0x00], 3);
  expectLenStatus(exports, memory, [0x81, 0x7f], 3);
  expectLenStatus(exports, memory, [0x82, 0x00, 0x80], 3);
  expectLenStatus(exports, memory, [0x82, 0x00, 0xff], 3);
  expectLenStatus(exports, memory, [0x83, 0x00, 0x01, 0x00], 3);
  expectLenStatus(exports, memory, [0x84, 0x10, 0x00, 0x00, 0x00], 4);

  for (const [value, bytes] of [
    [0x7f, [0x7f]],
    [0x80, [0x81, 0x80]],
    [0xff, [0x81, 0xff]],
    [0x100, [0x82, 0x01, 0x00]],
    [0xffff, [0x82, 0xff, 0xff]],
    [0x10000, [0x83, 0x01, 0x00, 0x00]],
  ]) {
    expectLenDecode(exports, memory, bytes, {
      status: 0,
      consumed: bytes.length,
      value,
    });
    expectEncode(exports, memory, value, bytes);
  }

  expectEncode(exports, memory, 0x0fffffff, [0x84, 0x0f, 0xff, 0xff, 0xff]);
  assert.deepStrictEqual(low32High32(exports.der_len_encode(0x10000000, 2048, 8)), {
    status: 4,
    written: 0,
  });

  assert.deepStrictEqual(low32High32(exports.der_len_encode(0x7f, 2048, 0)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.der_len_encode(0x80, 2048, 1)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.der_len_encode(0x100, 2048, 2)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(low32High32(exports.der_len_encode(0x10000, 2048, 3)), {
    status: 2,
    written: 0,
  });

  write(memory, [0x30, 0x03, 0x01, 0x01, 0xff]);
  assert.deepStrictEqual(derHeaderParts(exports.der_header_decode(1024, 5)), {
    status: 0,
    consumed: 2,
    tag: 0x30,
    length: 3,
  });

  write(memory, [0x30, 0x83, 0xff, 0xff, 0xff]);
  assert.deepStrictEqual(derHeaderParts(exports.der_header_decode(1024, 5)), {
    status: 0,
    consumed: 5,
    tag: 0x30,
    length: 0xffffff,
  });
  expectHeaderStatus(exports, memory, [], 1);
  expectHeaderStatus(exports, memory, [0x30], 1);
  expectHeaderStatus(exports, memory, [0x30, 0x80], 3);
  expectHeaderStatus(exports, memory, [0x30, 0x82, 0x01], 1);
  expectHeaderStatus(exports, memory, [0x30, 0x82, 0x00, 0x80], 3);
  expectHeaderStatus(exports, memory, [0x30, 0x84, 0x01, 0x00, 0x00, 0x00], 4);

  write(memory, [0x30]);
  assert.deepStrictEqual(low32High32(exports.der_tag_decode(1024, 5)), {
    status: 0,
    written: 0x30,
  });
  for (const tag of [0x01, 0x02, 0x06, 0x0c, 0x31, 0x40, 0x7e, 0x80, 0xbe, 0xc0, 0xfe]) {
    write(memory, [tag]);
    assert.deepStrictEqual(low32High32(exports.der_tag_decode(1024, 1)), {
      status: 0,
      written: tag,
    });
  }

  for (const tag of [0x00, 0x07, 0x08, 0x0b, 0x1f, 0x3f, 0x7f, 0xbf, 0xff]) {
    write(memory, [tag]);
    assert.strictEqual(low32High32(exports.der_tag_decode(1024, 1)).status, 3);
    expectHeaderStatus(exports, memory, [tag, 0x00], 3);
  }
  assert.strictEqual(low32High32(exports.der_tag_decode(1024, 0)).status, 1);

  const result = {
    unit: "der-tlv",
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [
      "short-input",
      "indefinite-reject",
      "minimal-long-length",
      "packed-header-limit",
      "supported-tag-octets",
      "high-tag-number-reject",
      "len-encode-capacity",
      "boundary-lengths",
    ],
  };
  console.log(JSON.stringify(result, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
