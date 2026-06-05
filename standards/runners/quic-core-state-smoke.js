#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath =
  process.argv[2] ||
  path.join(root, "standards/build/wasm/codec-primitives/quic-core-state.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "quic-core-state-"));
const wasmPath = path.join(tmpDir, "quic-core-state.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function unpack(value) {
  const bits = BigInt.asUintN(64, value);
  return {
    status: Number(bits & 0xffff_ffffn),
    high: Number((bits >> 32n) & 0xffff_ffffn),
  };
}

function writeBytes(mem, offset, bytes) {
  mem.fill(0, offset, offset + Math.max(bytes.length + 16, 32));
  mem.set(bytes, offset);
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300104);

  assert.equal(e.quic_packet_type_from_first(0xc0), 0);
  assert.equal(e.quic_packet_type_from_first(0xd0), 1);
  assert.equal(e.quic_packet_type_from_first(0xe0), 2);
  assert.equal(e.quic_packet_type_from_first(0xf1), 3);
  assert.equal(e.quic_packet_type_from_first(0x40), 4);
  assert.equal(e.quic_packet_form_flags(0xc0), 0x03);
  assert.equal(e.quic_packet_form_flags(0xf1), 0x13);
  assert.equal(e.quic_packet_form_flags(0x44), 0x0d);
  assert.equal(e.quic_packet_form_flags(0x00), 0x04);
  assert.equal(e.quic_packet_number_length(0xcf), 4);
  assert.equal(e.quic_packet_number_length_for_value(0xffn), 1);
  assert.equal(e.quic_packet_number_length_for_value(0x100n), 2);
  assert.equal(e.quic_packet_number_length_for_value(0x1_0000n), 3);
  assert.equal(e.quic_packet_number_length_for_value(0x1_000000n), 4);

  const varints = [
    { bytes: [0x00], value: 0n, len: 1 },
    { bytes: [0x3f], value: 63n, len: 1 },
    { bytes: [0x40, 0x40], value: 64n, len: 2 },
    { bytes: [0x7f, 0xff], value: 16_383n, len: 2 },
    { bytes: [0x80, 0x00, 0x40, 0x00], value: 16_384n, len: 4 },
    {
      bytes: [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
      value: 4_611_686_018_427_387_903n,
      len: 8,
    },
  ];

  for (const item of varints) {
    writeBytes(mem, 1024, item.bytes);
    assert.deepEqual(unpack(e.quic_varint_decode_at(1024, item.bytes.length, 0, 2048)), {
      status: 0,
      high: item.len,
    });
    assert.equal(view.getBigUint64(2048, true), item.value);
  }
  writeBytes(mem, 1024, [0x40]);
  assert.deepEqual(unpack(e.quic_varint_decode_at(1024, 1, 0, 2048)), { status: 5, high: 0 });
  assert.deepEqual(unpack(e.quic_varint_decode_at(1024, 0, 0, 2048)), { status: 1, high: 0 });

  assert.equal(e.quic_transport_param_classify(0x01n), 0);
  assert.equal(e.quic_transport_param_classify(0x03n), 1);
  assert.equal(e.quic_transport_param_classify(0x0an), 2);
  assert.equal(e.quic_transport_param_classify(0x0bn), 3);
  assert.equal(e.quic_transport_param_classify(0x0en), 4);
  assert.equal(e.quic_transport_param_classify(0x1bn), 8);
  assert.equal(e.quic_transport_param_classify(0x00n), 9);
  assert.equal(e.quic_transport_param_classify(0x40n), 10);

  const frameCases = [
    [0x00, 0],
    [0x01, 1],
    [0x02, 2],
    [0x03, 3],
    [0x04, 4],
    [0x05, 5],
    [0x06, 6],
    [0x07, 7],
    [0x08, 8],
    [0x0f, 8],
    [0x10, 9],
    [0x11, 10],
    [0x12, 11],
    [0x13, 12],
    [0x14, 13],
    [0x15, 14],
    [0x16, 15],
    [0x17, 16],
    [0x18, 17],
    [0x19, 18],
    [0x1a, 19],
    [0x1b, 20],
    [0x1c, 21],
    [0x1d, 22],
    [0x1e, 23],
    [0x1f, 255],
  ];
  for (const [frameType, cls] of frameCases) {
    assert.equal(e.quic_frame_type_classify(frameType), cls);
  }

  assert.equal(e.quic_crypto_level_for_packet_type(0), 0);
  assert.equal(e.quic_crypto_level_for_packet_type(2), 1);
  assert.equal(e.quic_crypto_level_for_packet_type(1), 2);
  assert.equal(e.quic_crypto_level_for_packet_type(4), 2);
  assert.equal(e.quic_crypto_level_for_packet_type(3), 3);
  assert.equal(e.quic_packet_number_space_for_packet_type(0), 0);
  assert.equal(e.quic_packet_number_space_for_packet_type(2), 1);
  assert.equal(e.quic_packet_number_space_for_packet_type(4), 2);

  assert.equal(e.quic_tls_message_classify(2), 0);
  assert.equal(e.quic_tls_message_classify(8), 1);
  assert.equal(e.quic_tls_message_classify(11), 2);
  assert.equal(e.quic_tls_message_classify(15), 3);
  assert.equal(e.quic_tls_message_classify(20), 4);
  assert.equal(e.quic_tls_message_classify(99), 9);

  let state = 0;
  for (const event of [1, 2, 8, 11, 15, 20, 21]) {
    state = e.quic_handshake_transition(state, event);
  }
  assert.equal(state, 7);
  assert.equal(e.quic_handshake_transition(2, 20), 100);
  assert.equal(e.quic_handshake_transition(4, 0), 4);

  assert.equal(e.quic_transport_default_u64(0), 65_535n);
  assert.equal(e.quic_transport_default_u64(1), 65_535n);
  assert.equal(e.quic_transport_default_u64(2), 1_200n);
  assert.equal(e.quic_transport_default_u64(3), 100_000n);
  assert.equal(e.quic_transport_default_u64(4), 50_000n);

  console.log(
    JSON.stringify(
      {
        unit: "quic-core-state",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
        cases: [
          "packet-type-form-flags",
          "packet-number-lengths",
          "quic-varint-status",
          "transport-parameter-classification",
          "frame-type-classification",
          "crypto-level-pn-space",
          "handshake-state-transition",
          "transport-defaults",
        ],
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
