#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/kv-scan.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 64));
  memory.set(bytes, ptr);
  return bytes.length;
}

function unpack(packed) {
  return {
    status: Number(packed & 0xffffffffn),
    next: Number(packed >> 32n),
  };
}

function span2(view, out) {
  return {
    off: view.getUint32(out, true),
    len: view.getUint32(out + 4, true),
  };
}

function span4(view, out) {
  return {
    aOff: view.getUint32(out, true),
    aLen: view.getUint32(out + 4, true),
    bOff: view.getUint32(out + 8, true),
    bLen: view.getUint32(out + 12, true),
  };
}

function slice(memory, base, off, len) {
  return Buffer.from(memory.slice(base + off, base + off + len)).toString("ascii");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const ptr = 1024;
  const out = 4096;

  assert.equal(exports.proto_abi_version(), 2);
  assert.equal(exports.proto_standard_id(), 300061);

  let text = "Key: value";
  let len = write(memory, ptr, text);
  assert.equal(exports.kv_colon_scan(ptr, len, out), 0);
  let rec = span4(view, out);
  assert.equal(slice(memory, ptr, rec.aOff, rec.aLen), "Key");
  assert.equal(slice(memory, ptr, rec.bOff, rec.bLen), "value");

  text = " \t stream_id \t : \t abc:123 \r\n";
  len = write(memory, ptr, text);
  assert.equal(exports.kv_colon_scan(ptr, len, out), 0);
  rec = span4(view, out);
  assert.equal(slice(memory, ptr, rec.aOff, rec.aLen), "stream_id");
  assert.equal(slice(memory, ptr, rec.bOff, rec.bLen), "abc:123");

  len = write(memory, ptr, "missing colon");
  assert.equal(exports.kv_colon_scan(ptr, len, out), 3);

  text = "[a, b, c]";
  len = write(memory, ptr, text);
  let step = unpack(exports.bracket_list_next(ptr, len, 0, out));
  assert.equal(step.status, 0);
  let item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "a");
  step = unpack(exports.bracket_list_next(ptr, len, step.next, out));
  assert.equal(step.status, 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "b");
  step = unpack(exports.bracket_list_next(ptr, len, step.next, out));
  assert.equal(step.status, 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "c");
  assert.equal(unpack(exports.bracket_list_next(ptr, len, step.next, out)).status, 5);

  for (const empty of ["[]", "", " [ \t ] "]) {
    len = write(memory, ptr, empty);
    assert.equal(unpack(exports.bracket_list_next(ptr, len, 0, out)).status, 5, empty);
  }

  text = "  \"hello world\"  ";
  len = write(memory, ptr, text);
  assert.equal(exports.unquote_span(ptr, len, out), 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "hello world");

  text = " 'single' ";
  len = write(memory, ptr, text);
  assert.equal(exports.unquote_span(ptr, len, out), 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "single");

  text = " plain ";
  len = write(memory, ptr, text);
  assert.equal(exports.unquote_span(ptr, len, out), 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "plain");

  text = "\"mismatch'";
  len = write(memory, ptr, text);
  assert.equal(exports.unquote_span(ptr, len, out), 0);
  item = span2(view, out);
  assert.equal(slice(memory, ptr, item.off, item.len), "\"mismatch'");

  console.log(
    JSON.stringify({
      unit: "kv-scan",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
