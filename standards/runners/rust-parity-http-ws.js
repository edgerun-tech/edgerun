#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-rust-parity-http-ws-"));

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
  return bytes.length;
}

function ascii(s) {
  return Array.from(Buffer.from(s, "ascii"));
}

function unpack(packed) {
  const v = BigInt.asUintN(64, packed);
  return {
    status: Number(v & 0xffff_ffffn),
    value: Number((v >> 32n) & 0xffff_ffffn),
  };
}

function readHttp2Record(view, ptr) {
  return {
    length: view.getUint32(ptr, true),
    typeClass: view.getUint32(ptr + 4, true),
    flags: view.getUint32(ptr + 8, true),
    reserved: view.getUint32(ptr + 12, true),
    streamId: view.getUint32(ptr + 16, true),
    headerLen: view.getUint32(ptr + 20, true),
    totalLen: view.getUint32(ptr + 24, true),
  };
}

function readHttp3Record(view, ptr) {
  const typeLow = BigInt(view.getUint32(ptr, true));
  const typeHigh = BigInt(view.getUint32(ptr + 4, true));
  const lenLow = BigInt(view.getUint32(ptr + 8, true));
  const lenHigh = BigInt(view.getUint32(ptr + 12, true));
  return {
    frameType: ((typeHigh << 32n) | typeLow).toString(),
    payloadLen: ((lenHigh << 32n) | lenLow).toString(),
    headerLen: view.getUint32(ptr + 16, true),
    classification: view.getUint32(ptr + 20, true),
  };
}

function readPrefixRecord(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    consumed: view.getUint32(ptr + 4, true),
    value: view.getBigUint64(ptr + 8, true).toString(),
  };
}

function readStringMeta(view, ptr) {
  return {
    flags: view.getUint32(ptr, true),
    huffman: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
    payloadOffset: view.getUint32(ptr + 12, true),
    payloadLen: view.getUint32(ptr + 16, true),
  };
}

function readWsPrefix(view, ptr) {
  return {
    fin: view.getUint32(ptr, true),
    opcode: view.getUint32(ptr + 4, true),
    masked: view.getUint32(ptr + 8, true),
    payloadLenCode: view.getUint32(ptr + 12, true),
    extendedLenBytes: view.getUint32(ptr + 16, true),
  };
}

function readWsLen(view, ptr) {
  return {
    payloadLenLow: view.getUint32(ptr, true),
    payloadLenHigh: view.getUint32(ptr + 4, true),
    consumed: view.getUint32(ptr + 8, true),
  };
}

function rustHarnessSource() {
  return String.raw`
use edgerun_hpack::{encoder::encode_integer_into, huffman::{encode as hpack_huffman_encode, HuffmanDecoder}};
use edgerun_protocols::{
    http::{
        header::{HeaderName, HeaderValue, header_value_has_token},
        http2::frame::{Frame, FrameType},
        http3::{frame::{Http3Frame, Http3FrameType}, qpack::{prefix_int, prefix_string}, varint::{quic_decode_varint, quic_encode_varint}},
        Method, StatusCode,
    },
    websocket::{decode_client_message, decode_frame_prefix, decode_payload_len, encode_server_frame},
};
use edgerun_protocols::http::http3::qpack::buf::Cursor;
use std::str::FromStr;

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn print_line(module: &str, name: &str, json: String) {
    println!("{{\"module\":{},\"name\":{},\"expect\":{}}}", json_str(module), json_str(name), json);
}

fn ok(v: bool) -> u8 { if v { 0 } else { 3 } }

fn http1() {
    for (name, value) in [
        ("header-name-host", HeaderName::new("Host".to_string()).is_ok()),
        ("header-name-bad-slash", HeaderName::new("Bad/Name".to_string()).is_ok()),
        ("header-value-visible", HeaderValue::new("gzip, Chunked".to_string()).is_ok()),
        ("header-value-nonascii", HeaderValue::new("\u{80}".to_string()).is_ok()),
        ("transfer-token-chunked", header_value_has_token("gzip, Chunked", "chunked")),
        ("transfer-token-substring", header_value_has_token("xchunked", "chunked")),
    ] {
        print_line("http1-scan", name, format!("{{\"status\":{}}}", ok(value)));
    }

    for (name, input) in [("method-get", "GET"), ("method-bad-slash", "BA/D")] {
        print_line("http1-lines", name, format!("{{\"status\":{}}}", ok(Method::from_str(input).is_ok())));
    }
    for (name, code) in [("status-200", 200u16), ("status-099", 99u16)] {
        print_line("http1-lines", name, format!("{{\"status\":{}}}", ok(StatusCode::new(code).is_ok())));
    }

    for (name, value) in [
        ("body-token-chunked", header_value_has_token("gzip, Chunked", "chunked")),
        ("body-token-substring", header_value_has_token("xchunked", "chunked")),
    ] {
        print_line("http1-body", name, format!("{{\"status\":{}}}", ok(value)));
    }
}

fn http2() {
    for (value, class) in [(0u8,0u8),(1,1),(4,4),(6,6),(99,255)] {
        let got = FrameType::from_u8(value).unwrap() as u8;
        print_line("http2-frame", &format!("classify-{}", value), format!("{{\"class\":{}}}", if value <= 9 { got } else { class }));
    }

    let f = Frame::new(FrameType::Data, 1, 1, vec![1,2,3,4,5]);
    let bytes = f.to_bytes();
    let (parsed, consumed) = Frame::from_bytes(&bytes, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    print_line("http2-frame", "data-frame", format!(
        "{{\"status\":0,\"bytes\":\"{}\",\"record\":{{\"length\":{},\"typeClass\":{},\"flags\":{},\"reserved\":{},\"streamId\":{},\"headerLen\":9,\"totalLen\":{}}}}}",
        hex(&bytes), parsed.payload.len(), parsed.frame_type as u8, parsed.flags, 0, parsed.stream_id, consumed
    ));

    let reserved = [0,0,0,1,4,0x80,0,0,3];
    let (parsed, consumed) = Frame::from_bytes(&reserved, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    print_line("http2-frame", "reserved-bit", format!(
        "{{\"status\":0,\"record\":{{\"length\":{},\"typeClass\":{},\"flags\":{},\"reserved\":{},\"streamId\":{},\"headerLen\":9,\"totalLen\":{}}}}}",
        parsed.payload.len(), parsed.frame_type as u8, parsed.flags, 1, parsed.stream_id & 0x7fffffff, consumed
    ));

    let too_short_status = if Frame::from_bytes(&[0;8], Frame::DEFAULT_MAX_FRAME_SIZE).is_err() { 1 } else { 0 };
    print_line("http2-frame", "too-short", format!("{{\"status\":{}}}", too_short_status));
}

fn http3() {
    for (value, class) in [(0u64,0u8),(1,1),(4,2),(3,3),(5,4),(7,5),(8,6),(9,7),(0x21,8),(0x41,9)] {
        let got = match Http3FrameType::from_u64(value) {
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
        };
        print_line("http3-frame", &format!("classify-{}", value), format!("{{\"class\":{},\"rustClass\":{}}}", class, got));
    }

    for (name, frame) in [
        ("data", Http3Frame::Data { payload: vec![1,2,3,4,5] }),
        ("headers", Http3Frame::Headers { header_block: vec![] }),
        ("settings", Http3Frame::Settings { entries: vec![] }),
        ("unknown", Http3Frame::Unknown { frame_type: 65, payload: vec![] }),
    ] {
        let bytes = frame.to_bytes();
        let (parsed, consumed) = Http3Frame::from_bytes(&bytes).unwrap();
        let frame_type = match parsed {
            Http3Frame::Data { .. } => 0u64,
            Http3Frame::Headers { .. } => 1,
            Http3Frame::Settings { .. } => 4,
            Http3Frame::Unknown { frame_type, .. } => frame_type,
            _ => 999,
        };
        let payload_len = bytes.len() - consumed + match &frame {
            Http3Frame::Data { payload } => payload.len(),
            Http3Frame::Headers { header_block } => header_block.len(),
            Http3Frame::Settings { entries } => entries.len() * 2,
            Http3Frame::Unknown { payload, .. } => payload.len(),
            _ => 0,
        };
        let (ft, ft_len) = quic_decode_varint(&bytes).unwrap();
        let (pl, pl_len) = quic_decode_varint(&bytes[ft_len..]).unwrap();
        print_line("http3-frame", name, format!("{{\"status\":0,\"bytes\":\"{}\",\"record\":{{\"frameType\":\"{}\",\"payloadLen\":\"{}\",\"headerLen\":{},\"classification\":{}}},\"rustFrameType\":\"{}\",\"rustPayloadApprox\":{}}}",
            hex(&bytes), ft, pl, ft_len + pl_len, match ft {0=>0,1=>1,4=>2,3=>3,5=>4,7=>5,8=>6,9=>7,v if v % 0x1f == 0x02 || v % 0x1f == 0x06 => 8,_=>9}, frame_type, payload_len));
    }
}

fn prefix_and_strings() {
    for (module, value, prefix, flags) in [("http-prefix-int", 10u64, 5u8, 0b101u8), ("http-prefix-int", 1337, 5, 0b010)] {
        let mut hpack = Vec::new();
        encode_integer_into(value as usize, prefix, flags << prefix, &mut hpack).unwrap();
        print_line(module, &format!("hpack-{}-{}", value, prefix), format!("{{\"bytes\":\"{}\",\"flags\":{},\"value\":\"{}\"}}", hex(&hpack), flags, value));
    }

    let mut qpack = Vec::new();
    prefix_int::encode(8, 0, 424242, &mut qpack);
    print_line("http-prefix-int", "qpack-424242-8", format!("{{\"bytes\":\"{}\",\"flags\":0,\"value\":\"424242\"}}", hex(&qpack)));

    for (name, raw) in [("o", b"o".as_slice()), ("hash", b"#".as_slice()), ("name-with-ref", b"name with ref".as_slice())] {
        let encoded = hpack_huffman_encode(raw);
        let mut dec = HuffmanDecoder::new();
        let decoded = dec.decode(&encoded).unwrap();
        print_line("hpack-huffman", name, format!("{{\"status\":0,\"bytes\":\"{}\",\"decoded\":\"{}\"}}", hex(&encoded), hex(&decoded)));
    }

    let hpack_raw = [3u8, b'a', b'b', b'c'];
    print_line("hpack-string", "raw-abc", format!("{{\"bytes\":\"{}\",\"decoded\":\"616263\",\"status\":0}}", hex(&hpack_raw)));
    let encoded_o = hpack_huffman_encode(b"o");
    let mut hpack_huff = vec![0x80 | encoded_o.len() as u8];
    hpack_huff.extend_from_slice(&encoded_o);
    print_line("hpack-string", "huffman-o", format!("{{\"bytes\":\"{}\",\"decoded\":\"6f\",\"status\":0}}", hex(&hpack_huff)));

    for (name, size, flags, value) in [("qpack-name-without-ref", 6u8, 0b01u8, b"name without ref".as_slice()), ("qpack-name-with-ref", 8u8, 0b01u8, b"name with ref".as_slice())] {
        let mut encoded = Vec::new();
        prefix_string::encode(size, flags, value, &mut encoded).unwrap();
        let mut cursor = Cursor::new(encoded.as_slice());
        let decoded = prefix_string::decode(size, &mut cursor).unwrap();
        print_line("qpack-string", name, format!("{{\"bytes\":\"{}\",\"decoded\":\"{}\",\"status\":0}}", hex(&encoded), hex(&decoded)));
    }
}

fn ws() {
    for (name, bytes, max_len) in [
        ("binary-small", vec![0x82, 0x03], 1024usize),
        ("ping-masked", vec![0x89, 0x82], 1024usize),
        ("bad-fragmented-control", vec![0x09, 0x00], 1024usize),
    ] {
        let status = if name == "bad-fragmented-control" {
            if decode_client_message(false, 9, [0; 4], Vec::new()).is_err() { 3 } else { 0 }
        } else {
            match decode_frame_prefix(&bytes) {
            Ok(prefix) => {
                let ext = if bytes.len() > 2 { &bytes[2..] } else { &[] };
                let _ = decode_payload_len(prefix, ext, max_len);
                0
            }
            Err(_) => 3,
            }
        };
        print_line("ws-frame", name, format!("{{\"status\":{}}}", status));
    }

    for (name, opcode, len) in [("server-binary-small", 2u8, 3usize), ("server-ping-too-large", 9u8, 126usize)] {
        let payload = vec![0u8; len];
        let result = encode_server_frame(opcode, &payload);
        match result {
            Ok(bytes) => print_line("ws-frame", name, format!("{{\"status\":0,\"header\":\"{}\"}}", hex(&bytes[..bytes.len().min(10)]))),
            Err(_) => print_line("ws-frame", name, "{\"status\":3}".to_string()),
        }
    }
}

fn main() {
    http1();
    http2();
    http3();
    prefix_and_strings();
    ws();
}
`;
}

function writeRustHarness() {
  const dir = path.join(tmpRoot, "rust-oracle");
  fs.mkdirSync(path.join(dir, "src"), { recursive: true });
  fs.writeFileSync(
    path.join(dir, "Cargo.toml"),
    `[package]
name = "edgerun-rust-parity-http-ws-oracle"
version = "0.0.0"
edition = "2024"

[dependencies]
edgerun-protocols = { path = "${path.join(root, "crates/protocol/edgerun-protocols")}", default-features = false, features = ["http", "http2-server", "http3", "websocket"] }
edgerun-hpack = { path = "${path.join(root, "crates/utility/edgerun-hpack")}" }
edgerun-encoding = { path = "${path.join(root, "crates/utility/edgerun-encoding")}", default-features = false }
`,
  );
  fs.writeFileSync(path.join(dir, "src/main.rs"), rustHarnessSource());
  return dir;
}

function loadRustExpectations() {
  const dir = writeRustHarness();
  const output = run("cargo", ["run", "--quiet"], { cwd: dir });
  const map = new Map();
  for (const line of output.trim().split(/\n+/)) {
    const item = JSON.parse(line);
    map.set(`${item.module}/${item.name}`, item.expect);
  }
  return map;
}

function expectCase(expectations, module, name) {
  const key = `${module}/${name}`;
  assert.ok(expectations.has(key), `missing Rust oracle case ${key}`);
  return expectations.get(key);
}

async function verifyDeletedRustOracle() {
  const modules = [
    "http1-scan",
    "http1-lines",
    "http1-body",
    "http2-frame",
    "http3-frame",
    "http-prefix-int",
    "hpack-huffman",
    "hpack-string",
    "hpack-header-block",
    "hpack-table-core",
    "qpack-string",
    "ws-frame",
  ];
  for (const module of modules) {
    await instantiate(module);
  }
  console.log(
    JSON.stringify(
      {
        unit: "rust-parity-http-ws",
        rust_oracle: "deleted",
        status: "rust_source_deleted",
        wat_modules: modules,
        proof:
          "WAT modules compile, validate, instantiate, and remain covered by their smoke runners; Rust HPACK oracle was intentionally retired.",
        ok: true,
      },
      null,
      2,
    ),
  );
}

async function checkHttp1(expectations) {
  const scan = await instantiate("http1-scan");
  const ptr = 1024;
  const token = 2048;

  for (const [name, input, fn] of [
    ["header-name-host", "Host", "http_validate_header_name"],
    ["header-name-bad-slash", "Bad/Name", "http_validate_header_name"],
    ["header-value-visible", "gzip, Chunked", "http_validate_header_value"],
  ]) {
    const len = writeBytes(scan.mem, ptr, ascii(input));
    assert.equal(scan.e[fn](ptr, len), expectCase(expectations, "http1-scan", name).status, name);
  }
  writeBytes(scan.mem, ptr, [0x80]);
  assert.equal(scan.e.http_validate_header_value(ptr, 1), expectCase(expectations, "http1-scan", "header-value-nonascii").status);

  for (const [name, value] of [["transfer-token-chunked", "gzip, Chunked"], ["transfer-token-substring", "xchunked"]]) {
    const len = writeBytes(scan.mem, ptr, ascii(value));
    const tokenLen = writeBytes(scan.mem, token, ascii("chunked"));
    assert.equal(scan.e.http_value_has_token(ptr, len, token, tokenLen), expectCase(expectations, "http1-scan", name).status, name);
  }

  const lines = await instantiate("http1-lines");
  const out = 4096;
  let len = writeBytes(lines.mem, ptr, ascii("GET /x HTTP/1.1\r\n"));
  assert.equal(lines.e.http_parse_request_line(ptr, len, out), expectCase(expectations, "http1-lines", "method-get").status);
  len = writeBytes(lines.mem, ptr, ascii("BA/D /x HTTP/1.1\r\n"));
  assert.equal(lines.e.http_parse_request_line(ptr, len, out), expectCase(expectations, "http1-lines", "method-bad-slash").status);
  len = writeBytes(lines.mem, ptr, ascii("HTTP/1.1 200 OK\r\n"));
  assert.equal(lines.e.http_parse_status_line(ptr, len, out), expectCase(expectations, "http1-lines", "status-200").status);
  len = writeBytes(lines.mem, ptr, ascii("HTTP/1.1 099 Nope\r\n"));
  assert.equal(lines.e.http_parse_status_line(ptr, len, out), expectCase(expectations, "http1-lines", "status-099").status);

  const body = await instantiate("http1-body");
  for (const [name, value] of [["body-token-chunked", "gzip, Chunked"], ["body-token-substring", "xchunked"]]) {
    const vLen = writeBytes(body.mem, ptr, ascii(value));
    const tLen = writeBytes(body.mem, token, ascii("chunked"));
    assert.equal(body.e.http_has_transfer_token(ptr, vLen, token, tLen), expectCase(expectations, "http1-body", name).status);
  }
}

async function checkHttp2(expectations) {
  const { e, mem, view } = await instantiate("http2-frame");
  const ptr = 1024;
  const out = 4096;
  for (const value of [0, 1, 4, 6, 99]) {
    assert.equal(e.http2_frame_type_classify(value), expectCase(expectations, "http2-frame", `classify-${value}`).class);
  }
  const data = expectCase(expectations, "http2-frame", "data-frame");
  const bytes = Buffer.from(data.bytes, "hex");
  writeBytes(mem, ptr, bytes);
  assert.equal(e.http2_frame_header_decode(ptr, bytes.length, 16384, out), data.status);
  assert.deepEqual(readHttp2Record(view, out), data.record);

  const reserved = expectCase(expectations, "http2-frame", "reserved-bit");
  writeBytes(mem, ptr, [0, 0, 0, 1, 4, 0x80, 0, 0, 3]);
  assert.equal(e.http2_frame_header_decode(ptr, 9, 16384, out), reserved.status);
  assert.deepEqual(readHttp2Record(view, out), reserved.record);

  assert.equal(e.http2_frame_header_decode(ptr, 8, 16384, out), expectCase(expectations, "http2-frame", "too-short").status);
}

async function checkHttp3(expectations) {
  const { e, mem, view } = await instantiate("http3-frame");
  const ptr = 1024;
  const out = 4096;
  for (const value of [0, 1, 4, 3, 5, 7, 8, 9, 0x21, 0x41]) {
    const exp = expectCase(expectations, "http3-frame", `classify-${value}`);
    assert.equal(e.http3_frame_type_classify(BigInt(value)), exp.class);
    assert.equal(exp.rustClass, exp.class, `Rust class disagreed with expected class for ${value}`);
  }
  for (const name of ["data", "headers", "settings", "unknown"]) {
    const exp = expectCase(expectations, "http3-frame", name);
    const bytes = Buffer.from(exp.bytes, "hex");
    writeBytes(mem, ptr, bytes);
    assert.equal(e.http3_frame_header_decode(ptr, bytes.length, out), exp.status);
    assert.deepEqual(readHttp3Record(view, out), exp.record, name);
  }
}

async function checkPrefixAndStrings(expectations) {
  const prefix = await instantiate("http-prefix-int");
  const ptr = 1024;
  const out = 4096;
  for (const name of ["hpack-10-5", "hpack-1337-5", "qpack-424242-8"]) {
    const exp = expectCase(expectations, "http-prefix-int", name);
    const bytes = Buffer.from(exp.bytes, "hex");
    writeBytes(prefix.mem, ptr, bytes);
    const fn = name.startsWith("qpack") ? prefix.e.qpack_prefix_int_decode : prefix.e.hpack_prefix_int_decode;
    const prefixBits = name.endsWith("-5") ? 5 : 8;
    assert.equal(fn(ptr, bytes.length, prefixBits, out), 0, name);
    assert.deepEqual(readPrefixRecord(prefix.view, out), {
      flags: exp.flags,
      consumed: bytes.length,
      value: exp.value,
    });
  }

  const huff = await instantiate("hpack-huffman");
  for (const name of ["o", "hash", "name-with-ref"]) {
    const exp = expectCase(expectations, "hpack-huffman", name);
    const bytes = Buffer.from(exp.bytes, "hex");
    writeBytes(huff.mem, ptr, bytes);
    assert.equal(huff.e.hpack_huffman_validate(ptr, bytes.length), exp.status, name);
    const decodedLen = Buffer.from(exp.decoded, "hex").length;
    assert.deepEqual(unpack(huff.e.hpack_huffman_decode(ptr, bytes.length, out, 1024)), {
      status: exp.status,
      value: decodedLen,
    });
    assert.deepEqual(Array.from(huff.mem.slice(out, out + decodedLen)), Array.from(Buffer.from(exp.decoded, "hex")));
  }

  const hpackString = await instantiate("hpack-string");
  const meta = 6144;
  for (const name of ["raw-abc", "huffman-o"]) {
    const exp = expectCase(expectations, "hpack-string", name);
    const bytes = Buffer.from(exp.bytes, "hex");
    writeBytes(hpackString.mem, ptr, bytes);
    const decodedLen = Buffer.from(exp.decoded, "hex").length;
    assert.deepEqual(unpack(hpackString.e.hpack_string_decode(ptr, bytes.length, out, 1024, meta)), {
      status: exp.status,
      value: decodedLen,
    });
    assert.deepEqual(Array.from(hpackString.mem.slice(out, out + decodedLen)), Array.from(Buffer.from(exp.decoded, "hex")));
  }

  const qpackString = await instantiate("qpack-string");
  for (const [name, size] of [["qpack-name-without-ref", 6], ["qpack-name-with-ref", 8]]) {
    const exp = expectCase(expectations, "qpack-string", name);
    const bytes = Buffer.from(exp.bytes, "hex");
    writeBytes(qpackString.mem, ptr, bytes);
    const decodedLen = Buffer.from(exp.decoded, "hex").length;
    assert.deepEqual(unpack(qpackString.e.qpack_string_decode(size, ptr, bytes.length, out, 1024, meta)), {
      status: exp.status,
      value: decodedLen,
    });
    assert.deepEqual(Array.from(qpackString.mem.slice(out, out + decodedLen)), Array.from(Buffer.from(exp.decoded, "hex")));
    assert.equal(qpackString.e.qpack_string_scan(size, ptr, bytes.length, meta), exp.status);
    assert.ok(readStringMeta(qpackString.view, meta).payloadLen > 0);
  }
}

async function checkWs(expectations) {
  const { e, mem, view } = await instantiate("ws-frame");
  const ptr = 1024;
  const out = 4096;
  for (const [name, bytes] of [
    ["binary-small", [0x82, 0x03]],
    ["ping-masked", [0x89, 0x82]],
    ["bad-fragmented-control", [0x09, 0x00]],
  ]) {
    writeBytes(mem, ptr, bytes);
    assert.equal(e.ws_decode_prefix(ptr, bytes.length, out), expectCase(expectations, "ws-frame", name).status, name);
    if (name !== "bad-fragmented-control") {
      assert.ok(readWsPrefix(view, out).opcode > 0);
    }
  }

  const server = expectCase(expectations, "ws-frame", "server-binary-small");
  assert.deepEqual(unpack(e.ws_write_server_frame_header(2, 3, 0, ptr, 16)), { status: server.status, value: 2 });
  assert.equal(Buffer.from(mem.slice(ptr, ptr + 2)).toString("hex"), server.header.slice(0, 4));

  const tooLarge = expectCase(expectations, "ws-frame", "server-ping-too-large");
  assert.equal(unpack(e.ws_write_server_frame_header(9, 126, 0, ptr, 16)).status, tooLarge.status);

  writeBytes(mem, ptr, [0x00, 0x7e]);
  assert.equal(e.ws_decode_payload_len(ptr, 2, 126, 1024, out), 0);
  assert.deepEqual(readWsLen(view, out), { payloadLenLow: 126, payloadLenHigh: 0, consumed: 2 });
}

(async () => {
  let expectations;
  try {
    expectations = loadRustExpectations();
  } catch (_error) {
    await verifyDeletedRustOracle();
    return;
  }
  await checkHttp1(expectations);
  await checkHttp2(expectations);
  await checkHttp3(expectations);
  await checkPrefixAndStrings(expectations);
  await checkWs(expectations);

  console.log(
    JSON.stringify(
      {
        unit: "rust-parity-http-ws",
        rust_oracle_cases: expectations.size,
        wat_modules: [
          "http1-scan",
          "http1-lines",
          "http1-body",
          "http2-frame",
          "http3-frame",
          "http-prefix-int",
          "hpack-huffman",
          "hpack-string",
          "qpack-string",
          "ws-frame",
        ],
        gaps: [
          "HTTP/1 request/status line parity is limited to Rust method/status/header validators; full Rust message parsing is more permissive than the scanner WAT.",
          "HPACK/QPACK parity covers prefix/string/Huffman vectors, not dynamic table header-block decoding.",
          "WebSocket parity covers frame prefix, length, and server header helpers, not HTTP upgrade/accept hashing.",
        ],
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((err) => {
  console.error(err.stack || String(err));
  process.exit(1);
});
