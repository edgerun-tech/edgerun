#!/usr/bin/env node

const fs = require("fs");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/hex-compat.wat";
const wasmPath = process.argv[3] || "/tmp/hex-compat-smoke.wasm";

function status(packed) {
  return Number(packed & 0xffffffffn);
}

function value(packed) {
  return Number((packed >> 32n) & 0xffffffffn);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function write(memory, ptr, input) {
  memory.fill(0, ptr, ptr + input.length + 64);
  memory.set(input, ptr);
}

function callDecode(exports, input, outCap = 64) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  const bytes = Buffer.from(input, "ascii");
  write(memory, inPtr, bytes);
  memory.fill(0, outPtr, outPtr + 64);
  const packed = exports.hex_decode_compat(inPtr, bytes.length, outPtr, outCap);
  const written = value(packed);
  return {
    status: status(packed),
    written,
    output: Buffer.from(memory.slice(outPtr, outPtr + written)),
  };
}

function callParseU32(exports, input) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const bytes = Buffer.from(input, "ascii");
  write(memory, inPtr, bytes);
  const packed = exports.hex_parse_u32(inPtr, bytes.length);
  return { status: status(packed), value: value(packed) };
}

function callScan(exports, name, input) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  const bytes = Buffer.from(input, "ascii");
  write(memory, inPtr, bytes);
  memory.fill(0, outPtr, outPtr + 6);
  const scanStatus = exports[name](inPtr, bytes.length, outPtr);
  return {
    status: scanStatus,
    output: Buffer.from(memory.slice(outPtr, outPtr + 6)),
  };
}

(async () => {
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

  const module = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {});
  const exports = module.instance.exports;

  assert(exports.proto_abi_version() === 2, "unexpected ABI version");
  assert(exports.proto_standard_id() === 300062, "unexpected standard id");

  for (const input of ["0102abcd", "0x0102abcd", "0X0102ABCD", "102abcd"]) {
    const decoded = callDecode(exports, input);
    assert(decoded.status === 0, `decode failed for ${input}`);
    assert(decoded.written === 4, `decode length mismatch for ${input}`);
    assert(
      decoded.output.equals(Buffer.from([0x01, 0x02, 0xab, 0xcd])),
      `decode bytes mismatch for ${input}`,
    );
  }

  const empty = callDecode(exports, "");
  assert(empty.status === 0, "empty decode should succeed");
  assert(empty.written === 0, "empty decode should write no bytes");

  const invalid = callDecode(exports, "xyz");
  assert(invalid.status === 3, "invalid hex should fail");

  const short = callDecode(exports, "0102abcd", 3);
  assert(short.status === 2, "short output should fail");
  assert(short.written === 0, "short output should not report bytes");

  for (const [input, expected] of [
    ["ff", 0xff],
    ["0xff", 0xff],
    ["0XFF", 0xff],
    ["00001234", 0x1234],
    ["deadbeef", 0xdeadbeef],
  ]) {
    const parsed = callParseU32(exports, input);
    assert(parsed.status === 0, `parse_u32 failed for ${input}`);
    assert(parsed.value === expected, `parse_u32 mismatch for ${input}`);
  }

  assert(callParseU32(exports, "").status === 3, "empty parse_u32 should fail");
  assert(callParseU32(exports, "100000000").status === 3, "overflow should fail");

  const mac = callScan(exports, "mac_scan", "00:11:22:33:44:55");
  assert(mac.status === 0, "mac scan failed");
  assert(
    mac.output.equals(Buffer.from([0x00, 0x11, 0x22, 0x33, 0x44, 0x55])),
    "mac bytes mismatch",
  );

  const bdaddr = callScan(exports, "bdaddr_scan", "00:11:22:33:44:55");
  assert(bdaddr.status === 0, "bdaddr scan failed");
  assert(
    bdaddr.output.equals(Buffer.from([0x55, 0x44, 0x33, 0x22, 0x11, 0x00])),
    "bdaddr reversed bytes mismatch",
  );

  assert(callScan(exports, "mac_scan", "00:11:22:33:44").status === 3, "short mac should fail");
  assert(callScan(exports, "mac_scan", "00:11:22:33:44:zz").status === 3, "bad mac should fail");

  console.log(
    JSON.stringify(
      {
        unit: "hex-compat",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
        cases: [
          "decode_plain",
          "decode_prefixed",
          "decode_odd",
          "decode_invalid",
          "decode_output_short",
          "parse_u32",
          "parse_u32_overflow",
          "mac_scan",
          "bdaddr_scan_reversed",
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
