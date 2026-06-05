#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/ipv4-net.wat";

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

function u32(value) {
  return value >>> 0;
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300055);

  const inPtr = 1024;
  const outPtr = 2048;

  memory.set([192, 168, 1, 1], inPtr);
  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 4, 0)), {
    status: 0,
    value: 0xc0a80101,
  });

  memory.set([9, 9, 192, 168, 1, 1], inPtr);
  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 6, 2)), {
    status: 0,
    value: 0xc0a80101,
  });

  memory.set([0, 0, 0, 0], inPtr);
  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 4, 0)), {
    status: 0,
    value: 0,
  });

  memory.set([255, 255, 255, 255], inPtr);
  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 4, 0)), {
    status: 0,
    value: 0xffffffff,
  });

  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 3, 0)), {
    status: 4,
    value: 0,
  });

  assert.deepStrictEqual(unpack(exports.ipv4_to_u32(inPtr, 6, 3)), {
    status: 4,
    value: 0,
  });

  memory.fill(0, outPtr, outPtr + 8);
  assert.deepStrictEqual(unpack(exports.ipv4_from_u32(0xc0a80101, outPtr, 4)), {
    status: 0,
    value: 4,
  });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [192, 168, 1, 1]);

  memory.fill(0, outPtr, outPtr + 8);
  assert.deepStrictEqual(unpack(exports.ipv4_from_u32(0xffffffff, outPtr, 3)), {
    status: 2,
    value: 0,
  });
  assert.deepStrictEqual(Array.from(memory.slice(outPtr, outPtr + 4)), [0, 0, 0, 0]);

  const ip24 = 0xc0a8010a;
  const mask24 = 0xffffff00;
  assert.strictEqual(u32(exports.ipv4_network(ip24, mask24)), 0xc0a80100);
  assert.strictEqual(u32(exports.ipv4_broadcast(ip24, mask24)), 0xc0a801ff);

  const ip8 = 0x0a000001;
  const mask8 = 0xff000000;
  assert.strictEqual(u32(exports.ipv4_network(ip8, mask8)), 0x0a000000);
  assert.strictEqual(u32(exports.ipv4_broadcast(ip8, mask8)), 0x0affffff);

  assert.strictEqual(exports.ipv4_in_network(0xc0a80132, 0xc0a80100, mask24), 1);
  assert.strictEqual(exports.ipv4_in_network(0xc0a80232, 0xc0a80100, mask24), 0);

  console.log(
    JSON.stringify(
      {
        unit: "ipv4-net",
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
