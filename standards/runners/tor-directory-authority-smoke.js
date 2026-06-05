#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-directory-authority/tor-directory-authority.wat");
const wasm = path.join(os.tmpdir(), `tor-directory-authority-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) {
  mem.set(bytes, ptr);
}

function take(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function u32(mem, ptr) {
  return new DataView(mem.buffer).getUint32(ptr, true);
}

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300218);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_da_max_descriptors(), 128);
  assert.equal(e.tor_da_descriptor_size(), 128);
  assert.equal(e.tor_da_consensus_entry_size(), 96);
  assert.equal(e.tor_da_init(), 0);

  assert.equal(e.tor_da_descriptor_upload_path(1000), 12);
  assert.equal(take(mem, 1000, 12).toString("ascii"), "/tor/server/");
  assert.equal(e.tor_da_consensus_path(1020, 0), 34);
  assert.equal(take(mem, 1020, 34).toString("ascii"), "/tor/status-vote/current/consensus");
  assert.equal(e.tor_da_vote_path(1080), 31);
  assert.equal(take(mem, 1080, 31).toString("ascii"), "/tor/status-vote/next/authority");

  const id1 = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x10 + i));
  const id2 = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x40 + i));
  const id3 = Buffer.from(Array.from({ length: 32 }, (_, i) => 0x80 + i));
  const nick1 = Buffer.from("guardrelay", "ascii");
  const nick2 = Buffer.from("exitrelay", "ascii");
  const nick3 = Buffer.from("thirdrelay", "ascii");
  put(mem, 2000, id1); put(mem, 2040, id2); put(mem, 2080, id3);
  put(mem, 3000, nick1); put(mem, 3040, nick2); put(mem, 3080, nick3);

  // proto bits: bit0 DirCache, bit5 Exit-capable local policy.
  assert.equal(e.tor_da_accept_descriptor(2000, 3000, nick1.length, 4096, 240, 100, 9001, 9030, 0x01, 0xaaa1, 1000, 0), 0);
  assert.equal(e.tor_da_accept_descriptor(2040, 3040, nick2.length, 512, 20, 101, 9002, 0, 0x20, 0xaaa1, 1000, 0), 0);
  assert.equal(e.tor_da_descriptor_count(), 2);

  const p1 = e.tor_da_descriptor_ptr(2000);
  assert.notEqual(p1, 0);
  const flags1 = u32(mem, p1 + 4);
  assert.equal(Boolean(flags1 & 1), true);   // Valid
  assert.equal(Boolean(flags1 & 2), true);   // Running
  assert.equal(Boolean(flags1 & 4), true);   // Stable
  assert.equal(Boolean(flags1 & 8), true);   // Fast
  assert.equal(Boolean(flags1 & 16), true);  // Guard
  assert.equal(Boolean(flags1 & 64), true);  // HSDir
  assert.equal(Boolean(flags1 & 128), true); // V2Dir
  assert.deepEqual(take(mem, p1 + 36, 32), id1);
  assert.equal(take(mem, p1 + 72, nick1.length).toString("ascii"), "guardrelay");

  const p2 = e.tor_da_descriptor_ptr(2040);
  const flags2 = u32(mem, p2 + 4);
  assert.equal(Boolean(flags2 & 32), true); // Exit

  assert.equal(e.tor_da_required_protocols_ok(0x21, 0x01), 1);
  assert.equal(e.tor_da_required_protocols_ok(0x20, 0x01), 0);

  assert.equal(e.tor_da_build_consensus(120, 180, 240), 2);
  const c0 = e.tor_da_consensus_entry_ptr(0);
  assert.deepEqual(take(mem, c0, 32), id1);
  assert.equal(u32(mem, c0 + 32), 0xaaa1);
  assert.equal(u32(mem, c0 + 44), 120);
  assert.equal(u32(mem, c0 + 48), 180);
  assert.equal(u32(mem, c0 + 52), 240);
  assert.equal(u32(mem, c0 + 60), 9001);
  assert.equal(u32(mem, c0 + 64), 9030);
  assert.equal(take(mem, c0 + 68, nick1.length).toString("ascii"), "guardrelay");

  assert.equal(e.tor_da_route_lookup(2000, 4000), 0);
  assert.equal(u32(mem, 4000), 0xaaa1);
  assert.equal(e.tor_da_route_lookup(2080, 4000), 1);

  assert.equal(e.tor_da_accept_descriptor(2000, 3000, nick1.length, 4096, 240, 99, 9001, 9030, 0x01, 0xaaa1, 1000, 0), -3);
  assert.equal(e.tor_da_accept_descriptor(2080, 3080, nick3.length, 100, 20, 105, 0, 0, 0, 0xaaa3, 1000, 0), -4);
  assert.equal(e.tor_da_accept_descriptor(2080, 3080, 20, 100, 20, 105, 9003, 0, 0, 0xaaa3, 1000, 0), -1);

  // More than two relays sharing one next_hop gets Sybil treatment for the third.
  assert.equal(e.tor_da_accept_descriptor(2080, 3080, nick3.length, 100, 20, 106, 9003, 0, 0, 0xaaa1, 1000, 0), 0);
  const p3 = e.tor_da_descriptor_ptr(2080);
  const flags3 = u32(mem, p3 + 4);
  assert.equal(Boolean(flags3 & 1024), true);
  assert.equal(Boolean(flags3 & 1), false);

  console.log("tor directory authority smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
