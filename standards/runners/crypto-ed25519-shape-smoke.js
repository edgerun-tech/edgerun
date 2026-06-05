const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/crypto-ed25519-shape.wat");
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "crypto-ed25519-shape-"));
const wasm = path.join(tmp, "crypto-ed25519-shape.wasm");

function packStatus(value) {
  return Number(BigInt.asUintN(32, BigInt(value)));
}

function packWritten(value) {
  return Number(BigInt(value) >> 32n);
}

function bytes(mem, ptr, len) {
  return Array.from(new Uint8Array(mem.buffer, ptr, len));
}

try {
  execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasm], { stdio: "pipe" });

  const module = new WebAssembly.Module(fs.readFileSync(wasm));
  const instance = new WebAssembly.Instance(module, {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300081);

  assert.equal(e.ed25519_seed_status(32, 32), 0);
  assert.equal(e.ed25519_seed_status(32, 31), 2);
  assert.equal(e.ed25519_seed_status(32, 33), 2);

  mem.fill(0, 128, 160);
  assert.equal(e.ed25519_public_key_status(128, 32), 3);
  mem[128] = 1;
  assert.equal(e.ed25519_public_key_status(128, 32), 0);
  assert.equal(e.ed25519_public_key_status(128, 31), 2);
  assert.equal(e.ed25519_public_key_status(128, 33), 2);

  for (let i = 0; i < 64; i++) mem[256 + i] = i;
  const split = e.ed25519_signature_split(256, 64, 384, 448);
  assert.equal(packStatus(split), 0);
  assert.equal(packWritten(split), 64);
  assert.deepEqual(bytes(e.memory, 384, 32), Array.from({ length: 32 }, (_, i) => i));
  assert.deepEqual(bytes(e.memory, 448, 32), Array.from({ length: 32 }, (_, i) => i + 32));

  const splitBad = e.ed25519_signature_split(256, 63, 384, 448);
  assert.equal(packStatus(splitBad), 2);
  assert.equal(packWritten(splitBad), 0);

  for (let i = 0; i < 32; i++) mem[512 + i] = 0xff;
  mem[512] = 0x07;
  mem[543] = 0xff;
  const prune = e.ed25519_expanded_secret_prune(512, 32, 576);
  assert.equal(packStatus(prune), 0);
  assert.equal(packWritten(prune), 32);
  const pruned = bytes(e.memory, 576, 32);
  assert.equal(pruned[0], 0x00);
  assert.equal(pruned[31], 0x7f);
  for (let i = 1; i < 31; i++) assert.equal(pruned[i], 0xff);

  mem.fill(0, 640, 672);
  mem[640] = 0xff;
  mem[671] = 0x00;
  const prune2 = e.ed25519_expanded_secret_prune(640, 32, 704);
  assert.equal(packStatus(prune2), 0);
  assert.equal(packWritten(prune2), 32);
  assert.equal(mem[704], 0xf8);
  assert.equal(mem[735], 0x40);

  const pruneBad = e.ed25519_expanded_secret_prune(512, 31, 576);
  assert.equal(packStatus(pruneBad), 2);
  assert.equal(packWritten(pruneBad), 0);

  console.log(JSON.stringify({
    unit: "crypto-ed25519-shape",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
    cases: [
      "seed_len",
      "public_key_len",
      "public_key_zero_reject",
      "signature_split",
      "signature_len_reject",
      "expanded_secret_prune",
      "expanded_secret_len_reject"
    ]
  }));
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
