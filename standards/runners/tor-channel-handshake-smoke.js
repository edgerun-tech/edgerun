#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-channel-handshake/tor-channel-handshake.wat");
const wasm = path.join(os.tmpdir(), `tor-channel-handshake-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) { mem.set(bytes, ptr); }
function take(mem, ptr, len) { return Buffer.from(mem.slice(ptr, ptr + len)); }
function u16(n) { return [(n >>> 8) & 0xff, n & 0xff]; }
function u32(n) { return [(n >>> 24) & 0xff, (n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff]; }
function le32(mem, ptr) { return new DataView(mem.buffer).getUint32(ptr, true); }

function certBody(type, exp, keyType, subject, exts = []) {
  return Buffer.concat([
    Buffer.from([1, type]),
    Buffer.from(u32(exp)),
    Buffer.from([keyType]),
    subject,
    Buffer.from([exts.length]),
    ...exts,
  ]);
}

function extSignedWith(pub) {
  return Buffer.concat([Buffer.from([0, 32, 4, 0]), pub]);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300221);
  assert.equal(e.proto_abi_version(), 1);
  put(mem, 1000, Buffer.from([0, 3, 0, 4, 0, 5]));
  put(mem, 1020, Buffer.from([0, 4, 0, 5]));
  assert.equal(e.tor_link_versions_negotiate(1000, 6, 1020, 4), 5);
  assert.equal(e.tor_link_versions_negotiate(1000, 5, 1020, 4), -1);

  const relaySeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x11 + i));
  const signSeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x41 + i));
  const linkSeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x71 + i));
  const tlsHash = crypto.createHash("sha256").update("tls cert der").digest();

  put(mem, 2000, relaySeed);
  put(mem, 2040, signSeed);
  put(mem, 2080, linkSeed);
  assert.equal(e.tor_channel_ed25519_public(2000, 2120), 0);
  assert.equal(e.tor_channel_ed25519_public(2040, 2160), 0);
  assert.equal(e.tor_channel_ed25519_public(2080, 2200), 0);
  const relayPub = take(mem, 2120, 32);
  const signPub = take(mem, 2160, 32);
  const linkPub = take(mem, 2200, 32);
  put(mem, 2240, tlsHash);

  const idBody = certBody(4, 1000, 1, signPub, [extSignedWith(relayPub)]);
  const tlsBody = certBody(5, 1000, 3, tlsHash);
  const linkBody = certBody(6, 1000, 1, linkPub);
  const idCertPtr = 3000;
  const tlsCertPtr = 3400;
  const linkCertPtr = 3800;
  put(mem, idCertPtr, idBody);
  put(mem, tlsCertPtr, tlsBody);
  put(mem, linkCertPtr, linkBody);
  assert.equal(e.tor_channel_ed25519_sign_seeded(idCertPtr, idBody.length, 2000, 5000, 5100), 0);
  assert.deepEqual(take(mem, 5100, 32), relayPub);
  const idSig = take(mem, 5000, 64);
  assert.notEqual(idSig.toString("hex"), "00".repeat(64));
  assert.equal(e.tor_channel_ed25519_verify_raw(idCertPtr, idBody.length, 5000, 5100), 0);
  assert.equal(e.tor_channel_ed25519_sign_seeded(tlsCertPtr, tlsBody.length, 2040, 5000, 5100), 0);
  assert.deepEqual(take(mem, 5100, 32), signPub);
  const signSig = take(mem, 5000, 64);
  put(mem, idCertPtr + idBody.length, idSig);
  put(mem, tlsCertPtr + tlsBody.length, signSig);
  put(mem, linkCertPtr + linkBody.length, signSig);

  const idLen = idBody.length + 64;
  const tlsLen = tlsBody.length + 64;
  const certs = Buffer.concat([
    Buffer.from([2, 4]), Buffer.from(u16(idLen)), take(mem, idCertPtr, idLen),
    Buffer.from([5]), Buffer.from(u16(tlsLen)), take(mem, tlsCertPtr, tlsLen),
  ]);
  put(mem, 7000, certs);
  assert.equal(e.tor_certs_find(7000, certs.length, 4, 7600), 0);
  assert.deepEqual([le32(mem, 7600), le32(mem, 7604), le32(mem, 7608)], [4, 4, idLen]);
  assert.equal(e.tor_ed25519_cert_signed_with(idCertPtr, idLen, 7900), 0);
  assert.deepEqual(take(mem, 7900, 32), relayPub);
  assert.equal(e.tor_ed25519_cert_verify(idCertPtr, idLen, 4, 1, 2120, 500, 7940), 0);
  assert.deepEqual(take(mem, 7940, 32), signPub);
  assert.equal(e.tor_ed25519_cert_verify(tlsCertPtr, tlsLen, 5, 3, 2160, 500, 7980), 0);
  assert.deepEqual(take(mem, 7980, 32), tlsHash);
  assert.equal(e.tor_certs_validate_responder(7000, certs.length, 2240, 500, 7800, 7840), 0);
  assert.deepEqual(take(mem, 7800, 32), relayPub);
  assert.deepEqual(take(mem, 7840, 32), signPub);

  const challenge = crypto.randomBytes(32);
  put(mem, 8000, challenge);
  assert.equal(e.tor_auth_challenge_build(8040, 8000), 36);
  assert.equal(e.tor_auth_challenge_parse(8040, 36, 8100), 0);
  assert.deepEqual(take(mem, 8100, 32), challenge);

  for (let i = 0; i < 32 * 8 + 24; i += 1) mem[9000 + i] = (i * 7) & 0xff;
  assert.equal(e.tor_authenticate_auth0003_build(9400, 9000, 9032, 9064, 9096, 9128, 9160, 9192, 9224, 9256, 2080), 360);
  assert.equal(e.tor_authenticate_auth0003_verify(9400, 360, 9000, 9032, 9064, 9096, 9128, 9160, 9192, 9224, 2200), 0);
  mem[9400 + 20] ^= 1;
  assert.equal(e.tor_authenticate_auth0003_verify(9400, 360, 9000, 9032, 9064, 9096, 9128, 9160, 9192, 9224, 2200), -3);

  assert.equal(e.tor_netinfo_build_ipv4(10000, 0, 0x7f000001, 0x0a000001, 1), 17);
  assert.deepEqual(take(mem, 10000, 17), Buffer.from([...u32(0), 4, 4, ...u32(0x7f000001), 1, 4, 4, ...u32(0x0a000001)]));

  console.log("tor channel handshake smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
