#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const codecDir = path.join(root, "standards/build/wasm/codec-primitives");

function watPath(name) {
  return path.join(codecDir, `${name}.wat`);
}

function compileWat(name) {
  const source = watPath(name);
  if (!fs.existsSync(source)) {
    throw new Error(`missing WAT module for compression composition: ${source}`);
  }
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-composition-`));
  const wasmPath = path.join(tmp, `${name}.wasm`);
  execFileSync("wat2wasm", [source, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.rmSync(tmp, { recursive: true, force: true });
  return bytes;
}

async function load(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return {
    exports: instance.exports,
    memory: new Uint8Array(instance.exports.memory.buffer),
    view: new DataView(instance.exports.memory.buffer),
  };
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    written: Number((packed >> 32n) & 0xffffffffn),
  };
}

function zlibRecord(view, ptr) {
  return {
    deflateOff: view.getUint32(ptr, true),
    deflateLen: view.getUint32(ptr + 4, true),
    cmf: view.getUint32(ptr + 8, true),
    flg: view.getUint32(ptr + 12, true),
    windowLog2: view.getUint32(ptr + 16, true),
    flevel: view.getUint32(ptr + 20, true),
    fdict: view.getUint32(ptr + 24, true),
    expectedAdler32: view.getUint32(ptr + 28, true),
  };
}

function storedRecord(view, ptr) {
  return {
    blockCount: view.getUint32(ptr, true),
    payloadTotal: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    finalSeen: view.getUint32(ptr + 12, true),
  };
}

function inflateRecord(view, ptr) {
  return {
    consumed: view.getUint32(ptr, true),
    written: view.getUint32(ptr + 4, true),
    blockCount: view.getUint32(ptr + 8, true),
    lastBlockType: view.getUint32(ptr + 12, true),
    crc32: view.getUint32(ptr + 16, true),
    adler32: view.getUint32(ptr + 20, true),
  };
}

function bytes(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

async function tryInflateRoundTrip(member, wrapperRecord, payload, adler32) {
  if (!fs.existsSync(watPath("deflate-inflate"))) {
    return {
      status: "blocked",
      reason: `missing inflate-dependent module: ${watPath("deflate-inflate")}`,
    };
  }

  const inflate = await load("deflate-inflate");
  const fn =
    inflate.exports.deflate_inflate_raw ||
    inflate.exports.deflate_raw_inflate ||
    inflate.exports.deflate_inflate;
  if (typeof fn !== "function") {
    throw new Error("deflate-inflate.wat is present but exports no raw inflate function");
  }

  const raw = member.slice(wrapperRecord.deflateOff, wrapperRecord.deflateOff + wrapperRecord.deflateLen);
  const inPtr = 1024;
  const outPtr = 65536;
  const recPtr = 131072;
  inflate.memory.set(raw, inPtr);
  assert.equal(fn(inPtr, raw.length, outPtr, payload.length + 16, payload.length + 16, recPtr), 0);
  const result = inflateRecord(inflate.view, recPtr);
  assert.equal(result.consumed, raw.length);
  assert.equal(result.written, payload.length);
  assert.deepEqual(bytes(inflate.memory, outPtr, result.written), payload);

  const core = await load("encoding-core");
  core.memory.set(bytes(inflate.memory, outPtr, result.written), 1024);
  assert.equal(core.exports.adler32(1024, result.written) >>> 0, adler32);
  assert.equal(result.adler32, adler32);
  return {
    status: "ok",
    standard_id: inflate.exports.proto_standard_id(),
    written: result.written,
    adler32: `0x${result.adler32.toString(16).padStart(8, "0")}`,
  };
}

(async () => {
  const core = await load("encoding-core");
  const zlib = await load("zlib-wrapper");
  const stored = await load("deflate-stored");

  assert.equal(core.exports.proto_abi_version(), 2);
  assert.equal(zlib.exports.proto_abi_version(), 2);
  assert.equal(stored.exports.proto_abi_version(), 2);
  assert.equal(zlib.exports.proto_standard_id(), 300065);
  assert.equal(stored.exports.proto_standard_id(), 300066);

  const payload = Buffer.from("EdgeRun zlib stored composition proof.\n", "utf8");

  core.memory.set(payload, 1024);
  const adler32 = core.exports.adler32(1024, payload.length) >>> 0;

  stored.memory.set(payload, 1024);
  const encoded = unpack(stored.exports.deflate_stored_encode(1024, payload.length, 4096, payload.length + 16));
  assert.equal(encoded.status, 0);
  assert.equal(stored.exports.deflate_stored_scan(4096, encoded.written, 8192), 0);
  assert.deepEqual(storedRecord(stored.view, 8192), {
    blockCount: 1,
    payloadTotal: payload.length,
    consumed: encoded.written,
    finalSeen: 1,
  });

  let header = unpack(zlib.exports.zlib_write_header(6, 1024, 2));
  assert.deepEqual(header, { status: 0, written: 2 });
  let trailer = unpack(zlib.exports.zlib_write_trailer(adler32, 2048, 4));
  assert.deepEqual(trailer, { status: 0, written: 4 });

  const member = Buffer.concat([
    bytes(zlib.memory, 1024, header.written),
    bytes(stored.memory, 4096, encoded.written),
    bytes(zlib.memory, 2048, trailer.written),
  ]);

  zlib.memory.set(member, 4096);
  assert.equal(zlib.exports.zlib_member_scan(4096, member.length, 8192), 0);
  const wrapper = zlibRecord(zlib.view, 8192);
  assert.deepEqual(wrapper, {
    deflateOff: 2,
    deflateLen: encoded.written,
    cmf: 0x78,
    flg: 0x9c,
    windowLog2: 15,
    flevel: 2,
    fdict: 0,
    expectedAdler32: adler32,
  });
  assert.deepEqual(member.slice(member.length - 4), bytes(zlib.memory, 2048, 4));

  const inflate = await tryInflateRoundTrip(member, wrapper, payload, adler32);

  const dynamicRaw = Buffer.from(
    "cdcb470100410803404b942d202790c5bf84b371f31fbb05768673f486d491b6f241d2a10991b70ed037636d1d99576c6c4b9301c8b19ffc0f",
    "hex",
  );
  const dynamicPayload = Buffer.from(
    "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc983df1780b60c2b3fa9d3a19a00e46aac798451f0febdca52920faaddf" +
      "27badc98",
    "ascii",
  );
  core.memory.set(dynamicPayload, 1024);
  const dynamicAdler32 = core.exports.adler32(1024, dynamicPayload.length) >>> 0;
  trailer = unpack(zlib.exports.zlib_write_trailer(dynamicAdler32, 2048, 4));
  assert.deepEqual(trailer, { status: 0, written: 4 });
  const dynamicMember = Buffer.concat([
    bytes(zlib.memory, 1024, header.written),
    dynamicRaw,
    bytes(zlib.memory, 2048, trailer.written),
  ]);
  zlib.memory.set(dynamicMember, 4096);
  assert.equal(zlib.exports.zlib_member_scan(4096, dynamicMember.length, 8192), 0);
  const dynamicWrapper = zlibRecord(zlib.view, 8192);
  assert.equal(dynamicWrapper.deflateOff, 2);
  assert.equal(dynamicWrapper.deflateLen, dynamicRaw.length);
  assert.equal(dynamicWrapper.expectedAdler32, dynamicAdler32);
  const dynamicInflate = await tryInflateRoundTrip(
    dynamicMember,
    dynamicWrapper,
    dynamicPayload,
    dynamicAdler32,
  );

  console.log(
    JSON.stringify(
      {
        unit: "compression-zlib-composition",
        ok: true,
        wrapper_standard_id: zlib.exports.proto_standard_id(),
        deflate_stored_standard_id: stored.exports.proto_standard_id(),
        payload_len: payload.length,
        member_len: member.length,
        deflate_len: encoded.written,
        adler32: `0x${adler32.toString(16).padStart(8, "0")}`,
        inflate_round_trip: inflate,
        dynamic_inflate_round_trip: dynamicInflate,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
