#!/usr/bin/env node

const assert = require("node:assert/strict");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-directory-vote/tor-directory-vote.wat");
const wasm = path.join(os.tmpdir(), `tor-directory-vote-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) { mem.set(bytes, ptr); }
function take(mem, ptr, len) { return Buffer.from(mem.slice(ptr, ptr + len)); }
function le32(mem, ptr) { return new DataView(mem.buffer).getUint32(ptr, true); }
function text(mem, ptr, len) { return take(mem, ptr, len).toString("ascii"); }

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300222);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_dir_vote_signature_record_len(), 164);
  assert.equal(e.tor_dir_vote_algorithm_sha256_ed25519(), 2);

  const document = Buffer.from([
    "network-status-version 3",
    "vote-status vote",
    "consensus-methods 33",
    "valid-after 2026-06-05 00:00:00",
    "fresh-until 2026-06-05 01:00:00",
    "valid-until 2026-06-05 03:00:00",
    "known-flags Authority Exit Fast Guard HSDir MiddleOnly Running Stable V2Dir Valid",
  ].join("\n") + "\n", "ascii");
  const identity = crypto.createHash("sha256").update("authority identity").digest();
  const signingSeed = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x90 + i));
  const expectedDigest = crypto.createHash("sha256").update(document).digest();

  put(mem, 1000, document);
  put(mem, 2000, identity);
  put(mem, 2040, signingSeed);
  assert.equal(e.tor_dir_vote_public_from_seed(2040, 2080), 0);
  const signingKey = take(mem, 2080, 32);

  assert.equal(e.tor_dir_vote_doc_digest(1000, document.length, 2120), 0);
  assert.deepEqual(take(mem, 2120, 32), expectedDigest);
  assert.equal(e.tor_dir_vote_hex32(2120, 2160), 64);
  assert.equal(text(mem, 2160, 64), expectedDigest.toString("hex"));

  assert.equal(e.tor_dir_vote_sign(1000, document.length, 2000, 2040, 2200), 164);
  assert.deepEqual(take(mem, 2204, 32), identity);
  assert.deepEqual(take(mem, 2236, 32), signingKey);
  assert.deepEqual(take(mem, 2268, 32), expectedDigest);
  assert.notEqual(take(mem, 2300, 64).toString("hex"), "00".repeat(64));
  assert.equal(e.tor_dir_vote_parse_signature_record(2200, 164, 2400), 0);
  assert.deepEqual([le32(mem, 2400), le32(mem, 2404), le32(mem, 2408), le32(mem, 2412), le32(mem, 2416), le32(mem, 2420)], [2, 4, 36, 68, 100, 164]);
  assert.equal(e.tor_dir_vote_verify(1000, document.length, 2000, 2080, 2200, 164), 0);

  let n = e.tor_dir_vote_signature_object(2200, 164, 3000);
  const sigObject = text(mem, 3000, n);
  assert(sigObject.startsWith("-----BEGIN SIGNATURE-----\n"));
  assert(sigObject.endsWith("-----END SIGNATURE-----\n"));
  assert(sigObject.includes(take(mem, 2300, 64).toString("base64")));
  n = e.tor_dir_vote_directory_signature_block(2200, 164, 3300);
  const dirSig = text(mem, 3300, n);
  assert(dirSig.startsWith(`directory-signature sha256 ${identity.toString("hex")} ${signingKey.toString("hex")}\n`));
  assert(dirSig.includes("-----BEGIN SIGNATURE-----\n"));
  n = e.tor_dir_vote_consensus_digest_line(2120, 3800);
  assert.equal(text(mem, 3800, n), `consensus-digest ${expectedDigest.toString("hex")}\n`);
  put(mem, 3900, Buffer.from("microdesc", "ascii"));
  n = e.tor_dir_vote_additional_digest_line(3900, 9, 2120, 4000);
  assert.equal(text(mem, 4000, n), `additional-digest microdesc sha256 ${expectedDigest.toString("hex")}\n`);
  n = e.tor_dir_vote_additional_signature_line(3900, 9, 2200, 164, 4300);
  assert(text(mem, 4300, n).startsWith(`additional-signature microdesc sha256 ${identity.toString("hex")} ${signingKey.toString("hex")}\n`));
  put(mem, 4900, Buffer.from("2026-06-05 00:00:00", "ascii"));
  put(mem, 4930, Buffer.from("2026-06-05 01:00:00", "ascii"));
  put(mem, 4960, Buffer.from("2026-06-05 03:00:00", "ascii"));
  n = e.tor_dir_vote_detached_timing_block(4900, 19, 4930, 19, 4960, 19, 5000);
  assert.equal(text(mem, 5000, n), "valid-after 2026-06-05 00:00:00\nfresh-until 2026-06-05 01:00:00\nvalid-until 2026-06-05 03:00:00\n");

  mem[1000 + 5] ^= 1;
  assert.equal(e.tor_dir_vote_verify(1000, document.length, 2000, 2080, 2200, 164), -3);
  mem[1000 + 5] ^= 1;
  mem[2300] ^= 1;
  assert.equal(e.tor_dir_vote_verify(1000, document.length, 2000, 2080, 2200, 164), -3);
  mem[2300] ^= 1;
  mem[2204] ^= 1;
  assert.equal(e.tor_dir_vote_verify(1000, document.length, 2000, 2080, 2200, 164), -3);
  mem[2204] ^= 1;
  mem[2203] = 7;
  assert.equal(e.tor_dir_vote_parse_signature_record(2200, 164, 2400), -4);
  mem[2203] = 2;

  assert.equal(e.tor_dir_vote_median3(30, 10, 20), 20);
  assert.equal(e.tor_dir_vote_low_median3(3, 3, 9), 3);
  assert.equal(e.tor_dir_vote_params_low_median3(100, 40, 70), 70);
  assert.equal(e.tor_dir_vote_known_flags_union(0x01, 0x80, 0x10), 0x91);
  assert.equal(e.tor_dir_vote_timing_median3(30, 10, 20, 180, 120, 150, 300, 360, 330, 2500), 0);
  assert.deepEqual([le32(mem, 2500), le32(mem, 2504), le32(mem, 2508)], [20, 150, 330]);

  assert.equal(e.tor_dir_vote_threshold(3), 3);
  assert.equal(e.tor_dir_vote_threshold(5), 4);
  put(mem, 2600, Buffer.from([31, 32, 33, 44]));
  put(mem, 2610, Buffer.from([32, 33, 44]));
  put(mem, 2620, Buffer.from([33, 44, 50]));
  assert.equal(e.tor_dir_vote_method_mask(2600, 4, 2700), 0);
  assert.equal(e.tor_dir_vote_method_mask(2610, 3, 2708), 0);
  assert.equal(e.tor_dir_vote_method_mask(2620, 3, 2716), 0);
  assert.equal(e.tor_dir_vote_choose_method_masks(2700, 3), 44);
  mem[2620 + 1] = 45;
  assert.equal(e.tor_dir_vote_method_mask(2620, 3, 2716), 0);
  assert.equal(e.tor_dir_vote_choose_method_masks(2700, 3), 33);

  new DataView(mem.buffer).setUint32(2800, 0b1111, true);
  new DataView(mem.buffer).setUint32(2804, 0b0111, true);
  new DataView(mem.buffer).setUint32(2808, 0b0011, true);
  assert.equal(e.tor_dir_vote_flag_consensus(2800, 3), 0b0011);
  [90, 10, 70, 40].forEach((v, i) => new DataView(mem.buffer).setUint32(2900 + i * 4, v, true));
  assert.equal(e.tor_dir_vote_median_u32(2900, 4), 70);
  assert.equal(e.tor_dir_vote_low_median_u32(2900, 4), 40);

  console.log("tor directory vote smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
