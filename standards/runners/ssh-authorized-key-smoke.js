#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/ssh-authorized-key.wat";

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
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function read(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len)).toString("ascii");
}

function record(view, outPtr) {
  return {
    typeOff: view.getUint32(outPtr, true),
    typeLen: view.getUint32(outPtr + 4, true),
    b64Off: view.getUint32(outPtr + 8, true),
    b64Len: view.getUint32(outPtr + 12, true),
    commentOff: view.getUint32(outPtr + 16, true),
    commentLen: view.getUint32(outPtr + 20, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300045);

  const inPtr = 1024;
  const outPtr = 4096;

  let line = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEdH";
  let len = write(memory, inPtr, line);
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 0);
  let rec = record(view, outPtr);
  assert.deepStrictEqual(rec, {
    typeOff: 0,
    typeLen: 11,
    b64Off: 12,
    b64Len: 28,
    commentOff: 40,
    commentLen: 0,
  });
  assert.strictEqual(read(memory, inPtr + rec.typeOff, rec.typeLen), "ssh-ed25519");
  assert.strictEqual(read(memory, inPtr + rec.b64Off, rec.b64Len), "AAAAC3NzaC1lZDI1NTE5AAAAIEdH");

  line = "\t  sk-ssh-ed25519@openssh.com\tQUJDRA==";
  len = write(memory, inPtr, line);
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 0);
  rec = record(view, outPtr);
  assert.strictEqual(read(memory, inPtr + rec.typeOff, rec.typeLen), "sk-ssh-ed25519@openssh.com");
  assert.strictEqual(read(memory, inPtr + rec.b64Off, rec.b64Len), "QUJDRA==");
  assert.strictEqual(rec.commentLen, 0);

  line = "ecdsa-sha2-nistp256 QUJDRA== alice@example";
  len = write(memory, inPtr, line);
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 0);
  rec = record(view, outPtr);
  assert.strictEqual(read(memory, inPtr + rec.typeOff, rec.typeLen), "ecdsa-sha2-nistp256");
  assert.strictEqual(read(memory, inPtr + rec.b64Off, rec.b64Len), "QUJDRA==");
  assert.strictEqual(read(memory, inPtr + rec.commentOff, rec.commentLen), "alice@example");

  line = "ssh-rsa QUJDRA== comment tail\nignored";
  len = write(memory, inPtr, line);
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 0);
  rec = record(view, outPtr);
  assert.strictEqual(read(memory, inPtr + rec.commentOff, rec.commentLen), "comment tail");

  for (const invalid of ["", "   \t", "# disabled", "   # disabled"]) {
    len = write(memory, inPtr, invalid);
    assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 3);
  }

  len = write(memory, inPtr, "ssh-ed25519");
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 3);

  len = write(memory, inPtr, "ssh-ed25519 AAAA*BAD");
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 3);

  len = write(memory, inPtr, "ssh-ed25519 AAA=BAD");
  assert.strictEqual(exports.ssh_authorized_key_scan(inPtr, len, outPtr), 3);

  console.log(
    JSON.stringify(
      {
        unit: "ssh-authorized-key",
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
