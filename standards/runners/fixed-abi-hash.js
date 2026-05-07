#!/usr/bin/env node

const crypto = require("crypto");
const fs = require("fs");
const { hashCost, sha256Hex } = require("./cost-models");

const wasmPath = process.argv[2] || "standards/build/wasm/sha256-fips180/sha256-fips180.wasm";
const algorithm =
  process.argv[3] ||
  (wasmPath.includes("sha1")
    ? "sha1"
    : wasmPath.includes("sha384")
      ? "sha384"
      : wasmPath.includes("sha512")
        ? "sha512"
        : "sha256");

const cases = [
  Buffer.from(""),
  Buffer.from("abc"),
  Buffer.from("hello world"),
  Buffer.from("a".repeat(1000)),
];

function writeU16LE(buf, offset, value) {
  buf[offset] = value & 0xff;
  buf[offset + 1] = (value >>> 8) & 0xff;
}

function writeU32LE(buf, offset, value) {
  buf[offset] = value & 0xff;
  buf[offset + 1] = (value >>> 8) & 0xff;
  buf[offset + 2] = (value >>> 16) & 0xff;
  buf[offset + 3] = (value >>> 24) & 0xff;
}

function inputBytesFrame(payload) {
  const frame = Buffer.alloc(8 + payload.length);
  writeU16LE(frame, 0, 1);
  writeU16LE(frame, 2, 0);
  writeU32LE(frame, 4, payload.length);
  payload.copy(frame, 8);
  return frame;
}

(async () => {
  const module = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const handle = exports.proto_open(0, 0);

  const result = {
    unit: `${algorithm}-fips180`,
    abi_version: exports.proto_abi_version(),
    standard_id: exports.proto_standard_id(),
    ok: true,
    cases: [],
  };

  for (const payload of cases) {
    const frame = inputBytesFrame(payload);
    memory.set(frame, 1024);
    const packed = exports.proto_push(handle, 1024, frame.length);
    const ptr = Number(packed >> 32n);
    const len = Number(packed & 0xffffffffn);
    const out = Buffer.from(memory.slice(ptr, ptr + len));

    const kind = out.readUInt16LE(0);
    const status = out.readUInt16LE(2);
    const payloadLen = out.readUInt32LE(4);
    const digestLen =
      algorithm === "sha1" ? 20 : algorithm === "sha384" ? 48 : algorithm === "sha512" ? 64 : 32;
    const digest = out.slice(8, 8 + digestLen).toString("hex");
    const expected = crypto.createHash(algorithm).update(payload).digest("hex");
    const ok = kind === 11 && status === 0 && payloadLen === digestLen && digest === expected;
    const inputFrameHash = sha256Hex(frame);
    const outputFrameHash = sha256Hex(out);

    result.ok &&= ok;
    result.cases.push({
      input_len: payload.length,
      ok,
      digest,
      cost_quote: hashCost(`${algorithm}-fips180`, payload.length),
      transcript: {
        input_frame_sha256: inputFrameHash,
        output_frame_sha256: outputFrameHash,
      },
    });
  }

  console.log(JSON.stringify(result, null, 2));
  process.exit(result.ok ? 0 : 1);
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
