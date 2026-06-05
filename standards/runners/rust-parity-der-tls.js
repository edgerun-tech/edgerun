#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const repoRoot = path.resolve(__dirname, "../..");
const watRoot = path.join(repoRoot, "standards/build/wasm/codec-primitives");
const tmpRoot = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-rust-parity-der-tls-"));

const STATUS = {
  ok: 0,
  short: 1,
  cap: 2,
  invalid: 3,
  overflow: 4,
  incomplete: 5,
};

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
    exports: instance.exports,
    memory: new Uint8Array(instance.exports.memory.buffer),
    view: new DataView(instance.exports.memory.buffer),
  };
}

function write(memory, bytes, ptr = 1024) {
  memory.fill(0, ptr, ptr + Math.max(bytes.length + 64, 128));
  memory.set(bytes, ptr);
  return ptr;
}

function writeAscii(memory, text, ptr = 1024) {
  return write(memory, [...Buffer.from(text, "ascii")], ptr);
}

function u16(n) {
  return [(n >>> 8) & 0xff, n & 0xff];
}

function u24(n) {
  return [(n >>> 16) & 0xff, (n >>> 8) & 0xff, n & 0xff];
}

function u32(memory, ptr) {
  return (
    memory[ptr] |
    (memory[ptr + 1] << 8) |
    (memory[ptr + 2] << 16) |
    (memory[ptr + 3] << 24)
  ) >>> 0;
}

function pack32(value) {
  return {
    status: Number(value & 0xffffffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function derLenParts(value) {
  return {
    status: Number(value & 0xffffn),
    consumed: Number((value >> 16n) & 0xffffn),
    value: Number((value >> 32n) & 0xffffffffn),
  };
}

function derHeaderParts(value) {
  return {
    status: Number(value & 0xffffn),
    consumed: Number((value >> 16n) & 0xffffn),
    tag: Number((value >> 32n) & 0xffn),
    length: Number((value >> 40n) & 0xffffffn),
  };
}

function tlsRecordParts(value) {
  return {
    status: Number(value & 0xffffn),
    contentType: Number((value >> 16n) & 0xffn),
    version: Number((value >> 24n) & 0xffffn),
    fragmentLen: Number((value >> 40n) & 0xffffn),
  };
}

function tlsHandshakeParts(value) {
  return {
    status: Number(value & 0xffffn),
    handshakeType: Number((value >> 16n) & 0xffn),
    bodyLen: Number((value >> 24n) & 0xffffffn),
  };
}

function span(memory, ptr = 2048) {
  return {
    ptr: u32(memory, ptr),
    len: u32(memory, ptr + 4),
    headerLen: u32(memory, ptr + 8),
    totalLen: u32(memory, ptr + 12),
    aux: u32(memory, ptr + 16),
  };
}

function span3(memory, ptr = 2048) {
  return {
    dataOffset: u32(memory, ptr),
    dataLen: u32(memory, ptr + 4),
    nextOffset: u32(memory, ptr + 8),
  };
}

function extRecord(memory, ptr = 2048) {
  return {
    type: u32(memory, ptr),
    dataOffset: u32(memory, ptr + 4),
    dataLen: u32(memory, ptr + 8),
    nextOffset: u32(memory, ptr + 12),
  };
}

function alpnRecord(memory, ptr = 2048) {
  return {
    protoOffset: u32(memory, ptr),
    protoLen: u32(memory, ptr + 4),
    nextOffset: u32(memory, ptr + 8),
    listLen: u32(memory, ptr + 12),
  };
}

function rustSource() {
  return String.raw`
use edgerun_crypto::der::{Decode, Header, Length, Reader, SliceReader, Tag};
use edgerun_crypto::der::asn1::{BitStringRef, Null, ObjectIdentifier, OctetStringRef, SequenceRef};
use edgerun_crypto::pem_rfc7468::{self, LineEnding};
use edgerun_protocols::tls::handshake::ClientHelloBuilder;
use edgerun_protocols::tls::name_match::{normalize_tls_dns_name, tls_dns_name_matches};
use edgerun_protocols::tls::record::TlsRecord;
use edgerun_protocols::tls::server::client_hello::ClientHello;

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn status(ok: bool) -> &'static str {
    if ok { "ok" } else { "err" }
}

fn line(name: &str, value: String) {
    println!("{name}={value}");
}

fn der_len_case(name: &str, bytes: &[u8]) {
    let mut reader = SliceReader::new(bytes).expect("slice reader");
    match Length::decode(&mut reader) {
        Ok(len) => line(name, format!("ok:{}:{}", reader.position(), u32::from(len))),
        Err(_) => line(name, "err".to_string()),
    }
}

fn der_tag_case(name: &str, byte: u8) {
    line(name, match Tag::try_from(byte) {
        Ok(tag) => format!("ok:{:02x}", u8::from(tag)),
        Err(_) => "err".to_string(),
    });
}

fn der_header_case(name: &str, bytes: &[u8]) {
    let mut reader = SliceReader::new(bytes).expect("slice reader");
    match Header::decode(&mut reader) {
        Ok(header) => line(name, format!("ok:{}:{:02x}:{}", reader.position(), u8::from(header.tag), u32::from(header.length))),
        Err(_) => line(name, "err".to_string()),
    }
}

fn der_asn1_case(name: &str, kind: &str, bytes: &[u8]) {
    let ok = match kind {
        "int-i16" => i16::from_der(bytes).map(|_| ()).is_ok(),
        "int-u16" => u16::from_der(bytes).map(|_| ()).is_ok(),
        "bit" => BitStringRef::from_der(bytes).map(|_| ()).is_ok(),
        "octet" => OctetStringRef::from_der(bytes).map(|_| ()).is_ok(),
        "null" => Null::from_der(bytes).map(|_| ()).is_ok(),
        "seq" => SequenceRef::from_der(bytes).map(|_| ()).is_ok(),
        _ => false,
    };
    line(name, status(ok).to_string());
}

fn der_oid_case(name: &str, body: &[u8]) {
    line(name, status(ObjectIdentifier::from_der(&[&[0x06, body.len() as u8], body].concat()).is_ok()).to_string());
}

fn pem_case(name: &str, pem: &str) {
    line(name, match pem_rfc7468::decode_label(pem.as_bytes()) {
        Ok(label) => format!("ok:{label}"),
        Err(_) => "err".to_string(),
    });
}

fn pem_encoded_len_case(name: &str, label: &str, der_len: usize) {
    let input = vec![0u8; der_len];
    match pem_rfc7468::encoded_len(label, LineEnding::LF, &input) {
        Ok(len) => line(name, format!("ok:{len}")),
        Err(_) => line(name, "err".to_string()),
    }
}

fn tls_record_case(name: &str, bytes: &[u8]) {
    match TlsRecord::from_bytes(bytes) {
        Ok((record, consumed)) => line(name, format!("ok:{consumed}:{}:{}:{}", record.content_type, record.version, record.fragment.len())),
        Err(_) => line(name, "err".to_string()),
    }
}

fn tls_name_norm_case(name: &str, input: &str) {
    match normalize_tls_dns_name(input) {
        Some(value) => line(name, format!("ok:{value}")),
        None => line(name, "err".to_string()),
    }
}

fn tls_name_match_case(name: &str, pattern: &str, host: &str) {
    line(name, if tls_dns_name_matches(pattern, host) { "ok:1" } else { "ok:0" }.to_string());
}

fn tls_clienthello_cases() {
    let random = [7u8; 32];
    let msg = ClientHelloBuilder::new(random, "example.com")
        .alpn_protocols(&[b"h3", b"acme-tls/1"])
        .build()
        .expect("client hello");
    let body_len = ((msg[1] as usize) << 16) | ((msg[2] as usize) << 8) | msg[3] as usize;
    let body = &msg[4..4 + body_len];
    let parsed = ClientHello::parse(&msg).expect("parse client hello");
    line("tls_clienthello_build_parse", format!("ok:{}:{}:{}", body.len(), parsed.server_name.unwrap_or_default(), parsed.alpn_protocols.len()));
    line("tls_clienthello_body_hex", hex(body));

    let mut truncated = msg.clone();
    truncated.truncate(10);
    line("tls_clienthello_truncated", status(ClientHello::parse(&truncated).is_ok()).to_string());
}

fn tls_certificate_list_ref(name: &str, bytes: &[u8]) {
    if bytes.is_empty() {
        line(name, "err".to_string());
        return;
    }
    let context_len = bytes[0] as usize;
    if bytes.len() < 1 + context_len + 3 {
        line(name, "err".to_string());
        return;
    }
    let list_len = ((bytes[1 + context_len] as usize) << 16)
        | ((bytes[2 + context_len] as usize) << 8)
        | bytes[3 + context_len] as usize;
    let list_start = 4 + context_len;
    if bytes.len() < list_start + list_len {
        line(name, "err".to_string());
        return;
    }
    line(name, format!("ok:{context_len}:{list_start}:{list_len}:{}", list_start + list_len));
}

fn main() {
    der_len_case("der_len_short_127", &[0x7f]);
    der_len_case("der_len_long_128", &[0x81, 0x80]);
    der_len_case("der_len_nonminimal", &[0x81, 0x7f]);
    der_len_case("der_len_indefinite", &[0x80]);
    der_len_case("der_len_max", &[0x84, 0x0f, 0xff, 0xff, 0xff]);
    der_len_case("der_len_over_max", &[0x84, 0x10, 0x00, 0x00, 0x00]);

    der_tag_case("der_tag_sequence", 0x30);
    der_tag_case("der_tag_high_tag", 0x1f);
    der_tag_case("der_tag_context", 0x80);

    der_header_case("der_header_sequence_int", &[0x30, 0x03, 0x02, 0x01, 0x01]);
    der_header_case("der_header_bad_len", &[0x30, 0x80]);

    der_oid_case("der_oid_rsa_prefix", &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d]);
    der_oid_case("der_oid_invalid_root", &[0x78]);
    der_oid_case("der_oid_truncated_arc", &[0x2a, 0x80]);

    der_asn1_case("der_asn1_i127", "int-i16", &[0x02, 0x01, 0x7f]);
    der_asn1_case("der_asn1_i_nonminimal", "int-i16", &[0x02, 0x02, 0x00, 0x00]);
    der_asn1_case("der_asn1_bit_ok", "bit", &[0x03, 0x03, 0x01, 0xaa, 0x80]);
    der_asn1_case("der_asn1_bit_bad_unused", "bit", &[0x03, 0x01, 0x08]);
    der_asn1_case("der_asn1_octet_ok", "octet", &[0x04, 0x03, 0xde, 0xad, 0xbe]);
    der_asn1_case("der_asn1_null_ok", "null", &[0x05, 0x00]);
    der_asn1_case("der_asn1_null_bad", "null", &[0x05, 0x01, 0x00]);
    der_asn1_case("der_asn1_seq_ok", "seq", &[0x30, 0x05, 0x02, 0x01, 0x01, 0x05, 0x00]);

    pem_case("pem_cert_label", "-----BEGIN CERTIFICATE-----\nSGVsbG8=\n-----END CERTIFICATE-----\n");
    pem_case("pem_lowercase_label", "-----BEGIN certificate-----\nSGVsbG8=\n-----END certificate-----\n");
    pem_case("pem_mismatched_label", "-----BEGIN CERTIFICATE-----\nSGVsbG8=\n-----END PRIVATE KEY-----\n");
    pem_encoded_len_case("pem_encoded_len_13_cert", "CERTIFICATE", 13);

    tls_record_case("tls_record_valid", &[0x17, 0x03, 0x03, 0x00, 0x05, 1, 2, 3, 4, 5]);
    tls_record_case("tls_record_short", &[0x17, 0x03, 0x03, 0x00]);
    tls_record_case("tls_record_unknown_type", &[0x13, 0x03, 0x03, 0x00, 0x00]);
    tls_record_case("tls_record_bad_version", &[0x17, 0x03, 0x05, 0x00, 0x00]);
    tls_record_case("tls_record_over_16k", &[0x17, 0x03, 0x03, 0x40, 0x01]);

    tls_name_norm_case("tls_name_example", "Example.COM.");
    tls_name_norm_case("tls_name_localhost", "localhost");
    tls_name_match_case("tls_name_match_exact", "example.com", "Example.COM.");
    tls_name_match_case("tls_name_match_wild", "*.example.com", "www.example.com");
    tls_name_match_case("tls_name_match_multi", "*.example.com", "a.b.example.com");

    tls_clienthello_cases();
    tls_certificate_list_ref("tls_cert_list_scan", &[0, 0, 0, 10, 0, 0, 5, 0x30, 0x03, 0x02, 0x01, 0x01, 0, 0]);
    tls_certificate_list_ref("tls_cert_list_truncated", &[0, 0, 0, 10, 0, 0, 5, 0x30, 0x03]);
}
`;
}

function runRust() {
  const cargoToml = `
[package]
name = "edgerun-rust-parity-der-tls"
version = "0.0.0"
edition = "2024"

[dependencies]
edgerun-crypto = { path = "${path.join(repoRoot, "crates/utility/edgerun-crypto")}", default-features = false, features = ["std", "alloc", "p256", "pem"] }
edgerun-protocols = { path = "${path.join(repoRoot, "crates/protocol/edgerun-protocols")}", default-features = false, features = ["tls", "tls-cert"] }
`;
  fs.writeFileSync(path.join(tmpRoot, "Cargo.toml"), cargoToml);
  fs.mkdirSync(path.join(tmpRoot, "src"));
  fs.writeFileSync(path.join(tmpRoot, "src/main.rs"), rustSource());
  const output = execFileSync("cargo", ["run", "--quiet", "--manifest-path", path.join(tmpRoot, "Cargo.toml")], {
    cwd: repoRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  });
  const map = new Map();
  for (const line of output.trim().split(/\n+/)) {
    const idx = line.indexOf("=");
    if (idx > 0) map.set(line.slice(0, idx), line.slice(idx + 1));
  }
  return map;
}

function expectEq(results, name, wat, rust, note = "exact") {
  const ok = wat === rust;
  results.push({ name, ok, wat, rust, note });
  if (!ok && note === "exact") {
    throw new Error(`${name} mismatch: WAT=${wat} Rust=${rust}`);
  }
}

async function runWat(results, rust) {
  {
    const { exports, memory } = await instantiate("der-tlv");
    let ptr = write(memory, [0x7f]);
    expectEq(results, "der_len_short_127", `ok:${derLenParts(exports.der_len_decode(ptr, 1)).consumed}:${derLenParts(exports.der_len_decode(ptr, 1)).value}`, rust.get("der_len_short_127"));
    ptr = write(memory, [0x81, 0x80]);
    expectEq(results, "der_len_long_128", `ok:${derLenParts(exports.der_len_decode(ptr, 2)).consumed}:${derLenParts(exports.der_len_decode(ptr, 2)).value}`, rust.get("der_len_long_128"));
    for (const [name, bytes] of [
      ["der_len_nonminimal", [0x81, 0x7f]],
      ["der_len_indefinite", [0x80]],
      ["der_len_over_max", [0x84, 0x10, 0x00, 0x00, 0x00]],
    ]) {
      ptr = write(memory, bytes);
      expectEq(results, name, derLenParts(exports.der_len_decode(ptr, bytes.length)).status === STATUS.ok ? "ok" : "err", rust.get(name));
    }
    ptr = write(memory, [0x84, 0x0f, 0xff, 0xff, 0xff]);
    expectEq(results, "der_len_max", `ok:${derLenParts(exports.der_len_decode(ptr, 5)).consumed}:${derLenParts(exports.der_len_decode(ptr, 5)).value}`, rust.get("der_len_max"));

    ptr = write(memory, [0x30]);
    expectEq(results, "der_tag_sequence", pack32(exports.der_tag_decode(ptr, 1)).status === STATUS.ok ? "ok:30" : "err", rust.get("der_tag_sequence"));
    ptr = write(memory, [0x1f]);
    expectEq(results, "der_tag_high_tag", pack32(exports.der_tag_decode(ptr, 1)).status === STATUS.ok ? "ok:1f" : "err", rust.get("der_tag_high_tag"));
    ptr = write(memory, [0x80]);
    expectEq(results, "der_tag_context", pack32(exports.der_tag_decode(ptr, 1)).status === STATUS.ok ? "ok:80" : "err", rust.get("der_tag_context"));

    ptr = write(memory, [0x30, 0x03, 0x02, 0x01, 0x01]);
    let h = derHeaderParts(exports.der_header_decode(ptr, 5));
    expectEq(results, "der_header_sequence_int", h.status === STATUS.ok ? `ok:${h.consumed}:${h.tag.toString(16).padStart(2, "0")}:${h.length}` : "err", rust.get("der_header_sequence_int"));
    ptr = write(memory, [0x30, 0x80]);
    expectEq(results, "der_header_bad_len", derHeaderParts(exports.der_header_decode(ptr, 2)).status === STATUS.ok ? "ok" : "err", rust.get("der_header_bad_len"));
  }

  {
    const { exports, memory } = await instantiate("der-oid");
    for (const [name, bytes] of [
      ["der_oid_rsa_prefix", [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d]],
      ["der_oid_invalid_root", [0x78]],
      ["der_oid_truncated_arc", [0x2a, 0x80]],
    ]) {
      const ptr = write(memory, bytes);
      expectEq(results, name, exports.der_oid_value_validate(ptr, bytes.length, 2048) === STATUS.ok ? "ok" : "err", rust.get(name));
    }
  }

  {
    const { exports, memory } = await instantiate("der-asn1-basic");
    const checks = [
      ["der_asn1_i127", "der_asn1_integer_decode", [0x02, 0x01, 0x7f]],
      ["der_asn1_i_nonminimal", "der_asn1_integer_decode", [0x02, 0x02, 0x00, 0x00]],
      ["der_asn1_bit_ok", "der_asn1_bit_string_decode", [0x03, 0x03, 0x01, 0xaa, 0x80]],
      ["der_asn1_bit_bad_unused", "der_asn1_bit_string_decode", [0x03, 0x01, 0x08]],
      ["der_asn1_octet_ok", "der_asn1_octet_string_decode", [0x04, 0x03, 0xde, 0xad, 0xbe]],
      ["der_asn1_null_ok", "der_asn1_null_decode", [0x05, 0x00]],
      ["der_asn1_null_bad", "der_asn1_null_decode", [0x05, 0x01, 0x00]],
      ["der_asn1_seq_ok", "der_asn1_sequence_decode", [0x30, 0x05, 0x02, 0x01, 0x01, 0x05, 0x00]],
    ];
    for (const [name, fnName, bytes] of checks) {
      const ptr = write(memory, bytes);
      const status = fnName === "der_asn1_null_decode"
        ? exports[fnName](ptr, bytes.length)
        : exports[fnName](ptr, bytes.length, 2048);
      expectEq(results, name, status === STATUS.ok ? "ok" : "err", rust.get(name));
    }
  }

  {
    const { exports, memory, view } = await instantiate("pem-rfc7468");
    const pemCases = [
      ["pem_cert_label", "-----BEGIN CERTIFICATE-----\nSGVsbG8=\n-----END CERTIFICATE-----\n"],
      ["pem_lowercase_label", "-----BEGIN certificate-----\nSGVsbG8=\n-----END certificate-----\n"],
      ["pem_mismatched_label", "-----BEGIN CERTIFICATE-----\nSGVsbG8=\n-----END PRIVATE KEY-----\n"],
    ];
    for (const [name, text] of pemCases) {
      const ptr = writeAscii(memory, text);
      const status = exports.pem_find_boundaries(ptr, text.length, 2048);
      if (status === STATUS.ok) {
        const labelOff = view.getUint32(2048 + 8, true);
        const labelLen = view.getUint32(2048 + 12, true);
        const label = Buffer.from(memory.slice(ptr + labelOff, ptr + labelOff + labelLen)).toString("ascii");
        expectEq(results, name, `ok:${label}`, rust.get(name));
      } else {
        expectEq(results, name, "err", rust.get(name), name === "pem_lowercase_label" ? "wat-stricter" : "exact");
      }
    }
    expectEq(results, "pem_encoded_len_13_cert", `ok:${pack32(exports.pem_encoded_len(13, "CERTIFICATE".length)).value}`, rust.get("pem_encoded_len_13_cert"));
  }

  {
    const { exports, memory } = await instantiate("tls-frame");
    const recordCases = [
      ["tls_record_valid", [0x17, 0x03, 0x03, 0x00, 0x05, 1, 2, 3, 4, 5], "exact"],
      ["tls_record_short", [0x17, 0x03, 0x03, 0x00], "exact"],
      ["tls_record_unknown_type", [0x13, 0x03, 0x03, 0x00, 0x00], "wat-stricter"],
      ["tls_record_bad_version", [0x17, 0x03, 0x05, 0x00, 0x00], "wat-stricter"],
      ["tls_record_over_16k", [0x17, 0x03, 0x03, 0x40, 0x01], "exact"],
    ];
    for (const [name, bytes, note] of recordCases) {
      const ptr = write(memory, bytes);
      const r = tlsRecordParts(exports.tls_record_header_decode(ptr, bytes.length));
      const wat = r.status === STATUS.ok ? `ok:${5 + r.fragmentLen}:${r.contentType}:${r.version}:${r.fragmentLen}` : "err";
      expectEq(results, name, wat, rust.get(name), note);
    }
    const ptr = write(memory, [0x01, 0x00, 0x00, 0x20]);
    const hs = tlsHandshakeParts(exports.tls_handshake_header_decode(ptr, 4));
    expectEq(results, "tls_handshake_header_basic", hs.status === STATUS.ok ? `ok:${hs.handshakeType}:${hs.bodyLen}` : "err", "ok:1:32");
  }

  {
    const { exports, memory } = await instantiate("tls-vector");
    let ptr = write(memory, [3, 0xaa, 0xbb, 0xcc]);
    let status = exports.tls_vector_u8_decode(ptr, 4, 2048);
    expectEq(results, "tls_vector_u8_basic", status === STATUS.ok ? `ok:${span3(memory).dataOffset}:${span3(memory).dataLen}:${span3(memory).nextOffset}` : "err", "ok:1:3:4");
    ptr = write(memory, [0, 2, 0xaa, 0xbb]);
    status = exports.tls_vector_u16_decode(ptr, 4, 2048);
    expectEq(results, "tls_vector_u16_basic", status === STATUS.ok ? `ok:${span3(memory).dataOffset}:${span3(memory).dataLen}:${span3(memory).nextOffset}` : "err", "ok:2:2:4");
    const extensions = [0, 16, 0, 5, 0, 3, 2, 0x68, 0x33];
    ptr = write(memory, extensions);
    status = exports.tls_extension_next(ptr, extensions.length, 0, 2048);
    const ext = extRecord(memory);
    expectEq(results, "tls_extension_next_basic", status === STATUS.ok ? `ok:${ext.type}:${ext.dataOffset}:${ext.dataLen}:${ext.nextOffset}` : "err", "ok:16:4:5:9");
    status = exports.tls_alpn_next(ptr + ext.dataOffset, ext.dataLen, 0, 2048);
    const alpn = alpnRecord(memory);
    expectEq(results, "tls_alpn_next_basic", status === STATUS.ok ? `ok:${alpn.protoOffset}:${alpn.protoLen}:${alpn.nextOffset}:${alpn.listLen}` : "err", "ok:3:2:5:3");
  }

  {
    const { exports, memory } = await instantiate("tls-name");
    let ptr = writeAscii(memory, "Example.COM.");
    let out = pack32(exports.tls_dns_name_normalize(ptr, "Example.COM.".length, 2048, 64));
    let text = out.status === STATUS.ok ? Buffer.from(memory.slice(2048, 2048 + out.value)).toString("ascii") : "";
    expectEq(results, "tls_name_example", out.status === STATUS.ok ? `ok:${text}` : "err", rust.get("tls_name_example"));
    ptr = writeAscii(memory, "localhost");
    out = pack32(exports.tls_dns_name_normalize(ptr, "localhost".length, 2048, 64));
    expectEq(results, "tls_name_localhost", out.status === STATUS.ok ? "ok" : "err", rust.get("tls_name_localhost"));
    let pattern = writeAscii(memory, "example.com", 1024);
    let host = writeAscii(memory, "Example.COM.", 1536);
    expectEq(results, "tls_name_match_exact", `ok:${exports.tls_dns_name_matches(pattern, "example.com".length, host, "Example.COM.".length)}`, rust.get("tls_name_match_exact"));
    pattern = writeAscii(memory, "*.example.com", 1024);
    host = writeAscii(memory, "www.example.com", 1536);
    expectEq(results, "tls_name_match_wild", `ok:${exports.tls_dns_name_matches(pattern, "*.example.com".length, host, "www.example.com".length)}`, rust.get("tls_name_match_wild"));
    host = writeAscii(memory, "a.b.example.com", 1536);
    expectEq(results, "tls_name_match_multi", `ok:${exports.tls_dns_name_matches(pattern, "*.example.com".length, host, "a.b.example.com".length)}`, rust.get("tls_name_match_multi"));
  }

  {
    const { exports, memory } = await instantiate("tls-clienthello");
    const bodyHex = rust.get("tls_clienthello_body_hex");
    const body = [...Buffer.from(bodyHex, "hex")];
    const ptr = write(memory, body);
    const status = exports.tls_clienthello_scan(ptr, body.length, 2048);
    expectEq(results, "tls_clienthello_build_parse", status === STATUS.ok ? `ok:${body.length}:example.com:2` : "err", rust.get("tls_clienthello_build_parse"));
    expectEq(results, "tls_clienthello_truncated", exports.tls_clienthello_scan(ptr, 10, 2048) === STATUS.ok ? "ok" : "err", rust.get("tls_clienthello_truncated"));
  }

  {
    const { exports, memory } = await instantiate("tls-certificate-list");
    const body = [0, 0, 0, 10, 0, 0, 5, 0x30, 0x03, 0x02, 0x01, 0x01, 0, 0];
    let ptr = write(memory, body);
    let status = exports.tls_certificate_list_scan(ptr, body.length, 2048);
    const wat = status === STATUS.ok ? `ok:${u32(memory, 2052)}:${u32(memory, 2056)}:${u32(memory, 2060)}:${u32(memory, 2064)}` : "err";
    expectEq(results, "tls_cert_list_scan", wat, rust.get("tls_cert_list_scan"));
    const trunc = body.slice(0, 9);
    ptr = write(memory, trunc);
    status = exports.tls_certificate_list_scan(ptr, trunc.length, 2048);
    expectEq(results, "tls_cert_list_truncated", status === STATUS.ok ? "ok" : "err", rust.get("tls_cert_list_truncated"));
  }
}

(async () => {
  try {
    const rust = runRust();
    const results = [];
    await runWat(results, rust);
    const mismatches = results.filter((r) => !r.ok);
    const strict = mismatches.filter((r) => r.note === "wat-stricter");
    const unexpected = mismatches.filter((r) => r.note !== "wat-stricter");
    if (unexpected.length > 0) {
      throw new Error(`unexpected mismatches: ${JSON.stringify(unexpected, null, 2)}`);
    }
    console.log(JSON.stringify({
      unit: "rust-parity-der-tls",
      ok: true,
      exact_matches: results.filter((r) => r.ok).length,
      wat_stricter_mismatches: strict,
      rust_cases: rust.size,
    }, null, 2));
  } finally {
    fs.rmSync(tmpRoot, { recursive: true, force: true });
  }
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
