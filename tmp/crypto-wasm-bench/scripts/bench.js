#!/usr/bin/env node

const fs = require("fs");
const crypto = require("crypto");
const { execFileSync } = require("child_process");

const root = process.argv[2] || "tmp/crypto-wasm-bench";
const bytes = Number(process.argv[3] || 4096);
const iterations = Number(process.argv[4] || 20000);
const rustWasmPath = `${root}/dist/edgerun_crypto_wasm_bench.wasm`;
const asWasmPath = `${root}/dist/as-crypto.wasm`;
const nativePath = process.env.NATIVE_BENCH || "/home/ken/.cache/cargo-target/x86_64-unknown-linux-gnu/release/native-bench";

function fillInput(len) {
  const out = Buffer.alloc(len);
  let x = 0x12345678 >>> 0;
  for (let i = 0; i < len; i++) {
    x ^= (x << 13) >>> 0;
    x ^= x >>> 17;
    x ^= (x << 5) >>> 0;
    out[i] = x & 0xff;
  }
  return out;
}

function hex(buf) {
  return Buffer.from(buf).toString("hex");
}

async function loadWasm(path) {
  const imports = {
    env: {
      abort(_message, _file, line, column) {
        throw new Error(`wasm abort at ${line}:${column}`);
      },
    },
  };
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(path), imports);
  return instance.exports;
}

function memoryView(exports) {
  return new Uint8Array(exports.memory.buffer);
}

function ensureMemory(exports, requiredBytes) {
  const pageSize = 65536;
  const current = exports.memory.buffer.byteLength;
  if (current >= requiredBytes) return;
  exports.memory.grow(Math.ceil((requiredBytes - current) / pageSize));
}

function benchWasm(exports, names, input, key) {
  const inputPtr = 16384;
  const keyPtr = inputPtr + input.length + 64;
  const p256KeyPtr = keyPtr + key.length + 64;
  const ed25519KeyPtr = p256KeyPtr + 32 + 64;
  const outPtr = ed25519KeyPtr + 32 + 64;
  ensureMemory(exports, outPtr + 64 + input.length * 4 + 1024 * 1024);
  let mem = memoryView(exports);
  mem.set(input, inputPtr);
  mem.set(key, keyPtr);
  mem.set(Buffer.alloc(32, 7), p256KeyPtr);
  mem.set(Buffer.alloc(32, 9), ed25519KeyPtr);

  const shaStart = process.hrtime.bigint();
  for (let i = 0; i < iterations; i++) {
    const rc = exports[names.sha](inputPtr, input.length, outPtr);
    if (rc !== 0) throw new Error(`${names.engine} sha256 failed: ${rc}`);
  }
  const shaElapsed = Number(process.hrtime.bigint() - shaStart);
  mem = memoryView(exports);
  const sha = hex(mem.slice(outPtr, outPtr + 32));
  mem.set(input, inputPtr);
  mem.set(key, keyPtr);

  const hmacStart = process.hrtime.bigint();
  for (let i = 0; i < iterations; i++) {
    const rc = exports[names.hmac](keyPtr, key.length, inputPtr, input.length, outPtr);
    if (rc !== 0) throw new Error(`${names.engine} hmac failed: ${rc}`);
  }
  const hmacElapsed = Number(process.hrtime.bigint() - hmacStart);
  mem = memoryView(exports);
  const hmac = hex(mem.slice(outPtr, outPtr + 32));

  let p256Signature = null;
  let p256Elapsed = null;
  if (names.p256) {
    mem.set(Buffer.from(sha, "hex"), inputPtr);
    mem.set(Buffer.alloc(32, 7), p256KeyPtr);
    let sigLen = 0;
    const p256Start = process.hrtime.bigint();
    for (let i = 0; i < iterations; i++) {
      sigLen = exports[names.p256](p256KeyPtr, inputPtr, outPtr);
      if (sigLen <= 0) throw new Error(`${names.engine} p256 sign failed: ${sigLen}`);
    }
    p256Elapsed = Number(process.hrtime.bigint() - p256Start);
    mem = memoryView(exports);
    p256Signature = hex(mem.slice(outPtr, outPtr + sigLen));
  }

  let ed25519Signature = null;
  let ed25519Elapsed = null;
  if (names.ed25519) {
    mem = memoryView(exports);
    mem.set(input, inputPtr);
    mem.set(Buffer.alloc(32, 9), ed25519KeyPtr);
    let sigLen = 0;
    const ed25519Start = process.hrtime.bigint();
    for (let i = 0; i < iterations; i++) {
      sigLen = exports[names.ed25519](ed25519KeyPtr, inputPtr, input.length, outPtr);
      if (sigLen !== 64) throw new Error(`${names.engine} ed25519 sign failed: ${sigLen}`);
    }
    ed25519Elapsed = Number(process.hrtime.bigint() - ed25519Start);
    mem = memoryView(exports);
    ed25519Signature = hex(mem.slice(outPtr, outPtr + sigLen));
  }

  return {
    engine: names.engine,
    bytes,
    iterations,
    sha256_ns_per_iter: shaElapsed / iterations,
    hmac_sha256_ns_per_iter: hmacElapsed / iterations,
    p256_sign_ns_per_iter: p256Elapsed == null ? null : p256Elapsed / iterations,
    ed25519_sign_ns_per_iter: ed25519Elapsed == null ? null : ed25519Elapsed / iterations,
    sha256: sha,
    hmac_sha256: hmac,
    p256_signature: p256Signature,
    ed25519_signature: ed25519Signature,
  };
}

(async () => {
  const input = fillInput(bytes);
  const key = fillInput(32);
  const expectedSha = crypto.createHash("sha256").update(input).digest("hex");
  const expectedHmac = crypto.createHmac("sha256", key).update(input).digest("hex");

  const native = JSON.parse(execFileSync(nativePath, [String(bytes), String(iterations)], { encoding: "utf8" }));
  const rustWasm = benchWasm(await loadWasm(rustWasmPath), {
    engine: "rust-wasm-edgerun-crypto",
    sha: "edgerun_sha256",
    hmac: "edgerun_hmac_sha256",
    p256: "edgerun_p256_sign_prehash",
    ed25519: "edgerun_ed25519_sign",
  }, input, key);
  const asWasm = benchWasm(await loadWasm(asWasmPath), {
    engine: "assemblyscript-wasm",
    sha: "sha256",
    hmac: "hmac_sha256",
  }, input, key);

  const rows = [native, rustWasm, asWasm];
  for (const row of rows) {
    row.sha256_ok = row.sha256 === expectedSha;
    row.hmac_sha256_ok = row.hmac_sha256 === expectedHmac;
    row.p256_signature_ok = row.p256_signature == null || row.p256_signature === native.p256_signature;
    row.ed25519_signature_ok = row.ed25519_signature == null || row.ed25519_signature === native.ed25519_signature;
  }

  console.table(rows.map((row) => ({
    engine: row.engine,
    bytes: row.bytes,
    iterations: row.iterations,
    "sha256 ns/op": row.sha256_ns_per_iter.toFixed(2),
    "hmac ns/op": row.hmac_sha256_ns_per_iter.toFixed(2),
    "p256 sign ns/op": row.p256_sign_ns_per_iter == null ? "n/a" : row.p256_sign_ns_per_iter.toFixed(2),
    "ed25519 sign ns/op": row.ed25519_sign_ns_per_iter == null ? "n/a" : row.ed25519_sign_ns_per_iter.toFixed(2),
    sha_ok: row.sha256_ok,
    hmac_ok: row.hmac_sha256_ok,
    p256_ok: row.p256_signature_ok,
    ed25519_ok: row.ed25519_signature_ok,
  })));
  console.log(JSON.stringify(rows, null, 2));

  if (!rows.every((row) => row.sha256_ok && row.hmac_sha256_ok && row.p256_signature_ok && row.ed25519_signature_ok)) process.exit(1);
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
