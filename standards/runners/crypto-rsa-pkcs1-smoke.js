const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "..", "..");
const wat = path.join(root, "standards", "build", "wasm", "codec-primitives", "crypto-rsa-pkcs1.wat");
const wasm = path.join(os.tmpdir(), `crypto-rsa-pkcs1-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });
execFileSync("wasm-validate", [wasm], { stdio: "pipe" });

const bytes = fs.readFileSync(wasm);

const PREFIX = {
  256: Buffer.from("3031300d060960864801650304020105000420", "hex"),
  384: Buffer.from("3041300d060960864801650304020205000430", "hex"),
  512: Buffer.from("3051300d060960864801650304020305000440", "hex"),
};

const DIGEST_LEN = {
  256: 32,
  384: 48,
  512: 64,
};

function memory(exports) {
  return new Uint8Array(exports.memory.buffer);
}

function write(exports, ptr, data) {
  memory(exports).set(data, ptr);
}

function read(exports, ptr, len) {
  return Buffer.from(memory(exports).slice(ptr, ptr + len));
}

function packStatus(v) {
  return Number(v & 0xffffffffn);
}

function packWritten(v) {
  return Number((v >> 32n) & 0xffffffffn);
}

function digest(alg) {
  return Buffer.alloc(DIGEST_LEN[alg], alg & 0xff);
}

function encoded(alg, d, len = DIGEST_LEN[alg] + PREFIX[alg].length + 11) {
  const psLen = len - PREFIX[alg].length - d.length - 3;
  return Buffer.concat([
    Buffer.from([0x00, 0x01]),
    Buffer.alloc(psLen, 0xff),
    Buffer.from([0x00]),
    PREFIX[alg],
    d,
  ]);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300079);

  const digestPtr = 1024;
  const emPtr = 4096;
  const outPtr = 8192;

  for (const alg of [256, 384, 512]) {
    const d = digest(alg);
    const em = encoded(alg, d, 128);
    write(e, digestPtr, d);
    write(e, emPtr, em);
    assert.equal(e.rsa_pkcs1_v15_verify(alg, digestPtr, d.length, emPtr, em.length), 0);

    const emitted = e.rsa_pkcs1_v15_emit(alg, digestPtr, d.length, outPtr, em.length);
    assert.equal(packStatus(emitted), 0);
    assert.equal(packWritten(emitted), em.length);
    assert.deepEqual(read(e, outPtr, em.length), em);
    assert.equal(e.rsa_pkcs1_v15_verify(alg, digestPtr, d.length, outPtr, em.length), 0);
  }

  const d256 = digest(256);
  const em256 = encoded(256, d256, 128);
  write(e, digestPtr, d256);

  const wrongDigest = Buffer.from(em256);
  wrongDigest[wrongDigest.length - 1] ^= 0x01;
  write(e, emPtr, wrongDigest);
  assert.equal(e.rsa_pkcs1_v15_verify(256, digestPtr, d256.length, emPtr, wrongDigest.length), 3);

  const shortPadding = encoded(256, d256, PREFIX[256].length + d256.length + 10);
  write(e, emPtr, shortPadding);
  assert.equal(e.rsa_pkcs1_v15_verify(256, digestPtr, d256.length, emPtr, shortPadding.length), 2);

  const badHeader = Buffer.from(em256);
  badHeader[1] = 0x02;
  write(e, emPtr, badHeader);
  assert.equal(e.rsa_pkcs1_v15_verify(256, digestPtr, d256.length, emPtr, badHeader.length), 3);

  write(e, emPtr, em256);
  assert.equal(e.rsa_pkcs1_v15_verify(999, digestPtr, d256.length, emPtr, em256.length), 1);
  assert.equal(packStatus(e.rsa_pkcs1_v15_emit(999, digestPtr, d256.length, outPtr, 128)), 1);

  assert.equal(packStatus(e.rsa_pkcs1_v15_emit(256, digestPtr, d256.length - 1, outPtr, 128)), 3);
  assert.equal(packStatus(e.rsa_pkcs1_v15_emit(256, digestPtr, d256.length, outPtr, PREFIX[256].length + d256.length + 10)), 2);

  fs.unlinkSync(wasm);
  console.log(JSON.stringify({
    ok: true,
    standard_id: e.proto_standard_id(),
    cases: [
      "sha256_valid",
      "sha384_valid",
      "sha512_valid",
      "wrong_digest_byte",
      "short_padding",
      "bad_header",
      "unsupported_alg"
    ]
  }));
})().catch((err) => {
  try {
    fs.unlinkSync(wasm);
  } catch (_) {}
  console.error(err);
  process.exit(1);
});
