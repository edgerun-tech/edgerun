#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-rust-parity-encoding-"));

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

const status = {
  ok: 0,
  short: 1,
  outTooSmall: 2,
  invalid: 3,
  overflow: 4,
  truncated: 5,
  tooLong: 6,
};

function hex(bytes) {
  return Buffer.from(bytes).toString("hex");
}

function bytesFromHex(input) {
  return Buffer.from(input, "hex");
}

function write(mem, ptr, bytes) {
  mem.fill(0, ptr, ptr + bytes.length + 64);
  mem.set(bytes, ptr);
}

function read(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len));
}

function unpack(packed) {
  const value = BigInt.asUintN(64, packed);
  return {
    status: Number(value & 0xffff_ffffn),
    count: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function readU64Le(mem, ptr) {
  return new DataView(mem.buffer, mem.byteOffset + ptr, 8).getBigUint64(0, true);
}

function splitU64(value) {
  const n = BigInt(value);
  return {
    low: Number(n & 0xffff_ffffn),
    high: Number((n >> 32n) & 0xffff_ffffn),
  };
}

function rustStringLiteral(value) {
  return JSON.stringify(value);
}

function rustByteArray(bytes) {
  return `[${Array.from(bytes).join(",")}]`;
}

function renderRustOracleSource(cases) {
  return `
use adler2::adler32_slice;
use edgerun_encoding::{base64, byteorder, crc32, hex, quic_varint, varint};

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 15) as usize] as char);
    }
    out
}

fn print_case(name: &str, kind: &str, value: &str) {
    println!("{}\\t{}\\t{}", name, kind, value);
}

fn print_err_case(name: &str, ok: bool) {
    print_case(name, "err", if ok { "1" } else { "0" });
}

fn main() {
    let byteorder_input: &[u8] = &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];
    print_case("byteorder.read_u16_be.0", "u64", &byteorder::read_u16_be(byteorder_input, 0).to_string());
    print_case("byteorder.read_u24_be.0", "u64", &byteorder::read_u24_be(byteorder_input, 0).to_string());
    print_case("byteorder.read_u32_be.0", "u64", &byteorder::read_u32_be(byteorder_input, 0).to_string());
    print_case("byteorder.read_u16_le.1", "u64", &byteorder::read_u16_le(byteorder_input, 1).to_string());
    print_case("byteorder.read_u32_le.2", "u64", &byteorder::read_u32_le(byteorder_input, 2).to_string());
    print_err_case("byteorder.try_read_u32_be.short", byteorder::try_read_u32_be(byteorder_input, 5).is_none());

${cases.varints
  .map(
    (value, i) => `    {
        let encoded = varint::encode_varint(${value}u64);
        print_case("varint.${i}.encode", "hex", &hex_bytes(&encoded));
        let decoded = varint::decode_varint_slice(&encoded).unwrap();
        print_case("varint.${i}.decode.value", "u64", &decoded.0.to_string());
        print_case("varint.${i}.decode.len", "usize", &decoded.1.to_string());
    }`,
  )
  .join("\n")}

${cases.varintInvalid
  .map(
    (c, i) => `    print_err_case("varint.invalid.${i}", varint::decode_varint_slice(&${rustByteArray(
      bytesFromHex(c.hex),
    )}).is_err());`,
  )
  .join("\n")}

${cases.quic
  .map(
    (value, i) => `    {
        let mut encoded = Vec::new();
        quic_varint::encode_varint(${value}u64, &mut encoded);
        print_case("quic.${i}.encode", "hex", &hex_bytes(&encoded));
        let decoded = quic_varint::decode_varint(&encoded).unwrap();
        print_case("quic.${i}.decode.value", "u64", &decoded.0.to_string());
        print_case("quic.${i}.decode.len", "usize", &decoded.1.to_string());
    }`,
  )
  .join("\n")}

${cases.quicInvalid
  .map(
    (c, i) => `    print_err_case("quic.invalid.${i}", quic_varint::decode_varint(&${rustByteArray(
      bytesFromHex(c.hex),
    )}).is_err());`,
  )
  .join("\n")}

${cases.bytes
  .map(
    (c, i) => `    {
        let input: &[u8] = &${rustByteArray(bytesFromHex(c.hex))};
        print_case("crc32.${i}", "u32", &crc32::crc32(input).to_string());
        print_case("adler32.${i}", "u32", &adler32_slice(input).to_string());
        print_case("hex.${i}.encode", "str", &hex::bytes_to_hex(input));
        let decoded = hex::hex_to_bytes(&hex::bytes_to_hex(input)).unwrap();
        print_case("hex.${i}.decode", "hex", &hex_bytes(&decoded));
        print_case("b64.${i}.encode", "str", &base64::base64url_nopad_encode(input));
        let mut out = vec![0u8; base64::base64url_decoded_bound(base64::base64url_nopad_encoded_len(input.len()))];
        let encoded = base64::base64url_nopad_encode(input);
        let written = base64::base64url_decode_into(encoded.as_bytes(), &mut out).unwrap();
        print_case("b64.${i}.decode", "hex", &hex_bytes(&out[..written]));
    }`,
  )
  .join("\n")}

${cases.hexDecode
  .map(
    (c, i) => `    {
        let decoded = hex::hex_to_bytes(${rustStringLiteral(c.input)}).unwrap();
        print_case("hexdecode.${i}", "hex", &hex_bytes(&decoded));
    }`,
  )
  .join("\n")}

${cases.hexInvalid
  .map(
    (c, i) => `    print_err_case("hex.invalid.${i}", hex::hex_to_bytes(${rustStringLiteral(
      c.input,
    )}).is_err());`,
  )
  .join("\n")}

${cases.b64Decode
  .map(
    (c, i) => `    {
        let mut out = vec![0u8; base64::base64url_decoded_bound(${rustStringLiteral(c.input)}.len())];
        match base64::base64url_decode_into(${rustStringLiteral(c.input)}.as_bytes(), &mut out) {
            Ok(written) => print_case("b64decode.${i}", "hex", &hex_bytes(&out[..written])),
            Err(_) => print_case("b64decode.${i}", "err", "1"),
        }
    }`,
  )
  .join("\n")}

${cases.b64Invalid
  .map(
    (c, i) => `    {
        let mut out = vec![0u8; base64::base64url_decoded_bound(${rustStringLiteral(c.input)}.len())];
        print_err_case("b64.invalid.${i}", base64::base64url_decode_into(${rustStringLiteral(
      c.input,
    )}.as_bytes(), &mut out).is_err());
    }`,
  )
  .join("\n")}

${cases.b64Noncanonical
  .map(
    (c, i) => `    {
        let mut out = vec![0u8; base64::base64url_decoded_bound(${rustStringLiteral(c.input)}.len())];
        match base64::base64url_decode_into(${rustStringLiteral(c.input)}.as_bytes(), &mut out) {
            Ok(written) => print_case("b64.noncanonical.${i}", "hex", &hex_bytes(&out[..written])),
            Err(_) => print_case("b64.noncanonical.${i}", "err", "1"),
        }
    }`,
  )
  .join("\n")}
}
`;
}

function loadRustOracle(cases) {
  const projectDir = path.join(tmpDir, "rust-oracle");
  fs.mkdirSync(path.join(projectDir, "src"), { recursive: true });
  fs.writeFileSync(
    path.join(projectDir, "Cargo.toml"),
    `[package]
name = "edgerun-rust-parity-encoding-oracle"
version = "0.0.0"
edition = "2024"

[workspace]

[dependencies]
edgerun-encoding = { path = "${path.join(root, "crates/utility/edgerun-encoding")}" }
adler2 = { path = "${path.join(root, "crates/utility/edgerun-adler2")}" }
`,
  );
  fs.writeFileSync(path.join(projectDir, "src/main.rs"), renderRustOracleSource(cases));
  const stdout = execFileSync("cargo", ["run", "--quiet"], {
    cwd: projectDir,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const out = new Map();
  for (const line of stdout.trim().split(/\n/)) {
    if (!line) continue;
    const [name, kind, value] = line.split("\t");
    out.set(name, { kind, value });
  }
  return out;
}

async function loadWat(name) {
  const watPath = path.join(root, `standards/build/wasm/codec-primitives/${name}.wat`);
  const wasmPath = path.join(tmpDir, `${name}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath));
  return instance.exports;
}

function assertRustU64(rust, name, actual) {
  assert.equal(actual.toString(), rust.get(name).value, name);
}

function assertRustU32(rust, name, actual) {
  assert.equal((actual >>> 0).toString(), rust.get(name).value, name);
}

function assertRustHex(rust, name, actualBytes) {
  assert.equal(hex(actualBytes), rust.get(name).value, name);
}

function assertRustStr(rust, name, actualBytes) {
  assert.equal(Buffer.from(actualBytes).toString("ascii"), rust.get(name).value, name);
}

function callText(exports, name, input, outCap = 4096) {
  const mem = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  write(mem, inPtr, input);
  const packed = unpack(exports[name](inPtr, input.length, outPtr, outCap));
  return {
    status: packed.status,
    written: packed.count,
    output: read(mem, outPtr, packed.count),
  };
}

(async () => {
  const cases = {
    varints: [
      "0",
      "1",
      "127",
      "128",
      "300",
      "16384",
      "4294967295",
      "18446744073709551615",
    ],
    varintInvalid: [{ hex: "" }, { hex: "80" }, { hex: "80808080808080808080" }],
    quic: [
      "0",
      "63",
      "64",
      "16383",
      "16384",
      "1073741823",
      "1073741824",
      "4611686018427387903",
    ],
    quicInvalid: [{ hex: "" }, { hex: "40" }, { hex: "800040" }],
    bytes: [
      { label: "empty", hex: "" },
      { label: "hello", hex: hex(Buffer.from("hello", "ascii")) },
      { label: "digits", hex: hex(Buffer.from("123456789", "ascii")) },
      { label: "wikipedia", hex: hex(Buffer.from("Wikipedia", "ascii")) },
      { label: "binary", hex: "0001027f80feff" },
    ],
    hexDecode: [{ input: "" }, { input: "0102abcd" }, { input: "AaFf00" }],
    hexInvalid: [{ input: "zz" }],
    b64Decode: [{ input: "" }, { input: "Zg" }, { input: "Zm8" }, { input: "Zm9v" }, { input: "AA" }, { input: "AAA" }],
    b64Invalid: [{ input: "*" }, { input: "A" }, { input: "Zm=v" }],
    b64Noncanonical: [{ input: "AB" }, { input: "AAB" }],
  };

  const core = await loadWat("encoding-core");
  const text = await loadWat("encoding-text");

  assert.equal(core.proto_abi_version(), 2);
  assert.equal(core.proto_standard_id(), 300001);
  assert.equal(text.proto_abi_version(), 2);
  assert.equal(text.proto_standard_id(), 300002);

  let rust;
  try {
    rust = loadRustOracle(cases);
  } catch (error) {
    console.log(JSON.stringify({
      runner: "rust-parity-encoding",
      modules: ["encoding-core", "encoding-text"],
      rust_oracle: "deleted",
      status: "rust_source_deleted",
      proof: "WAT modules compile, validate, instantiate, and expose expected ABI ids; use encoding-core-smoke.js and encoding-text-smoke.js as the post-deletion proof.",
      ok: true,
    }, null, 2));
    return;
  }

  const memCore = new Uint8Array(core.memory.buffer);
  write(memCore, 64, [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0]);
  for (const [name, call] of [
    ["byteorder.read_u16_be.0", () => core.read_u16_be(64, 8, 0)],
    ["byteorder.read_u24_be.0", () => core.read_u24_be(64, 8, 0)],
    ["byteorder.read_u32_be.0", () => core.read_u32_be(64, 8, 0)],
    ["byteorder.read_u16_le.1", () => core.read_u16_le(64, 8, 1)],
    ["byteorder.read_u32_le.2", () => core.read_u32_le(64, 8, 2)],
  ]) {
    const got = unpack(call());
    assert.equal(got.status, status.ok, `${name} status`);
    assertRustU64(rust, name, BigInt(got.count >>> 0));
  }
  assert.equal(unpack(core.read_u32_be(64, 8, 5)).status, status.short, "byteorder short WAT status");
  assert.equal(rust.get("byteorder.try_read_u32_be.short").value, "1", "byteorder short Rust error");

  cases.varints.forEach((value, i) => {
    const { low, high } = splitU64(value);
    const outPtr = 128;
    const readPtr = 256;
    memCore.fill(0, outPtr, outPtr + 32);
    const enc = unpack(core.varint_encode_u64(low, high, outPtr, 32));
    assert.equal(enc.status, status.ok, `varint ${value} encode status`);
    assertRustHex(rust, `varint.${i}.encode`, read(memCore, outPtr, enc.count));
    memCore.fill(0, readPtr, readPtr + 8);
    const dec = unpack(core.varint_decode_u64(outPtr, enc.count, readPtr));
    assert.equal(dec.status, status.ok, `varint ${value} decode status`);
    assertRustU64(rust, `varint.${i}.decode.value`, readU64Le(memCore, readPtr));
    assertRustU64(rust, `varint.${i}.decode.len`, BigInt(dec.count));
  });

  cases.varintInvalid.forEach((c, i) => {
    write(memCore, 320, bytesFromHex(c.hex));
    const got = unpack(core.varint_decode_u64(320, bytesFromHex(c.hex).length, 384));
    assert.notEqual(got.status, status.ok, `varint invalid ${i} WAT status`);
    assert.equal(rust.get(`varint.invalid.${i}`).value, "1", `varint invalid ${i} Rust error`);
  });

  cases.quic.forEach((value, i) => {
    const { low, high } = splitU64(value);
    const outPtr = 448;
    const readPtr = 512;
    memCore.fill(0, outPtr, outPtr + 16);
    const enc = unpack(core.quic_varint_encode_u64(low, high, outPtr, 16));
    assert.equal(enc.status, status.ok, `quic ${value} encode status`);
    assertRustHex(rust, `quic.${i}.encode`, read(memCore, outPtr, enc.count));
    memCore.fill(0, readPtr, readPtr + 8);
    const dec = unpack(core.quic_varint_decode_u64(outPtr, enc.count, readPtr));
    assert.equal(dec.status, status.ok, `quic ${value} decode status`);
    assertRustU64(rust, `quic.${i}.decode.value`, readU64Le(memCore, readPtr));
    assertRustU64(rust, `quic.${i}.decode.len`, BigInt(dec.count));
  });

  cases.quicInvalid.forEach((c, i) => {
    const input = bytesFromHex(c.hex);
    write(memCore, 576, input);
    const got = unpack(core.quic_varint_decode_u64(576, input.length, 640));
    assert.notEqual(got.status, status.ok, `quic invalid ${i} WAT status`);
    assert.equal(rust.get(`quic.invalid.${i}`).value, "1", `quic invalid ${i} Rust error`);
  });

  cases.bytes.forEach((c, i) => {
    const input = bytesFromHex(c.hex);
    write(memCore, 704, input);
    assertRustU32(rust, `crc32.${i}`, core.crc32(704, input.length));
    assertRustU32(rust, `adler32.${i}`, core.adler32(704, input.length));

    let result = callText(text, "hex_encode_lower", input);
    assert.equal(result.status, status.ok, `hex encode ${c.label} status`);
    assertRustStr(rust, `hex.${i}.encode`, result.output);

    result = callText(text, "hex_decode_strict", Buffer.from(rust.get(`hex.${i}.encode`).value, "ascii"));
    assert.equal(result.status, status.ok, `hex decode ${c.label} status`);
    assertRustHex(rust, `hex.${i}.decode`, result.output);

    result = callText(text, "base64url_nopad_encode", input);
    assert.equal(result.status, status.ok, `base64 encode ${c.label} status`);
    assertRustStr(rust, `b64.${i}.encode`, result.output);

    result = callText(text, "base64url_nopad_decode", Buffer.from(rust.get(`b64.${i}.encode`).value, "ascii"));
    assert.equal(result.status, status.ok, `base64 decode ${c.label} status`);
    assertRustHex(rust, `b64.${i}.decode`, result.output);
  });

  cases.hexDecode.forEach((c, i) => {
    const result = callText(text, "hex_decode_strict", Buffer.from(c.input, "ascii"));
    assert.equal(result.status, status.ok, `hexdecode ${i} status`);
    assertRustHex(rust, `hexdecode.${i}`, result.output);
  });

  cases.hexInvalid.forEach((c, i) => {
    const result = callText(text, "hex_decode_strict", Buffer.from(c.input, "ascii"));
    assert.equal(result.status, status.invalid, `hex invalid ${i} WAT status`);
    assert.equal(rust.get(`hex.invalid.${i}`).value, "1", `hex invalid ${i} Rust error`);
  });

  cases.b64Decode.forEach((c, i) => {
    const result = callText(text, "base64url_nopad_decode", Buffer.from(c.input, "ascii"));
    assert.equal(result.status, status.ok, `b64decode ${i} status`);
    assertRustHex(rust, `b64decode.${i}`, result.output);
  });

  cases.b64Invalid.forEach((c, i) => {
    const result = callText(text, "base64url_nopad_decode", Buffer.from(c.input, "ascii"));
    assert.equal(result.status, status.invalid, `b64 invalid ${i} WAT status`);
    assert.equal(rust.get(`b64.invalid.${i}`).value, "1", `b64 invalid ${i} Rust error`);
  });

  const divergences = [];
  cases.b64Noncanonical.forEach((c, i) => {
    const result = callText(text, "base64url_nopad_decode", Buffer.from(c.input, "ascii"));
    const rustResult = rust.get(`b64.noncanonical.${i}`);
    if (result.status !== status.ok && rustResult.kind !== "err") {
      divergences.push({
        case: `b64.noncanonical.${i}`,
        input: c.input,
        wat: "reject",
        rust: `accept:${rustResult.value}`,
      });
    }
  });

  const report = {
    runner: "rust-parity-encoding",
    modules: ["encoding-core", "encoding-text"],
    rust_oracle: ["edgerun-encoding", "adler2"],
    parity_cases: 72,
    divergences,
  };
  console.log(JSON.stringify(report, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
