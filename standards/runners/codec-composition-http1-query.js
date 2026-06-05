#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const codecDir = path.join(root, "standards/build/wasm/codec-primitives");

function compileWat(name) {
  const watPath = path.join(codecDir, `${name}.wat`);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), `${name}-composition-`));
  const wasmPath = path.join(tmp, `${name}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "pipe" });
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

function reqLine(view, ptr) {
  return {
    methodOffset: view.getUint32(ptr, true),
    methodLen: view.getUint32(ptr + 4, true),
    targetOffset: view.getUint32(ptr + 8, true),
    targetLen: view.getUint32(ptr + 12, true),
    major: view.getUint32(ptr + 16, true),
    minor: view.getUint32(ptr + 20, true),
    lineEnd: view.getUint32(ptr + 24, true),
  };
}

function span6(view, ptr) {
  return {
    a: view.getUint32(ptr, true),
    b: view.getUint32(ptr + 4, true),
    c: view.getUint32(ptr + 8, true),
    d: view.getUint32(ptr + 12, true),
    e: view.getUint32(ptr + 16, true),
    f: view.getUint32(ptr + 20, true),
  };
}

(async () => {
  const http1 = await load("http1-lines");
  const percent = await load("percent-url-form");

  const request = Buffer.from("GET /run/app?name=EdgeRun&cache=verify%20cache&cache=again&=emptykey&plus=two+words&keyonly&empty= HTTP/1.1\r\nHost: example.com\r\n\r\n", "ascii");
  http1.memory.set(request, 1024);
  assert.equal(http1.exports.http_parse_request_line(1024, request.length, 4096), 0);
  const line = reqLine(http1.view, 4096);
  assert.deepEqual({
    method: Buffer.from(http1.memory.slice(1024 + line.methodOffset, 1024 + line.methodOffset + line.methodLen)).toString("ascii"),
    target: Buffer.from(http1.memory.slice(1024 + line.targetOffset, 1024 + line.targetOffset + line.targetLen)).toString("ascii"),
    major: line.major,
    minor: line.minor,
  }, {
    method: "GET",
    target: "/run/app?name=EdgeRun&cache=verify%20cache&cache=again&=emptykey&plus=two+words&keyonly&empty=",
    major: 1,
    minor: 1,
  });

  const targetBytes = http1.memory.slice(1024 + line.targetOffset, 1024 + line.targetOffset + line.targetLen);
  percent.memory.set(targetBytes, 1024);
  assert.equal(percent.exports.uri_scan_path_query(1024, targetBytes.length, 4096), 0);
  const uri = span6(percent.view, 4096);
  assert.equal(Buffer.from(percent.memory.slice(1024 + uri.a, 1024 + uri.a + uri.b)).toString("ascii"), "/run/app");
  assert.equal(Buffer.from(percent.memory.slice(1024 + uri.c, 1024 + uri.c + uri.d)).toString("ascii"), "name=EdgeRun&cache=verify%20cache&cache=again&=emptykey&plus=two+words&keyonly&empty=");

  let offset = uri.c;
  const pairs = [];
  while (offset < uri.c + uri.d) {
    assert.equal(percent.exports.form_urlencoded_next_pair(1024, targetBytes.length, offset, 8192), 0);
    const pair = span6(percent.view, 8192);
    pairs.push({
      key: Buffer.from(percent.memory.slice(1024 + pair.a, 1024 + pair.a + pair.b)).toString("ascii"),
      value: Buffer.from(percent.memory.slice(1024 + pair.c, 1024 + pair.c + pair.d)).toString("ascii"),
    });
    offset = pair.e;
  }
  assert.deepEqual(pairs, [
    { key: "name", value: "EdgeRun" },
    { key: "cache", value: "verify%20cache" },
    { key: "cache", value: "again" },
    { key: "", value: "emptykey" },
    { key: "plus", value: "two+words" },
    { key: "keyonly", value: "" },
    { key: "empty", value: "" },
  ]);

  const encodedValue = Buffer.from(pairs[1].value, "ascii");
  percent.memory.set(encodedValue, 12288);
  const packed = percent.exports.percent_decode_strict(12288, encodedValue.length, 14336, 128);
  assert.equal(Number(packed & 0xffffffffn), 0);
  const written = Number((packed >> 32n) & 0xffffffffn);
  assert.equal(Buffer.from(percent.memory.slice(14336, 14336 + written)).toString("ascii"), "verify cache");

  const plusValue = Buffer.from(pairs[4].value, "ascii");
  percent.memory.set(plusValue, 12288);
  const plusPacked = percent.exports.percent_decode_strict(12288, plusValue.length, 14336, 128);
  assert.equal(Number(plusPacked & 0xffffffffn), 0);
  const plusWritten = Number((plusPacked >> 32n) & 0xffffffffn);
  assert.equal(Buffer.from(percent.memory.slice(14336, 14336 + plusWritten)).toString("ascii"), "two+words");

  const malformed = Buffer.from("GET /bad?x=%GG HTTP/1.1\r\n\r\n", "ascii");
  http1.memory.fill(0, 2048, 2048 + malformed.length + 64);
  http1.memory.set(malformed, 2048);
  assert.equal(http1.exports.http_parse_request_line(2048, malformed.length, 4096), 0);
  const badLine = reqLine(http1.view, 4096);
  const badTarget = http1.memory.slice(2048 + badLine.targetOffset, 2048 + badLine.targetOffset + badLine.targetLen);
  percent.memory.fill(0, 16384, 16384 + badTarget.length + 64);
  percent.memory.set(badTarget, 16384);
  assert.equal(percent.exports.uri_scan_path_query(16384, badTarget.length, 4096), 0);
  const badUri = span6(percent.view, 4096);
  assert.equal(percent.exports.form_urlencoded_next_pair(16384, badTarget.length, badUri.c, 8192), 3);

  console.log(JSON.stringify({
    unit: "codec-composition-http1-query",
    ok: true,
    pipeline: "http1-lines -> percent-url-form",
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
