const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "..", "..");
const wat = path.join(root, "standards", "build", "wasm", "codec-primitives", "crypto-ecdsa-der.wat");
const wasm = path.join(os.tmpdir(), `crypto-ecdsa-der-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });
execFileSync("wasm-validate", [wasm], { stdio: "pipe" });

const bytes = fs.readFileSync(wasm);

function mem(e) {
  return new Uint8Array(e.memory.buffer);
}

function write(e, ptr, data) {
  mem(e).set(data, ptr);
}

function read(e, ptr, len) {
  return Buffer.from(mem(e).slice(ptr, ptr + len));
}

function status(v) {
  return Number(v & 0xffffffffn);
}

function high(v) {
  return Number((v >> 32n) & 0xffffffffn);
}

function derLen(n) {
  if (n < 0x80) return Buffer.from([n]);
  if (n <= 0xff) return Buffer.from([0x81, n]);
  return Buffer.from([0x82, n >> 8, n & 0xff]);
}

function derInt(raw) {
  const pad = raw[0] & 0x80 ? Buffer.from([0]) : Buffer.alloc(0);
  const body = Buffer.concat([pad, raw]);
  return Buffer.concat([Buffer.from([0x02]), derLen(body.length), body]);
}

function derSig(r, s) {
  const body = Buffer.concat([derInt(r), derInt(s)]);
  return Buffer.concat([Buffer.from([0x30]), derLen(body.length), body]);
}

function parse(e, sig, rCap = 80, sCap = 80) {
  const sigPtr = 1024;
  const rPtr = 4096;
  const sPtr = 8192;
  write(e, sigPtr, sig);
  const ret = e.ecdsa_der_parse(sigPtr, sig.length, rPtr, rCap, sPtr, sCap);
  return {
    status: status(ret),
    packed: high(ret),
    r: read(e, rPtr, e.ecdsa_der_last_r_len()),
    s: read(e, sPtr, e.ecdsa_der_last_s_len()),
  };
}

function emit(e, r, s, outCap = 200) {
  const rPtr = 12288;
  const sPtr = 14336;
  const outPtr = 16384;
  write(e, rPtr, r);
  write(e, sPtr, s);
  const ret = e.ecdsa_der_emit(rPtr, r.length, sPtr, s.length, outPtr, outCap);
  return {
    status: status(ret),
    written: high(ret),
    out: read(e, outPtr, high(ret)),
  };
}

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300082);

  const smallR = Buffer.from([0x01, 0x02, 0x03]);
  const smallS = Buffer.from([0x04, 0x05]);
  const smallSig = derSig(smallR, smallS);
  let p = parse(e, smallSig);
  assert.equal(p.status, 0);
  assert.equal(p.packed, (smallR.length << 16) | smallS.length);
  assert.deepEqual(p.r, smallR);
  assert.deepEqual(p.s, smallS);

  let emitted = emit(e, smallR, smallS);
  assert.equal(emitted.status, 0);
  assert.deepEqual(emitted.out, smallSig);

  const highR = Buffer.from([0x80, 0xaa, 0xbb]);
  const highS = Buffer.from([0x7f, 0xcc]);
  const highSig = derSig(highR, highS);
  assert.deepEqual(highSig.slice(3, 8), Buffer.from([0x04, 0x00, 0x80, 0xaa, 0xbb]));
  p = parse(e, highSig);
  assert.equal(p.status, 0);
  assert.deepEqual(p.r, highR);
  assert.deepEqual(p.s, highS);
  emitted = emit(e, highR, highS);
  assert.equal(emitted.status, 0);
  assert.deepEqual(emitted.out, highSig);

  const longR = Buffer.alloc(72, 0x11);
  const longS = Buffer.alloc(72, 0x22);
  const longSig = derSig(longR, longS);
  assert.equal(longSig[1], 0x81);
  p = parse(e, longSig);
  assert.equal(p.status, 0);
  assert.deepEqual(p.r, longR);
  assert.deepEqual(p.s, longS);
  emitted = emit(e, longR, longS);
  assert.equal(emitted.status, 0);
  assert.deepEqual(emitted.out, longSig);

  const negative = Buffer.from([0x30, 0x06, 0x02, 0x01, 0x80, 0x02, 0x01, 0x01]);
  assert.equal(parse(e, negative).status, 3);

  const nonMinimalZero = Buffer.from([0x30, 0x07, 0x02, 0x02, 0x00, 0x7f, 0x02, 0x01, 0x01]);
  assert.equal(parse(e, nonMinimalZero).status, 3);

  const wrongSeqLen = Buffer.from(smallSig);
  wrongSeqLen[1] ^= 0x01;
  assert.equal(parse(e, wrongSeqLen).status, 3);

  assert.equal(parse(e, smallSig, 2, 80).status, 2);
  assert.equal(emit(e, smallR, smallS, smallSig.length - 1).status, 2);
  assert.equal(emit(e, Buffer.concat([Buffer.from([0]), smallR]), smallS).status, 3);

  fs.unlinkSync(wasm);
  console.log(JSON.stringify({
    unit: "crypto-ecdsa-der",
    abi_version: e.proto_abi_version(),
    standard_id: e.proto_standard_id(),
    ok: true,
    cases: [
      "parse_small",
      "emit_small",
      "high_bit_r_pad",
      "long_sequence_len",
      "reject_negative_integer",
      "reject_non_minimal_leading_zero",
      "reject_wrong_sequence_length",
      "output_short"
    ]
  }));
})().catch((err) => {
  try {
    fs.unlinkSync(wasm);
  } catch (_) {}
  console.error(err);
  process.exit(1);
});
