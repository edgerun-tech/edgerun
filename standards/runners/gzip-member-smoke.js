#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/gzip-member.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function packReturn(value) {
  const n = BigInt.asUintN(64, value);
  return {
    status: Number(n & 0xffffffffn),
    written: Number(n >> 32n),
  };
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function u8(...bytes) {
  return Uint8Array.from(bytes);
}

function concat(...chunks) {
  const len = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const out = new Uint8Array(len);
  let pos = 0;
  for (const chunk of chunks) {
    out.set(chunk, pos);
    pos += chunk.length;
  }
  return out;
}

function le32(value) {
  return u8(value, value >>> 8, value >>> 16, value >>> 24);
}

function text(value) {
  return Buffer.from(value, "ascii");
}

function baseHeader(flags = 0, mtime = 0, xfl = 0, osByte = 255) {
  return u8(
    0x1f,
    0x8b,
    0x08,
    flags,
    mtime,
    mtime >>> 8,
    mtime >>> 16,
    mtime >>> 24,
    xfl,
    osByte,
  );
}

function member({ flags = 0, extra = null, name = null, comment = null, hcrc = false, body = u8(), crc = 0, isize = 0 }) {
  const chunks = [baseHeader(flags)];
  if (extra) {
    chunks.push(u8(extra.length, extra.length >>> 8), extra);
  }
  if (name !== null) {
    chunks.push(text(name), u8(0));
  }
  if (comment !== null) {
    chunks.push(text(comment), u8(0));
  }
  if (hcrc) {
    chunks.push(u8(0x34, 0x12));
  }
  chunks.push(body, le32(crc), le32(isize));
  return concat(...chunks);
}

function record(view, outPtr) {
  return {
    deflateOff: view.getUint32(outPtr, true),
    deflateLen: view.getUint32(outPtr + 4, true),
    flags: view.getUint32(outPtr + 8, true),
    mtime: view.getUint32(outPtr + 12, true),
    xfl: view.getUint32(outPtr + 16, true),
    os: view.getUint32(outPtr + 20, true),
    expectedCrc32: view.getUint32(outPtr + 24, true),
    expectedIsize: view.getUint32(outPtr + 28, true),
    headerLen: view.getUint32(outPtr + 32, true),
    trailerOff: view.getUint32(outPtr + 36, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300064);

  const inPtr = 1024;
  const outPtr = 4096;

  let len = write(memory, inPtr, member({
    body: u8(0x03, 0x00),
    crc: 0x78563412,
    isize: 0,
  }));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    deflateOff: 10,
    deflateLen: 2,
    flags: 0,
    mtime: 0,
    xfl: 0,
    os: 255,
    expectedCrc32: 0x78563412,
    expectedIsize: 0,
    headerLen: 10,
    trailerOff: 12,
  });

  len = write(memory, inPtr, member({
    flags: 0x04 | 0x08 | 0x10 | 0x02,
    extra: u8(0xaa, 0xbb),
    name: "file.txt",
    comment: "ok",
    hcrc: true,
    body: u8(0x01, 0x02, 0x03),
    crc: 0xaabbccdd,
    isize: 3,
  }));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 0);
  assert.deepStrictEqual(record(view, outPtr), {
    deflateOff: 28,
    deflateLen: 3,
    flags: 0x1e,
    mtime: 0,
    xfl: 0,
    os: 255,
    expectedCrc32: 0xaabbccdd,
    expectedIsize: 3,
    headerLen: 28,
    trailerOff: 31,
  });

  len = write(memory, inPtr, concat(baseHeader(0xe0), u8(0, 0, 0, 0, 0, 0, 0, 0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 3);

  len = write(memory, inPtr, concat(baseHeader(0x04), u8(4, 0, 1), le32(0), le32(0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 1);

  len = write(memory, inPtr, concat(baseHeader(0x08), text("unterminated"), le32(0), le32(0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 1);

  len = write(memory, inPtr, concat(baseHeader(0x10), text("unterminated"), le32(0), le32(0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 1);

  len = write(memory, inPtr, concat(baseHeader(0x02), u8(0), le32(0), le32(0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 1);

  len = write(memory, inPtr, concat(baseHeader(0), u8(0x03, 0x00), le32(0)));
  assert.strictEqual(exports.gzip_member_scan(inPtr, len, outPtr), 1);

  assert.deepStrictEqual(packReturn(exports.gzip_member_write_header(outPtr, 9)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(packReturn(exports.gzip_member_write_header(outPtr, 10)), {
    status: 0,
    written: 10,
  });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 10)), [
    0x1f,
    0x8b,
    0x08,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0x00,
    0xff,
  ]);

  assert.deepStrictEqual(packReturn(exports.gzip_member_write_trailer(0x78563412, 0xaabbccdd, outPtr, 7)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(packReturn(exports.gzip_member_write_trailer(0x78563412, 0xaabbccdd, outPtr, 8)), {
    status: 0,
    written: 8,
  });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 8)), [
    0x12,
    0x34,
    0x56,
    0x78,
    0xdd,
    0xcc,
    0xbb,
    0xaa,
  ]);

  console.log(
    JSON.stringify(
      {
        unit: "gzip-member",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
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
