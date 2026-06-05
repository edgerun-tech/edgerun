#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-http-frame-wat-proof-"));

process.on("exit", () => {
  fs.rmSync(tmpRoot, { recursive: true, force: true });
});

function run(cmd, args, options = {}) {
  return execFileSync(cmd, args, {
    cwd: options.cwd || root,
    encoding: options.encoding || "utf8",
    stdio: options.stdio || "pipe",
  });
}

function compileWat(name) {
  const watPath = path.join(watRoot, `${name}.wat`);
  const wasmPath = path.join(tmpRoot, `${name}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  return fs.readFileSync(wasmPath);
}

async function instantiate(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return {
    e: instance.exports,
    mem: new Uint8Array(instance.exports.memory.buffer),
    view: new DataView(instance.exports.memory.buffer),
  };
}

function writeBytes(mem, ptr, bytes) {
  mem.fill(0, ptr, ptr + Math.max(64, bytes.length + 32));
  mem.set(bytes, ptr);
}

function bytesOf(mem, ptr, len) {
  return Buffer.from(mem.slice(ptr, ptr + len)).toString("hex");
}

function unpack(packed) {
  const value = BigInt.asUintN(64, packed);
  return {
    status: Number(value & 0xffff_ffffn),
    written: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function readHttp2Record(view, ptr) {
  return {
    payload_len: view.getUint32(ptr, true),
    type_class: view.getUint32(ptr + 4, true),
    flags: view.getUint32(ptr + 8, true),
    reserved: view.getUint32(ptr + 12, true),
    stream_id_cleared: view.getUint32(ptr + 16, true),
    header_len: view.getUint32(ptr + 20, true),
    total_len: view.getUint32(ptr + 24, true),
  };
}

function readHttp3Record(view, ptr) {
  const typeLow = BigInt(view.getUint32(ptr, true));
  const typeHigh = BigInt(view.getUint32(ptr + 4, true));
  const lenLow = BigInt(view.getUint32(ptr + 8, true));
  const lenHigh = BigInt(view.getUint32(ptr + 12, true));
  return {
    frame_type: ((typeHigh << 32n) | typeLow).toString(),
    payload_len: ((lenHigh << 32n) | lenLow).toString(),
    header_len: view.getUint32(ptr + 16, true),
    type_class: view.getUint32(ptr + 20, true),
  };
}

function rustHarnessSource() {
  return String.raw`
use edgerun_protocols::http::{
    http2::frame::{Frame, FrameType},
    http3::frame::{Http3Frame, Http3FrameType},
};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn http2_type_class(frame_type: FrameType) -> u8 {
    match frame_type {
        FrameType::Data => 0,
        FrameType::Headers => 1,
        FrameType::Priority => 2,
        FrameType::RstStream => 3,
        FrameType::Settings => 4,
        FrameType::PushPromise => 5,
        FrameType::Ping => 6,
        FrameType::Goaway => 7,
        FrameType::WindowUpdate => 8,
        FrameType::Continuation => 9,
        FrameType::Extension => 255,
    }
}

fn http3_type_class(value: u64) -> u8 {
    match Http3FrameType::from_u64(value) {
        Some(Http3FrameType::Data) => 0,
        Some(Http3FrameType::Headers) => 1,
        Some(Http3FrameType::Settings) => 2,
        Some(Http3FrameType::CancelPush) => 3,
        Some(Http3FrameType::PushPromise) => 4,
        Some(Http3FrameType::MaxPushId) => 5,
        Some(Http3FrameType::Goaway) => 6,
        Some(Http3FrameType::StreamsBlocked) => 7,
        Some(Http3FrameType::Reserved) | Some(Http3FrameType::Reserved2) => 8,
        None => 9,
    }
}

fn line(key: &str, fields: &[String]) {
    println!("{}|{}", key, fields.join("|"));
}

fn h2_encode(name: &str, frame_type: FrameType, flags: u8, stream_id: u32, payload: &[u8]) {
    let bytes = Frame::new(frame_type, flags, stream_id, payload.to_vec()).to_bytes();
    line(
        &format!("h2.encode.{name}"),
        &[
            hex(&bytes),
            (payload.len() as u32).to_string(),
            http2_type_class(frame_type).to_string(),
            flags.to_string(),
            (stream_id & 0x7fffffff).to_string(),
        ],
    );
}

fn h2_decode(name: &str, bytes: &[u8], max_frame_size: u32) {
    match Frame::from_bytes(bytes, max_frame_size) {
        Ok((frame, consumed)) => {
            let raw_stream_id = frame.stream_id;
            line(
                &format!("h2.decode.{name}"),
                &[
                    "ok".to_string(),
                    frame.payload.len().to_string(),
                    http2_type_class(frame.frame_type).to_string(),
                    frame.flags.to_string(),
                    ((raw_stream_id >> 31) & 1).to_string(),
                    (raw_stream_id & 0x7fffffff).to_string(),
                    consumed.to_string(),
                ],
            );
        }
        Err(_) => line(&format!("h2.decode.{name}"), &["err".to_string()]),
    }
}

fn h3_encode(name: &str, frame: Http3Frame, frame_type: u64, payload_len: usize) {
    line(
        &format!("h3.encode.{name}"),
        &[
            hex(&frame.to_bytes()),
            frame_type.to_string(),
            payload_len.to_string(),
            http3_type_class(frame_type).to_string(),
        ],
    );
}

fn h3_decode(name: &str, bytes: &[u8], frame_type: u64, payload_len: usize, header_len: usize) {
    match Http3Frame::from_bytes(bytes) {
        Ok((_frame, consumed)) => line(
            &format!("h3.decode.{name}"),
            &[
                "ok".to_string(),
                frame_type.to_string(),
                payload_len.to_string(),
                header_len.to_string(),
                http3_type_class(frame_type).to_string(),
                consumed.to_string(),
            ],
        ),
        Err(_) => line(&format!("h3.decode.{name}"), &["err".to_string()]),
    }
}

fn main() {
    h2_encode("data", FrameType::Data, 0x01, 1, b"hello");
    h2_encode("headers", FrameType::Headers, 0x04, 0x8000_0003, &[0x82, 0x86, 0x84]);
    h2_encode("settings", FrameType::Settings, 0x00, 0, &[]);
    h2_encode("extension", FrameType::Extension, 0x7f, 7, &[0xaa, 0xbb]);

    h2_decode("data", &[0,0,5,0,1,0,0,0,1,b'h',b'e',b'l',b'l',b'o'], 16_384);
    h2_decode("reserved", &[0,0,0,1,4,0x80,0,0,3], 16_384);
    h2_decode("too_short", &[0,0,0,1,4,0,0,0], 16_384);
    h2_decode("oversize", &[0,0x40,1,0,0,0,0,0,1], 16_384);
    h2_decode("incomplete", &[0,0,5,0,0,0,0,0,1,1,2], 16_384);

    h3_encode("data", Http3Frame::Data { payload: b"hello".to_vec() }, 0, 5);
    h3_encode("headers", Http3Frame::Headers { header_block: vec![0x82, 0x86, 0x84] }, 1, 3);
    h3_encode("settings", Http3Frame::Settings { entries: vec![] }, 4, 0);
    h3_encode("unknown", Http3Frame::Unknown { frame_type: 0x41, payload: vec![0xaa, 0xbb] }, 0x41, 2);
    h3_encode("reserved", Http3Frame::Unknown { frame_type: 0x21, payload: vec![] }, 0x21, 0);

    h3_decode("data", &[0,5,b'h',b'e',b'l',b'l',b'o'], 0, 5, 2);
    h3_decode("headers", &[1,3,0x82,0x86,0x84], 1, 3, 2);
    h3_decode("settings", &[4,0], 4, 0, 2);
    h3_decode("unknown", &[0x40,0x41,2,0xaa,0xbb], 0x41, 2, 3);
    h3_decode("reserved", &[0x21,0], 0x21, 0, 2);
    h3_decode("incomplete", &[0,5,1,2], 0, 5, 2);
}
`;
}

function buildRustOracle() {
  fs.writeFileSync(
    path.join(tmpRoot, "Cargo.toml"),
    `[package]
name = "http-frame-wat-adapter-proof"
version = "0.0.0"
edition = "2024"

[dependencies]
edgerun-protocols = { path = "${path.join(root, "crates/protocol/edgerun-protocols")}", features = ["http", "http3"] }
`,
  );
  fs.mkdirSync(path.join(tmpRoot, "src"));
  fs.writeFileSync(path.join(tmpRoot, "src/main.rs"), rustHarnessSource());
  const output = run("cargo", ["run", "--quiet"], { cwd: tmpRoot });
  const map = new Map();
  for (const line of output.trim().split(/\n+/)) {
    const [key, ...fields] = line.split("|");
    map.set(key, fields);
  }
  return map;
}

function requireRust(map, key) {
  const fields = map.get(key);
  assert(fields, `missing Rust oracle row ${key}`);
  return fields;
}

function assertH2Encode(wat, rust, name, frameType, flags, streamId, payloadHex) {
  const [rustHex, payloadLen, typeClass, rustFlags, rustStreamId] = requireRust(rust, `h2.encode.${name}`);
  const payload = Buffer.from(payloadHex, "hex");
  const ptr = 1024;
  const packed = unpack(
    wat.e.http2_frame_header_encode(
      Number(payloadLen),
      frameType,
      flags,
      streamId,
      ptr,
      9,
    ),
  );
  assert.deepEqual(packed, { status: 0, written: 9 }, name);
  wat.mem.set(payload, ptr + 9);
  assert.equal(bytesOf(wat.mem, ptr, 9 + payload.length), rustHex, name);
  assert.equal(Number(typeClass), frameType <= 9 ? frameType : 255, name);
  assert.equal(Number(rustFlags), flags, name);
  assert.equal(Number(rustStreamId), streamId & 0x7fff_ffff, name);
}

function assertH2Decode(wat, rust, name, bytes, expectedStatus) {
  const ptr = 2048;
  const out = 4096;
  writeBytes(wat.mem, ptr, bytes);
  const status = wat.e.http2_frame_header_decode(ptr, bytes.length, 16_384, out);
  const rustFields = requireRust(rust, `h2.decode.${name}`);
  if (rustFields[0] === "err") {
    assert.equal(status, expectedStatus, name);
    return;
  }
  assert.equal(status, 0, name);
  const record = readHttp2Record(wat.view, out);
  assert.deepEqual(record, {
    payload_len: Number(rustFields[1]),
    type_class: Number(rustFields[2]),
    flags: Number(rustFields[3]),
    reserved: Number(rustFields[4]),
    stream_id_cleared: Number(rustFields[5]),
    header_len: 9,
    total_len: Number(rustFields[6]),
  });
}

function assertH3Encode(wat, rust, name, frameType, payloadLen, payloadHex) {
  const [rustHex, rustFrameType, rustPayloadLen] = requireRust(rust, `h3.encode.${name}`);
  assert.equal(rustFrameType, String(frameType));
  assert.equal(rustPayloadLen, String(payloadLen));
  const ptr = 1024;
  const packed = unpack(wat.e.http3_frame_header_encode(BigInt(frameType), BigInt(payloadLen), ptr, 16));
  assert.equal(packed.status, 0, name);
  wat.mem.set(Buffer.from(payloadHex, "hex"), ptr + packed.written);
  assert.equal(bytesOf(wat.mem, ptr, packed.written + Buffer.from(payloadHex, "hex").length), rustHex, name);
}

function assertH3Decode(wat, rust, name, bytes, expectedStatus) {
  const ptr = 2048;
  const out = 4096;
  writeBytes(wat.mem, ptr, bytes);
  const status = wat.e.http3_frame_header_decode(ptr, bytes.length, out);
  const rustFields = requireRust(rust, `h3.decode.${name}`);
  if (rustFields[0] === "err") {
    assert.equal(status, expectedStatus, name);
    return;
  }
  assert.equal(status, 0, name);
  const record = readHttp3Record(wat.view, out);
  assert.deepEqual(record, {
    frame_type: rustFields[1],
    payload_len: rustFields[2],
    header_len: Number(rustFields[3]),
    type_class: Number(rustFields[4]),
  });
  assert.equal(Number(rustFields[5]), Number(rustFields[3]) + Number(rustFields[2]), name);
}

(async () => {
  const rust = buildRustOracle();
  const h2 = await instantiate("http2-frame");
  const h3 = await instantiate("http3-frame");

  assert.equal(h2.e.proto_abi_version(), 2);
  assert.equal(h3.e.proto_abi_version(), 2);
  assert.equal(h2.e.proto_standard_id(), 300010);
  assert.equal(h3.e.proto_standard_id(), 300012);

  assertH2Encode(h2, rust, "data", 0, 0x01, 1, "68656c6c6f");
  assertH2Encode(h2, rust, "headers", 1, 0x04, 0x8000_0003, "828684");
  assertH2Encode(h2, rust, "settings", 4, 0x00, 0, "");
  assertH2Encode(h2, rust, "extension", 255, 0x7f, 7, "aabb");

  assertH2Decode(h2, rust, "data", [0, 0, 5, 0, 1, 0, 0, 0, 1, 0x68, 0x65, 0x6c, 0x6c, 0x6f], 0);
  assertH2Decode(h2, rust, "reserved", [0, 0, 0, 1, 4, 0x80, 0, 0, 3], 0);
  assertH2Decode(h2, rust, "too_short", [0, 0, 0, 1, 4, 0, 0, 0], 1);
  assertH2Decode(h2, rust, "oversize", [0, 0x40, 1, 0, 0, 0, 0, 0, 1], 3);
  assertH2Decode(h2, rust, "incomplete", [0, 0, 5, 0, 0, 0, 0, 0, 1, 1, 2], 1);

  assertH3Encode(h3, rust, "data", 0, 5, "68656c6c6f");
  assertH3Encode(h3, rust, "headers", 1, 3, "828684");
  assertH3Encode(h3, rust, "settings", 4, 0, "");
  assertH3Encode(h3, rust, "unknown", 0x41, 2, "aabb");
  assertH3Encode(h3, rust, "reserved", 0x21, 0, "");

  assertH3Decode(h3, rust, "data", [0, 5, 0x68, 0x65, 0x6c, 0x6c, 0x6f], 0);
  assertH3Decode(h3, rust, "headers", [1, 3, 0x82, 0x86, 0x84], 0);
  assertH3Decode(h3, rust, "settings", [4, 0], 0);
  assertH3Decode(h3, rust, "unknown", [0x40, 0x41, 2, 0xaa, 0xbb], 0);
  assertH3Decode(h3, rust, "reserved", [0x21, 0], 0);
  assertH3Decode(h3, rust, "incomplete", [0, 5, 1, 2], 5);

  console.log(
    JSON.stringify(
      {
        proof: "http-frame-wat-adapter",
        rust_oracle: "edgerun-protocols http2/http3 frame encode/decode",
        wat_modules: ["http2-frame", "http3-frame"],
        asserted_cases: 19,
        feasible_now: false,
        blocker: "HTTP/2/3 header parity is proven, but in-process Rust replacement still needs a bridge to the edgerun-c WASM runtime ABI or an approved Rust-side runtime adapter.",
        adapter_targets: [
          "crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs: Frame::to_bytes header construction",
          "crates/protocol/edgerun-protocols/src/http/http2/frame/mod.rs: Frame::from_bytes header scan and size checks",
          "crates/protocol/edgerun-protocols/src/http/http3/frame.rs: Http3Frame::to_bytes frame type/length varint header construction",
          "crates/protocol/edgerun-protocols/src/http/http3/frame.rs: Http3Frame::from_bytes frame type/length header scan and payload bounds",
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
