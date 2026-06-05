#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/zlib-wrapper.wat";
const coreWatPath = "standards/build/wasm/codec-primitives/encoding-core.wat";

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
    written: Number((packed >> 32n) & 0xffffffffn),
  };
}

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
}

function record(view, outPtr) {
  return {
    deflateOff: view.getUint32(outPtr, true),
    deflateLen: view.getUint32(outPtr + 4, true),
    cmf: view.getUint32(outPtr + 8, true),
    flg: view.getUint32(outPtr + 12, true),
    windowLog2: view.getUint32(outPtr + 16, true),
    flevel: view.getUint32(outPtr + 20, true),
    fdict: view.getUint32(outPtr + 24, true),
    expectedAdler32: view.getUint32(outPtr + 28, true),
  };
}

function bytes(memory, ptr, len) {
  return Array.from(memory.slice(ptr, ptr + len));
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const coreModule = await WebAssembly.instantiate(compileWat(coreWatPath), {});
  const e = module.instance.exports;
  const core = coreModule.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const coreMemory = new Uint8Array(core.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  const corePtr = 1024;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300065);

  for (const [level, flg, flevel] of [
    [0, 0x01, 0],
    [1, 0x5e, 1],
    [6, 0x9c, 2],
    [9, 0xda, 3],
  ]) {
    assert.deepStrictEqual(unpack(e.zlib_write_header(level, outPtr, 2)), {
      status: 0,
      written: 2,
    });
    assert.deepStrictEqual(bytes(memory, outPtr, 2), [0x78, flg]);

    write(memory, inPtr, Buffer.from([0x78, flg, 0x01, 0x00, 0x00, 0xff, 0xff, 0x12, 0x34, 0x56, 0x78]));
    assert.strictEqual(e.zlib_member_scan(inPtr, 11, outPtr), 0);
    assert.deepStrictEqual(record(view, outPtr), {
      deflateOff: 2,
      deflateLen: 5,
      cmf: 0x78,
      flg,
      windowLog2: 15,
      flevel,
      fdict: 0,
      expectedAdler32: 0x12345678,
    });
  }

  assert.deepStrictEqual(unpack(e.zlib_write_header(6, outPtr, 1)), {
    status: 2,
    written: 0,
  });
  assert.deepStrictEqual(unpack(e.zlib_write_header(5, outPtr, 2)), {
    status: 3,
    written: 0,
  });

  write(memory, inPtr, Buffer.from([0x77, 0x85, 1, 2, 3, 4]));
  assert.strictEqual(e.zlib_member_scan(inPtr, 6, outPtr), 3, "invalid method");

  write(memory, inPtr, Buffer.from([0x78, 0x02, 1, 2, 3, 4]));
  assert.strictEqual(e.zlib_member_scan(inPtr, 6, outPtr), 3, "invalid fcheck");

  write(memory, inPtr, Buffer.from([0x78, 0x20, 1, 2, 3, 4]));
  assert.strictEqual(e.zlib_member_scan(inPtr, 6, outPtr), 7, "fdict rejected");

  write(memory, inPtr, Buffer.from([0x78, 0x9c, 1, 2, 3]));
  assert.strictEqual(e.zlib_member_scan(inPtr, 5, outPtr), 1, "truncated trailer");

  const payload = Buffer.from("Hello, zlib!", "ascii");
  write(coreMemory, corePtr, payload);
  const adler = core.adler32(corePtr, payload.length) >>> 0;
  assert.strictEqual(adler, 0x1b650413);

  assert.deepStrictEqual(unpack(e.zlib_write_trailer(adler, outPtr, 4)), {
    status: 0,
    written: 4,
  });
  assert.deepStrictEqual(bytes(memory, outPtr, 4), [0x1b, 0x65, 0x04, 0x13]);
  assert.deepStrictEqual(unpack(e.zlib_write_trailer(adler, outPtr, 3)), {
    status: 2,
    written: 0,
  });

  write(memory, inPtr, Buffer.from([0x78, 0x9c, 0x01, 0x00, 0x00, 0xff, 0xff, 0x1b, 0x65, 0x04, 0x13]));
  assert.strictEqual(e.zlib_member_scan(inPtr, 11, outPtr), 0);
  assert.strictEqual(record(view, outPtr).expectedAdler32, adler);

  console.log(
    JSON.stringify(
      {
        unit: "zlib-wrapper",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
        adler32_hello_zlib: `0x${adler.toString(16).padStart(8, "0")}`,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
