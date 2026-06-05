#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath = process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/http-date.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "http-date-"));
const wasmPath = path.join(tmpDir, "http-date.wasm");

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

function u64(mem, ptr) {
  return (BigInt(u32(mem, ptr + 4)) << 32n) | BigInt(u32(mem, ptr));
}

function record(mem, out) {
  return {
    unix: u64(mem, out),
    year: u32(mem, out + 8),
    month: u32(mem, out + 12),
    day: u32(mem, out + 16),
    hour: u32(mem, out + 20),
    minute: u32(mem, out + 24),
    second: u32(mem, out + 28),
    weekday: u32(mem, out + 32),
    formatKind: u32(mem, out + 36),
  };
}

function parse(e, mem, text, ptr, out) {
  putAscii(mem, ptr, text);
  const status = e.http_date_parse(ptr, text.length, out);
  return { status, rec: record(mem, out) };
}

function format(e, mem, unix, out, cap) {
  mem.fill(0, out, out + Math.max(cap, 32));
  const packed = unpack(e.http_date_format(Number(unix & 0xffff_ffffn), Number(unix >> 32n), out, cap));
  return {
    ...packed,
    text: Buffer.from(mem.slice(out, out + packed.written)).toString("ascii"),
  };
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const inPtr = 1024;
  const out = 4096;
  const fmt = 8192;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300103);

  for (const [text, formatKind] of [
    ["Sun, 06 Nov 1994 08:49:37 GMT", 1],
    ["Sunday, 06-Nov-94 08:49:37 GMT", 2],
    ["Sun Nov  6 08:49:37 1994", 3],
  ]) {
    const got = parse(e, mem, text, inPtr, out);
    assert.equal(got.status, 0, text);
    assert.deepEqual(got.rec, {
      unix: 784111777n,
      year: 1994,
      month: 11,
      day: 6,
      hour: 8,
      minute: 49,
      second: 37,
      weekday: 7,
      formatKind,
    });
  }

  assert.equal(parse(e, mem, "Thu, 01 Jan 1970 00:00:00 GMT", inPtr, out).rec.unix, 0n);
  assert.equal(parse(e, mem, "Sun, 02 Oct 2016 14:44:11 GMT", inPtr, out).rec.unix, 1475419451n);

  assert.equal(parse(e, mem, "Sun Nov 10 08:00:00 1000", inPtr, out).status, 3, "invalid year 1000");
  assert.equal(parse(e, mem, "Sun Nov 10 08*00:00 2000", inPtr, out).status, 3, "bad asctime separator");
  assert.equal(parse(e, mem, "Sunday, 06-Nov-94 08+49:37 GMT", inPtr, out).status, 3, "bad RFC850 separator");
  assert.equal(parse(e, mem, "Sun, 07 Nov 1994 08:48:37 GMT", inPtr, out).status, 3, "wrong weekday/date");

  assert.equal(e.http_date_validate_parts(1970, 1, 1, 0, 0, 0, 4), 0);
  assert.equal(e.http_date_validate_parts(1970, 1, 1, 0, 0, 0, 5), 3);
  assert.equal(e.http_date_validate_parts(1969, 12, 31, 23, 59, 59, 3), 3);
  assert.equal(e.http_date_validate_parts(2030, 2, 29, 0, 0, 0, 5), 3);

  assert.deepEqual(format(e, mem, 0n, fmt, 29), {
    status: 0,
    written: 29,
    text: "Thu, 01 Jan 1970 00:00:00 GMT",
  });
  assert.deepEqual(format(e, mem, 1475419451n, fmt, 29), {
    status: 0,
    written: 29,
    text: "Sun, 02 Oct 2016 14:44:11 GMT",
  });
  assert.deepEqual(format(e, mem, 0n, fmt, 28), {
    status: 2,
    written: 0,
    text: "",
  });

  console.log(JSON.stringify({
    unit: "http-date",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
