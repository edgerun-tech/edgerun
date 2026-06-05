#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath = process.argv[2] || "standards/build/wasm/codec-primitives/tag-list.wat";

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

function record(view, out) {
  return {
    tagOff: view.getUint32(out, true),
    tagLen: view.getUint32(out + 4, true),
    valueOff: view.getUint32(out + 8, true),
    valueLen: view.getUint32(out + 12, true),
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
  assert.equal(exports.proto_standard_id(), 300051);

  let text = "v=1; a=rsa-sha256; bh=abc=; b=";
  let len = write(memory, ptr, text);
  let step = unpack(exports.tag_list_next(ptr, len, 0, out));
  assert.deepEqual(step, { status: 0, next: 4 });
  let rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "v");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "1");

  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  assert.equal(step.status, 0);
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "a");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "rsa-sha256");

  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  assert.equal(step.status, 0);
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "bh");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "abc=");

  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  assert.deepEqual(step, { status: 0, next: len });
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "b");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "");
  assert.deepEqual(unpack(exports.tag_list_next(ptr, len, step.next, out)), {
    status: 5,
    next: len,
  });

  text = "v=DMARC1; p=reject; pct=50";
  len = write(memory, ptr, text);
  step = unpack(exports.tag_list_next(ptr, len, 0, out));
  assert.equal(step.status, 0);
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "v");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "DMARC1");
  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "p");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "reject");
  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "pct");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "50");

  text = " \t ; ; foo_bar-1 \t = \t value with spaces \t ; empty= \t ; z=last ";
  len = write(memory, ptr, text);
  step = unpack(exports.tag_list_next(ptr, len, 0, out));
  assert.equal(step.status, 0);
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "foo_bar-1");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "value with spaces");
  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "empty");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "");
  step = unpack(exports.tag_list_next(ptr, len, step.next, out));
  rec = record(view, out);
  assert.equal(slice(memory, ptr, rec.tagOff, rec.tagLen), "z");
  assert.equal(slice(memory, ptr, rec.valueOff, rec.valueLen), "last");
  assert.deepEqual(unpack(exports.tag_list_next(ptr, len, step.next, out)), {
    status: 5,
    next: len,
  });

  for (const malformed of ["=value", "bad tag=value", "bad.tag=value", "tag", "tag ; x=1"]) {
    len = write(memory, ptr, malformed);
    assert.equal(unpack(exports.tag_list_next(ptr, len, 0, out)).status, 3, malformed);
  }

  len = write(memory, ptr, " ; ; \t ; ");
  assert.deepEqual(unpack(exports.tag_list_next(ptr, len, 0, out)), {
    status: 5,
    next: len,
  });

  console.log(
    JSON.stringify({
      unit: "tag-list",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
