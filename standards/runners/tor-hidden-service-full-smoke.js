#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const hsWat = path.join(root, "standards/build/wasm/app-primitives/tor-hidden-service-full/tor-hidden-service-full.wat");
const specWat = path.join(root, "standards/build/wasm/app-primitives/tor-full-spec/tor-full-spec.wat");
const hsWasm = path.join(os.tmpdir(), `tor-hidden-service-full-${process.pid}.wasm`);
const specWasm = path.join(os.tmpdir(), `tor-full-spec-${process.pid}.wasm`);

execFileSync("wat2wasm", [hsWat, "-o", hsWasm], { stdio: "inherit" });
execFileSync("wasm-validate", [hsWasm], { stdio: "inherit" });
execFileSync("wat2wasm", [specWat, "-o", specWasm], { stdio: "inherit" });
execFileSync("wasm-validate", [specWasm], { stdio: "inherit" });

function readCString(mem, ptr) {
  let end = ptr;
  while (mem[end] !== 0) end += 1;
  return Buffer.from(mem.slice(ptr, end)).toString("utf8");
}

function put(mem, ptr, textOrBytes) {
  const bytes = typeof textOrBytes === "string" ? Buffer.from(textOrBytes) : textOrBytes;
  mem.set(bytes, ptr);
  return bytes.length;
}

(async () => {
  const { instance: hsInstance } = await WebAssembly.instantiate(fs.readFileSync(hsWasm), {});
  const hs = hsInstance.exports;
  const mem = new Uint8Array(hs.memory.buffer);

  assert.equal(hs.proto_standard_id(), 300207);
  assert.equal(hs.proto_abi_version(), 1);
  assert.equal(hs.tor_hs_full_init(), 0);
  assert.equal(hs.tor_hs_onion_addr_len(), 62);
  assert.equal(hs.tor_hs_desc_lifetime_minutes(), 180);
  assert.equal(hs.tor_hs_desc_max_bytes(), 50000);
  assert.equal(hs.tor_hs_app_memory_base(), 32768);
  assert.equal(hs.tor_hs_app_memory_bytes(), 131072);

  const onion = "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";
  put(mem, 1800, onion);
  assert.equal(hs.tor_hs_validate_onion_address(1800, onion.length), 0);
  mem[1800] = "0".charCodeAt(0);
  assert.equal(hs.tor_hs_validate_onion_address(1800, onion.length), -1);

  const blinded = "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345";
  put(mem, 2048, blinded);
  const fetchLen = hs.tor_hs_build_fetch_path(3000, 2048, 56);
  assert.equal(Buffer.from(mem.slice(3000, 3000 + fetchLen)).toString(), `/tor/hs/3/${blinded.slice(0, 56)}`);
  const publishLen = hs.tor_hs_build_publish_path(3200);
  assert.equal(Buffer.from(mem.slice(3200, 3200 + publishLen)).toString(), "/tor/hs/3/publish");

  assert.equal(hs.tor_hs_schedule_republish(1000, 7), 1067);
  assert.equal(hs.tor_hs_should_republish(1066), 0);
  assert.equal(hs.tor_hs_should_republish(1067), 1);

  for (let i = 0; i < 32; i += 1) {
    mem[4000 + i] = i;
    mem[4040 + i] = 0x80 + i;
  }
  put(mem, 4080, Buffer.from([2, 20, ...Array.from({ length: 20 }, (_, i) => 0x30 + i)]));
  assert.equal(hs.tor_hs_register_intro_point(123, 4000, 4040, 4080, 22), 0);
  assert.equal(hs.tor_hs_intro_count(), 1);
  assert.notEqual(hs.tor_hs_intro_record_ptr(0), 0);

  assert.equal(hs.tor_hs_build_descriptor_record(5000, 9, 180, 1), 16);
  assert.equal(new DataView(hs.memory.buffer).getUint32(5000, true), 3);
  assert.equal(new DataView(hs.memory.buffer).getUint32(5004, true), 9);
  assert.equal(new DataView(hs.memory.buffer).getUint32(5012, true), 1);

  for (let i = 0; i < 20; i += 1) mem[6000 + i] = 0x55 + i;
  assert.equal(hs.tor_hs_build_establish_rendezvous(6200, 6000), 20);
  assert.equal(hs.tor_hs_parse_establish_rendezvous(6200, 20, 6400), 0);
  assert.deepEqual(Array.from(mem.slice(6400, 6420)), Array.from(mem.slice(6000, 6020)));

  for (let i = 0; i < 84; i += 1) mem[6600 + i] = i;
  assert.equal(hs.tor_hs_build_rendezvous1(7000, 6000, 6600, 84), 104);
  assert.equal(hs.tor_hs_parse_rendezvous1(7000, 104, 7200, 7240), 84);
  assert.deepEqual(Array.from(mem.slice(7200, 7220)), Array.from(mem.slice(6000, 6020)));
  assert.deepEqual(Array.from(mem.slice(7240, 7324)), Array.from(mem.slice(6600, 6684)));

  assert.equal(hs.tor_hs_build_intro_established(7600), 2);
  assert.equal(mem[7600], 38);
  assert.equal(hs.tor_hs_build_rendezvous_established(7600), 2);
  assert.equal(mem[7600], 39);
  assert.equal(hs.tor_hs_build_introduce_ack(7600, 1), 2);
  assert.equal(mem[7600], 40);
  assert.equal(mem[7601], 1);

  const { instance: specInstance } = await WebAssembly.instantiate(fs.readFileSync(specWasm), {});
  const spec = specInstance.exports;
  const specMem = new Uint8Array(spec.memory.buffer);
  assert.equal(spec.proto_standard_id(), 300208);
  assert.equal(spec.tor_spec_document_count(), 24);
  assert.equal(readCString(specMem, spec.tor_spec_document_name_ptr(12)), "13-onion-services.md");
  assert.equal(spec.tor_spec_has_relay_protocol(5), 1);
  assert.equal(spec.tor_spec_has_relay_protocol(6), 1);
  assert.equal(spec.tor_spec_has_hsintro_protocol(5), 1);
  assert.equal(spec.tor_spec_has_hsrend_protocol(2), 1);
  assert.equal(spec.tor_spec_relay_command_supported(40), 1);
  assert.equal(spec.tor_spec_relay_command_supported(99), 0);

  console.log("tor hidden service full smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
