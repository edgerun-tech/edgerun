#!/usr/bin/env node

const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/app-primitives/tor-cell-codec/tor-cell-codec.wat");
const wasm = path.join(os.tmpdir(), `tor-cell-codec-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });
execFileSync("wasm-validate", [wasm], { stdio: "inherit" });

function put(mem, ptr, bytes) { mem.set(bytes, ptr); }
function take(mem, ptr, len) { return Buffer.from(mem.slice(ptr, ptr + len)); }
function le32(mem, ptr) { return new DataView(mem.buffer).getUint32(ptr, true); }
function be16(buf, off) { return (buf[off] << 8) | buf[off + 1]; }
function be32(buf, off) { return (((buf[off] << 24) | (buf[off + 1] << 16) | (buf[off + 2] << 8) | buf[off + 3]) >>> 0); }

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_standard_id(), 300224);
  assert.equal(e.proto_abi_version(), 1);
  assert.equal(e.tor_cell_body_len(), 509);
  assert.equal(e.tor_cell_relay_header_len(), 11);
  assert.equal(e.tor_cell_relay_data_max(), 498);
  assert.equal(e.tor_cell_circ_id_len(3), 2);
  assert.equal(e.tor_cell_circ_id_len(4), 4);
  assert.equal(e.tor_cell_fixed_len(3), 512);
  assert.equal(e.tor_cell_fixed_len(4), 514);
  assert.equal(e.tor_cell_var_header_len(3), 5);
  assert.equal(e.tor_cell_var_header_len(4), 7);
  assert.equal(e.tor_cell_is_variable_command(7), 1);
  assert.equal(e.tor_cell_is_variable_command(128), 1);
  assert.equal(e.tor_cell_is_variable_command(10), 0);
  assert.equal(e.tor_cell_command_circ_requirement(7), 0);
  assert.equal(e.tor_cell_command_circ_requirement(10), 1);
  assert.equal(e.tor_cell_destroy_reason_none(), 0);
  assert.equal(e.tor_cell_destroy_reason_protocol(), 2);
  assert.equal(e.tor_cell_padding_command_stop(), 1);
  assert.equal(e.tor_cell_padding_command_start(), 2);
  assert.equal(e.tor_cell_sendme_version_0(), 0);
  assert.equal(e.tor_cell_sendme_version_1(), 1);
  assert.equal(e.tor_cell_relay_stream_requirement(2), 1);
  assert.equal(e.tor_cell_relay_stream_requirement(14), 0);
  assert.equal(e.tor_cell_relay_stream_requirement(5), -2);
  assert.equal(e.tor_cell_validate_relay_stream_id(2, 99), 0);
  assert.equal(e.tor_cell_validate_relay_stream_id(2, 0), -1);
  assert.equal(e.tor_cell_validate_relay_stream_id(14, 0), 0);
  assert.equal(e.tor_cell_validate_relay_stream_id(14, 99), -1);
  assert.equal(e.tor_cell_validate_relay_stream_id(5, 0), 0);
  assert.equal(e.tor_cell_validate_relay_stream_id(5, 99), 0);
  assert.equal(e.tor_cell_validate_command(4, 10, 0, 84), -1);
  assert.equal(e.tor_cell_validate_command(4, 7, 9, 4), -1);
  assert.equal(e.tor_cell_validate_command(3, 10, 0x10000, 84), -2);

  const body = Buffer.from(Array.from({ length: 84 }, (_, i) => i));
  put(mem, 1000, body);
  assert.equal(e.tor_cell_build_fixed(4, 2000, 0x01020304, 10, 1000, body.length), 514);
  const cell = take(mem, 2000, 514);
  assert.equal(be32(cell, 0), 0x01020304);
  assert.equal(cell[4], 10);
  assert.deepEqual(cell.slice(5, 5 + body.length), body);
  assert(cell.slice(5 + body.length).every((x) => x === 0));
  assert.equal(e.tor_cell_parse_fixed(4, 2000, 514, 3000), 0);
  assert.deepEqual([le32(mem, 3000), le32(mem, 3004), le32(mem, 3008), le32(mem, 3012), le32(mem, 3016)], [0x01020304, 10, 5, 509, 514]);
  assert.equal(e.tor_cell_parse_var(4, 2000, 514, 3000), -1);

  assert.equal(e.tor_cell_build_fixed(3, 3500, 0x4321, 3, 1000, 11), 512);
  const cell3 = take(mem, 3500, 512);
  assert.equal(be16(cell3, 0), 0x4321);
  assert.equal(cell3[2], 3);
  assert.equal(e.tor_cell_parse_fixed(3, 3500, 512, 3600), 0);
  assert.deepEqual([le32(mem, 3600), le32(mem, 3604), le32(mem, 3608)], [0x4321, 3, 3]);

  const versions = Buffer.from([0, 3, 0, 4, 0, 5]);
  put(mem, 4000, versions);
  new DataView(mem.buffer).setUint16(3900, 3, true);
  new DataView(mem.buffer).setUint16(3902, 4, true);
  new DataView(mem.buffer).setUint16(3904, 5, true);
  assert.equal(e.tor_cell_build_versions_body(3910, 3900, 3), 6);
  assert.deepEqual(take(mem, 3910, 6), versions);
  assert.equal(e.tor_cell_parse_versions_body(3910, 6, 3920, 4), 3);
  assert.deepEqual([new DataView(mem.buffer).getUint16(3920, true), new DataView(mem.buffer).getUint16(3922, true), new DataView(mem.buffer).getUint16(3924, true)], [3, 4, 5]);
  put(mem, 3930, Buffer.from([0, 4, 0, 5]));
  assert.equal(e.tor_cell_versions_negotiate(3910, 6, 3930, 4), 5);
  mem[3911] = 2;
  assert.equal(e.tor_cell_parse_versions_body(3910, 6, 3920, 4), -4);
  mem[3911] = 3;
  assert.equal(e.tor_cell_build_var(4, 4200, 0, 7, 4000, versions.length), 13);
  const varCell = take(mem, 4200, 13);
  assert.equal(be32(varCell, 0), 0);
  assert.equal(varCell[4], 7);
  assert.equal(be16(varCell, 5), 6);
  assert.deepEqual(varCell.slice(7), versions);
  assert.equal(e.tor_cell_parse_var(4, 4200, 13, 4400), 0);
  assert.deepEqual([le32(mem, 4400), le32(mem, 4404), le32(mem, 4408), le32(mem, 4412), le32(mem, 4416)], [0, 7, 7, 6, 13]);
  assert.equal(e.tor_cell_build_var(4, 4500, 1, 128, 4000, 3), -1);

  const data = Buffer.from("hello relay", "ascii");
  put(mem, 5000, data);
  assert.equal(e.tor_cell_build_relay_payload(5200, 2, 0, 99, 0xaabbccdd, 5000, data.length), 11 + data.length);
  const relay = take(mem, 5200, 509);
  assert.equal(relay[0], 2);
  assert.equal(be16(relay, 1), 0);
  assert.equal(be16(relay, 3), 99);
  assert.equal(new DataView(relay.buffer, relay.byteOffset + 5).getUint32(0, true), 0xaabbccdd);
  assert.equal(be16(relay, 9), data.length);
  assert.deepEqual(relay.slice(11, 11 + data.length), data);
  assert.equal(e.tor_cell_parse_relay_payload(5200, 509, 5600), 0);
  assert.deepEqual([le32(mem, 5600), le32(mem, 5604), le32(mem, 5608), le32(mem, 5612), le32(mem, 5616), le32(mem, 5620)], [2, 0, 99, 0xaabbccdd, data.length, 11]);
  assert.equal(e.tor_cell_build_relay_payload(5200, 2, 0, 0, 0, 5000, data.length), -1);
  assert.equal(e.tor_cell_build_relay_payload(5200, 14, 0, 99, 0, 5000, data.length), -1);

  assert.equal(e.tor_cell_build_create2_body(6000, 2, 1000, 84), 88);
  assert.equal(be16(take(mem, 6000, 88), 0), 2);
  assert.equal(be16(take(mem, 6000, 88), 2), 84);
  assert.equal(e.tor_cell_parse_create2_body(6000, 509, 6200), 0);
  assert.deepEqual([le32(mem, 6200), le32(mem, 6204), le32(mem, 6208), le32(mem, 6212)], [2, 4, 84, 88]);
  assert.equal(e.tor_cell_build_created2_body(6500, 1000, 64), 66);
  assert.equal(e.tor_cell_parse_created2_body(6500, 509, 6700), 0);
  assert.deepEqual([le32(mem, 6700), le32(mem, 6704), le32(mem, 6708)], [2, 64, 66]);
  mem[6500] = 0x02;
  mem[6501] = 0xff;
  assert.equal(e.tor_cell_parse_created2_body(6500, 509, 6700), -2);
  assert.equal(e.tor_cell_build_destroy_body(7000, 0), 1);
  assert.equal(mem[7000], 0);
  assert(take(mem, 7001, 508).every((x) => x === 0));
  assert.equal(e.tor_cell_parse_destroy_body(7000, 509), 0);
  assert.equal(e.tor_cell_build_destroy_body(7000, 2), 1);
  assert.equal(e.tor_cell_parse_destroy_body(7000, 509), 2);
  assert.equal(e.tor_cell_build_fixed(4, 7200, 0x11223344, 4, 7000, 1), 514);
  assert.equal(take(mem, 7200, 514)[4], 4);
  assert.equal(e.tor_cell_build_padding_negotiate_body(7600, 1, 1234, 9999, 1500), 6);
  assert.deepEqual(take(mem, 7600, 6), Buffer.from([0, 1, 0, 0, 0, 0]));
  assert.equal(e.tor_cell_parse_padding_negotiate_body(7600, 6, 1500, 7700), 0);
  assert.deepEqual([le32(mem, 7700), le32(mem, 7704), le32(mem, 7708), le32(mem, 7712), le32(mem, 7716)], [0, 1, 0, 0, 6]);
  assert.equal(e.tor_cell_build_padding_negotiate_body(7600, 2, 1000, 900, 1500), 6);
  assert.deepEqual(take(mem, 7600, 6), Buffer.from([0, 2, 0x05, 0xdc, 0x05, 0xdc]));
  mem[7602] = 0;
  mem[7603] = 10;
  mem[7604] = 0;
  mem[7605] = 9;
  assert.equal(e.tor_cell_parse_padding_negotiate_body(7600, 6, 1500, 7700), 0);
  assert.deepEqual([le32(mem, 7708), le32(mem, 7712)], [1500, 1500]);
  mem[7600] = 1;
  assert.equal(e.tor_cell_parse_padding_negotiate_body(7600, 6, 1500, 7700), -4);
  mem[7600] = 0;

  const sendmeDigest = Buffer.from(Array.from({ length: 20 }, (_, i) => 0x40 + i));
  put(mem, 8000, sendmeDigest);
  assert.equal(e.tor_cell_build_sendme_v0(8050), 3);
  assert.deepEqual(take(mem, 8050, 3), Buffer.from([0, 0, 0]));
  assert.equal(e.tor_cell_parse_sendme_body(8050, 3, 0, 8100), 0);
  assert.deepEqual([le32(mem, 8100), le32(mem, 8104), le32(mem, 8108), le32(mem, 8112)], [0, 3, 0, 3]);
  assert.equal(e.tor_cell_parse_sendme_body(8050, 3, 1, 8100), -4);
  assert.equal(e.tor_cell_build_sendme_v1(8050, 8000), 23);
  assert.equal(e.tor_cell_parse_sendme_body(8050, 23, 1, 8100), 0);
  assert.deepEqual([le32(mem, 8100), le32(mem, 8104), le32(mem, 8108), le32(mem, 8112)], [1, 3, 20, 23]);
  assert.equal(e.tor_cell_sendme_v1_digest_matches(8050, 23, 8000), 0);
  mem[8050 + 3] ^= 1;
  assert.equal(e.tor_cell_sendme_v1_digest_matches(8050, 23, 8000), -3);
  mem[8050 + 3] ^= 1;
  mem[8052] = 19;
  assert.equal(e.tor_cell_parse_sendme_body(8050, 23, 1, 8100), -2);

  console.log("tor cell codec smoke passed");
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
}).finally(() => {
  try { fs.unlinkSync(wasm); } catch {}
});
