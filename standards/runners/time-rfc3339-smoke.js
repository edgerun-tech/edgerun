#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/time-rfc3339.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "time-rfc3339-"));
const wasmPath = path.join(tmpDir, "time-rfc3339.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(result) {
  const value = BigInt.asUintN(64, result);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function putAscii(mem, ptr, text) {
  mem.fill(0, ptr, ptr + text.length + 64);
  mem.set(Buffer.from(text, "ascii"), ptr);
}

function u32(mem, ptr) {
  return (mem[ptr] | (mem[ptr + 1] << 8) | (mem[ptr + 2] << 16) | (mem[ptr + 3] << 24)) >>> 0;
}

function i32(mem, ptr) {
  return u32(mem, ptr) | 0;
}

function i64(mem, ptr) {
  const lo = BigInt(u32(mem, ptr));
  const hi = BigInt(u32(mem, ptr + 4));
  const value = (hi << 32n) | lo;
  return value & (1n << 63n) ? value - (1n << 64n) : value;
}

function record(mem, out) {
  return {
    year: u32(mem, out),
    month: u32(mem, out + 4),
    day: u32(mem, out + 8),
    hour: u32(mem, out + 12),
    minute: u32(mem, out + 16),
    second: u32(mem, out + 20),
    nanos: u32(mem, out + 24),
    offsetMinutes: i32(mem, out + 28),
    unix: i64(mem, out + 32),
  };
}

function parse(e, mem, text, ptr, out) {
  putAscii(mem, ptr, text);
  const status = e.rfc3339_parse(ptr, text.length, out);
  return { status, rec: record(mem, out) };
}

function unixSeconds(year, month, day, hour, minute, second) {
  return BigInt(Date.UTC(year, month - 1, day, hour, minute, second) / 1000);
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const inPtr = 1024;
  const out = 4096;
  const emitOut = 8192;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300053);

  let got = parse(e, mem, "2030-01-01T00:00:00Z", inPtr, out);
  assert.equal(got.status, 0);
  assert.deepEqual(got.rec, {
    year: 2030,
    month: 1,
    day: 1,
    hour: 0,
    minute: 0,
    second: 0,
    nanos: 0,
    offsetMinutes: 0,
    unix: 1893456000n,
  });

  got = parse(e, mem, "2030-01-01T07:00:00+07:00", inPtr, out);
  assert.equal(got.status, 0);
  assert.equal(got.rec.offsetMinutes, 420);
  assert.equal(got.rec.unix, 1893456000n);

  got = parse(e, mem, "2029-12-31T19:00:00-05:00", inPtr, out);
  assert.equal(got.status, 0);
  assert.equal(got.rec.offsetMinutes, -300);
  assert.equal(got.rec.unix, 1893456000n);

  got = parse(e, mem, "2030-01-01T00:00:00.123456789Z", inPtr, out);
  assert.equal(got.status, 0);
  assert.equal(got.rec.nanos, 123456789);
  assert.equal(got.rec.unix, unixSeconds(2030, 1, 1, 0, 0, 0));

  let packed = unpack(e.rfc3339_emit_utc(2030, 1, 1, 0, 0, 0, 0, emitOut, 20));
  assert.deepEqual(packed, { status: 0, written: 20 });
  assert.equal(Buffer.from(mem.slice(emitOut, emitOut + 20)).toString("ascii"), "2030-01-01T00:00:00Z");

  packed = unpack(e.rfc3339_emit_utc(2030, 1, 1, 0, 0, 0, 123456789, emitOut, 30));
  assert.deepEqual(packed, { status: 0, written: 30 });
  assert.equal(Buffer.from(mem.slice(emitOut, emitOut + 30)).toString("ascii"), "2030-01-01T00:00:00.123456789Z");

  assert.equal(parse(e, mem, "2030-01-01", inPtr, out).status, 1, "date-only is short");
  assert.equal(parse(e, mem, "2030-01-01 00:00:00Z", inPtr, out).status, 3, "space separator");
  assert.equal(parse(e, mem, "2030-02-29T00:00:00Z", inPtr, out).status, 3, "bad date");
  assert.equal(parse(e, mem, "2030-01-01T00:00:00.1234567890Z", inPtr, out).status, 3, "too many fractional digits");

  packed = unpack(e.rfc3339_emit_utc(2030, 1, 1, 0, 0, 0, 0, emitOut, 19));
  assert.deepEqual(packed, { status: 2, written: 0 });

  console.log(JSON.stringify({
    unit: "time-rfc3339",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
