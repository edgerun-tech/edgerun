#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/deflate-inflate.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function record(view, ptr) {
  return {
    consumed: view.getUint32(ptr, true),
    written: view.getUint32(ptr + 4, true),
    blockCount: view.getUint32(ptr + 8, true),
    lastBlockType: view.getUint32(ptr + 12, true),
    crc32: view.getUint32(ptr + 16, true),
    adler32: view.getUint32(ptr + 20, true),
  };
}

function storedBlock(payload, final = true) {
  const len = payload.length;
  const nlen = len ^ 0xffff;
  return Buffer.concat([
    Buffer.from([final ? 1 : 0, len & 0xff, (len >> 8) & 0xff, nlen & 0xff, (nlen >> 8) & 0xff]),
    Buffer.from(payload),
  ]);
}

function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i += 1) {
      crc = (crc & 1) ? ((crc >>> 1) ^ 0xedb88320) : (crc >>> 1);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function adler32(bytes) {
  let s1 = 1;
  let s2 = 0;
  for (const byte of bytes) {
    s1 = (s1 + byte) % 65521;
    s2 = (s2 + s1) % 65521;
  }
  return ((s2 << 16) | s1) >>> 0;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300067);

  const inPtr = 1024;
  const outPtr = 140000;
  const recPtr = 260000;

  let encoded = Buffer.from([0x01, 0x00, 0x00, 0xff, 0xff]);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, encoded.length, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: 5,
    written: 0,
    blockCount: 1,
    lastBlockType: 0,
    crc32: 0,
    adler32: 1,
  });
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, encoded.length, outPtr, 8, 8, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: 5,
    written: 0,
    blockCount: 1,
    lastBlockType: 0,
    crc32: 0,
    adler32: 1,
  });

  const hello = Buffer.from("Hello, zlib!", "ascii");
  encoded = storedBlock(hello);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, encoded.length, recPtr), 0);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: encoded.length,
    written: hello.length,
    blockCount: 1,
    lastBlockType: 0,
    crc32: 0,
    adler32: 1,
  });
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, encoded.length, outPtr, 64, 64, recPtr), 0);
  assert.deepStrictEqual(Buffer.from(memory.slice(outPtr, outPtr + hello.length)), hello);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: encoded.length,
    written: hello.length,
    blockCount: 1,
    lastBlockType: 0,
    crc32: crc32(hello),
    adler32: adler32(hello),
  });

  const one = Buffer.from("first-", "ascii");
  const two = Buffer.from("second", "ascii");
  encoded = Buffer.concat([storedBlock(one, false), storedBlock(two, true)]);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, encoded.length, outPtr, 64, 64, recPtr), 0);
  const joined = Buffer.concat([one, two]);
  assert.deepStrictEqual(Buffer.from(memory.slice(outPtr, outPtr + joined.length)), joined);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: encoded.length,
    written: joined.length,
    blockCount: 2,
    lastBlockType: 0,
    crc32: crc32(joined),
    adler32: adler32(joined),
  });

  write(memory, inPtr, Buffer.from([0x07]));
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, 1, recPtr), 3);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, 1, outPtr, 64, 64, recPtr), 3);

  write(memory, inPtr, Buffer.from([0x01, 0x01, 0x00, 0xff, 0xff, 0x41]));
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, 6, recPtr), 3);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, 6, outPtr, 64, 64, recPtr), 3);

  encoded = storedBlock(hello);
  write(memory, inPtr, encoded);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, encoded.length, outPtr, hello.length - 1, hello.length, recPtr), 2);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, encoded.length, outPtr, hello.length, hello.length - 1, recPtr), 6);

  const fixedSample = Buffer.from("73494dcb492c4955001100", "hex");
  const fixedText = Buffer.from("Deflate late", "ascii");
  write(memory, inPtr, fixedSample);
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, fixedSample.length, recPtr), 7);
  assert.strictEqual(record(view, recPtr).lastBlockType, 1);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, fixedSample.length, outPtr, 64, 64, recPtr), 0);
  assert.deepStrictEqual(Buffer.from(memory.slice(outPtr, outPtr + fixedText.length)), fixedText);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: fixedSample.length,
    written: fixedText.length,
    blockCount: 1,
    lastBlockType: 1,
    crc32: crc32(fixedText),
    adler32: adler32(fixedText),
  });

  const fixedCopy = Buffer.from("4b4c4a4e444500", "hex");
  const fixedCopyText = Buffer.from("abcabcabcabcabcabc", "ascii");
  write(memory, inPtr, fixedCopy);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, fixedCopy.length, outPtr, 64, 64, recPtr), 0);
  assert.deepStrictEqual(Buffer.from(memory.slice(outPtr, outPtr + fixedCopyText.length)), fixedCopyText);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: fixedCopy.length,
    written: fixedCopyText.length,
    blockCount: 1,
    lastBlockType: 1,
    crc32: crc32(fixedCopyText),
    adler32: adler32(fixedCopyText),
  });

  write(memory, inPtr, fixedSample.subarray(0, fixedSample.length - 1));
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, fixedSample.length - 1, outPtr, 64, 64, recPtr), 1);
  write(memory, inPtr, fixedSample);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, fixedSample.length, outPtr, fixedText.length - 1, fixedText.length, recPtr), 2);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, fixedSample.length, outPtr, fixedText.length, fixedText.length - 1, recPtr), 6);

  write(memory, inPtr, Buffer.from([0x03]));
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, 1, recPtr), 7);
  assert.strictEqual(record(view, recPtr).lastBlockType, 1);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, 1, outPtr, 64, 64, recPtr), 1);

  const dynamicSample = Buffer.from(
    "cdcb470100410803404b942d202790c5bf84b371f31fbb05768673f486d491b6f241d2a10991b70ed037636d1d99576c6c4b9301c8b19ffc0f",
    "hex",
  );
  const dynamicText = Buffer.from(
    "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc98",
    "ascii",
  );
  write(memory, inPtr, dynamicSample);
  assert.strictEqual(exports.deflate_scan_blocks(inPtr, dynamicSample.length, recPtr), 7);
  assert.strictEqual(record(view, recPtr).lastBlockType, 2);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, dynamicSample.length, outPtr, 256, 256, recPtr), 0);
  assert.deepStrictEqual(Buffer.from(memory.slice(outPtr, outPtr + dynamicText.length)), dynamicText);
  assert.deepStrictEqual(record(view, recPtr), {
    consumed: dynamicSample.length,
    written: dynamicText.length,
    blockCount: 1,
    lastBlockType: 2,
    crc32: crc32(dynamicText),
    adler32: adler32(dynamicText),
  });

  const corruptDynamic = Buffer.from(dynamicSample);
  corruptDynamic[1] ^= 0x02;
  write(memory, inPtr, corruptDynamic);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, corruptDynamic.length, outPtr, 256, 256, recPtr), 3);

  write(memory, inPtr, dynamicSample.subarray(0, dynamicSample.length - 1));
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, dynamicSample.length - 1, outPtr, 256, 256, recPtr), 1);
  write(memory, inPtr, dynamicSample);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, dynamicSample.length, outPtr, dynamicText.length - 1, dynamicText.length, recPtr), 2);
  assert.strictEqual(exports.deflate_inflate_raw(inPtr, dynamicSample.length, outPtr, dynamicText.length, dynamicText.length - 1, recPtr), 6);

  console.log(
    JSON.stringify(
      {
        unit: "deflate-inflate",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        fixed_huffman: "done",
        dynamic_huffman: "done",
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
