#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/pem-rfc7468.wat";

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
    value: Number((packed >> 32n) & 0xffffffffn),
  };
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

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300015);

  const pemPtr = 1024;
  const outPtr = 4096;
  const bodyOutPtr = 8192;

  const certPem = [
    "-----BEGIN CERTIFICATE-----",
    "SGVsbG8sIFdvcmxkIQ==",
    "-----END CERTIFICATE-----",
    "",
  ].join("\n");
  let len = write(memory, pemPtr, certPem);
  assert.strictEqual(exports.pem_find_boundaries(pemPtr, len, outPtr), 0);
  const labelOff = view.getUint32(outPtr + 8, true);
  const labelLen = view.getUint32(outPtr + 12, true);
  const bodyOff = view.getUint32(outPtr + 16, true);
  const bodyLen = view.getUint32(outPtr + 20, true);
  assert.strictEqual(read(memory, pemPtr + labelOff, labelLen), "CERTIFICATE");

  let compact = unpack(exports.pem_compact_base64(pemPtr + bodyOff, bodyLen, bodyOutPtr, 64));
  assert.deepStrictEqual(compact, { status: 0, value: 20 });
  assert.strictEqual(read(memory, bodyOutPtr, compact.value), "SGVsbG8sIFdvcmxkIQ==");

  const crlfPem = certPem.replace(/\n/g, "\r\n");
  len = write(memory, pemPtr, crlfPem);
  assert.strictEqual(exports.pem_find_boundaries(pemPtr, len, outPtr), 0);
  compact = unpack(
    exports.pem_compact_base64(
      pemPtr + view.getUint32(outPtr + 16, true),
      view.getUint32(outPtr + 20, true),
      bodyOutPtr,
      64,
    ),
  );
  assert.strictEqual(compact.status, 0);

  const wrapped64 = [
    "-----BEGIN CERTIFICATE-----",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "AAAA",
    "-----END CERTIFICATE-----",
    "",
  ].join("\n");
  len = write(memory, pemPtr, wrapped64);
  assert.strictEqual(exports.pem_find_boundaries(pemPtr, len, outPtr), 0);
  compact = unpack(
    exports.pem_compact_base64(
      pemPtr + view.getUint32(outPtr + 16, true),
      view.getUint32(outPtr + 20, true),
      bodyOutPtr,
      128,
    ),
  );
  assert.deepStrictEqual(compact, { status: 0, value: 68 });

  assert.strictEqual(
    exports.pem_validate_label(pemPtr + labelOff, labelLen),
    0,
    "CERTIFICATE label should validate",
  );
  len = write(memory, pemPtr, "PRIVATE KEY");
  assert.strictEqual(exports.pem_validate_label(pemPtr, len), 0);
  len = write(memory, pemPtr, "certificate");
  assert.strictEqual(exports.pem_validate_label(pemPtr, len), 3);
  len = write(memory, pemPtr, "CERTIFICATE!");
  assert.strictEqual(exports.pem_validate_label(pemPtr, len), 3);

  const mismatched = [
    "-----BEGIN CERTIFICATE-----",
    "SGVsbG8=",
    "-----END PRIVATE KEY-----",
    "",
  ].join("\n");
  len = write(memory, pemPtr, mismatched);
  assert.strictEqual(exports.pem_find_boundaries(pemPtr, len, outPtr), 5);

  const lowercase = [
    "-----BEGIN certificate-----",
    "SGVsbG8=",
    "-----END certificate-----",
    "",
  ].join("\n");
  len = write(memory, pemPtr, lowercase);
  assert.strictEqual(exports.pem_find_boundaries(pemPtr, len, outPtr), 3);

  const withHeader = "Proc-Type: 4,ENCRYPTED\nSGVsbG8=";
  len = write(memory, pemPtr, withHeader);
  compact = unpack(exports.pem_compact_base64(pemPtr, len, bodyOutPtr, 64));
  assert.strictEqual(compact.status, 3);

  len = write(memory, pemPtr, "SGVs*bG8=");
  compact = unpack(exports.pem_compact_base64(pemPtr, len, bodyOutPtr, 64));
  assert.strictEqual(compact.status, 3);

  len = write(memory, pemPtr, "SGVsbG8=");
  compact = unpack(exports.pem_compact_base64(pemPtr, len, bodyOutPtr, 4));
  assert.strictEqual(compact.status, 2);

  const encodedLen = unpack(exports.pem_encoded_len(13, "CERTIFICATE".length));
  assert.deepStrictEqual(encodedLen, { status: 0, value: certPem.length });

  console.log(
    JSON.stringify(
      {
        unit: "pem-rfc7468",
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
