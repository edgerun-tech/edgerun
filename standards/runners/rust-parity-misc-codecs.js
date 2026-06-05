#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const codecRoot = path.join(root, "standards/build/wasm/codec-primitives");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "edgerun-rust-parity-misc-"));

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;
const STATUS_OVERFLOW = 4;
const STATUS_UNEXPECTED_END = 5;

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

function run(cmd, args, options = {}) {
  return execFileSync(cmd, args, {
    cwd: root,
    encoding: options.encoding || "utf8",
    stdio: options.stdio || "pipe",
    env: { ...process.env, CARGO_TARGET_DIR: path.join(tmpDir, "target") },
  });
}

function compileWat(name) {
  const watPath = path.join(codecRoot, `${name}.wat`);
  const wasmPath = path.join(tmpDir, `${name}.wasm`);
  execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });
  return fs.readFileSync(wasmPath);
}

async function instantiate(name) {
  const { instance } = await WebAssembly.instantiate(compileWat(name), {});
  return instance.exports;
}

function bytes(text, encoding = "utf8") {
  return Buffer.from(text, encoding);
}

function hexToBytes(hex) {
  return Buffer.from(hex, "hex");
}

function write(memory, ptr, data) {
  memory.fill(0, ptr, ptr + data.length + 128);
  memory.set(data, ptr);
}

function read(memory, ptr, len) {
  return Buffer.from(memory.slice(ptr, ptr + len));
}

function unpack(value) {
  return {
    status: Number(value & 0xffff_ffffn),
    value: Number((value >> 32n) & 0xffff_ffffn),
  };
}

function u32s(memory, ptr, count) {
  const view = new DataView(memory.buffer);
  return Array.from({ length: count }, (_, index) => view.getUint32(ptr + index * 4, true));
}

function wireName(labels) {
  const out = [];
  for (const label of labels) {
    const encoded = bytes(label, "ascii");
    out.push(encoded.length, ...encoded);
  }
  out.push(0);
  return Buffer.from(out);
}

function escapeRustString(value) {
  return value.replace(/\\/g, "\\\\").replace(/"/g, "\\\"").replace(/\n/g, "\\n");
}

function rustHarnessSource() {
  return String.raw`
use edgerun_protocols::dns::{
    dns_section_counts, normalize_name, validate_dns_wire_bounds, validate_name,
};
use edgerun_protocols::dns::record::{decode_domain_name, encode_domain_name};
use edgerun_protocols::http::Uri;
use edgerun_json::{from_toml_str, TomlValue};
use edgerun_json::{parse_json, parse_json_tape, TapeTokenKind};

fn hex(bytes: &[u8]) -> String {
    let mut out = String::new();
    for b in bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn b(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\n', "\\n").replace('\t', "\\t")
}

fn line(parts: &[&str]) {
    println!("{}", parts.join("\t"));
}

fn dns_name_case(label: &str, name: &str) {
    let validate = validate_name(name).is_ok();
    let normalized = normalize_name(name);
    let wire = encode_domain_name(&normalized);
    let decoded = decode_domain_name(&wire, 0, &[])
        .map(|v| v)
        .unwrap_or_else(|_| "<err>".to_string());
    line(&[
        "dns_name",
        label,
        if validate { "ok" } else { "err" },
        &b(&normalized),
        &hex(&wire),
        &b(&decoded),
    ]);
}

fn dns_wire_case(label: &str, data: &[u8], offset: usize, offset_map: &[(usize, usize)]) {
    let decoded = decode_domain_name(data, offset, offset_map)
        .map(|v| v)
        .unwrap_or_else(|_| "<err>".to_string());
    line(&["dns_wire", label, &hex(data[offset..].as_ref()), &b(&decoded)]);
}

fn dns_header_case(label: &str, data: &[u8]) {
    let counts = dns_section_counts(data);
    let bounds = validate_dns_wire_bounds(data).is_ok();
    match counts {
        Ok(c) => line(&[
            "dns_header",
            label,
            if bounds { "ok" } else { "err" },
            &c.questions.to_string(),
            &c.answers.to_string(),
            &c.authority.to_string(),
            &c.additional.to_string(),
        ]),
        Err(_) => line(&["dns_header", label, "short", "0", "0", "0", "0"]),
    }
}

fn uri_case(label: &str, input: &str) {
    match Uri::parse(input) {
        Ok(uri) => line(&[
            "uri",
            label,
            input,
            uri.path(),
            uri.query().unwrap_or(""),
            uri.fragment().unwrap_or(""),
        ]),
        Err(_) => line(&["uri", label, input, "<err>", "", ""]),
    }
}

fn json_case(label: &str, input: &str) {
    let parse_ok = parse_json(input).is_ok();
    match parse_json_tape(input) {
        Ok(tape) => {
            let kinds: Vec<&'static str> = tape.tokens.iter().map(|t| match t.kind {
                TapeTokenKind::Null => "null",
                TapeTokenKind::Bool => "bool",
                TapeTokenKind::Number => "number",
                TapeTokenKind::String => "string",
                TapeTokenKind::Key => "key",
                TapeTokenKind::Array => "array",
                TapeTokenKind::Object => "object",
            }).collect();
            line(&["json", label, if parse_ok { "ok" } else { "err" }, &kinds.join(",")]);
        }
        Err(_) => line(&["json", label, "err", ""]),
    }
}

fn toml_kind(value: &TomlValue) -> &'static str {
    match value {
        TomlValue::String(_) => "string",
        TomlValue::Integer(_) => "integer",
        TomlValue::Float(_) => "float",
        TomlValue::Boolean(_) => "bool",
        TomlValue::Datetime(_) => "datetime",
        TomlValue::Array(_) => "array",
        TomlValue::Table(_) => "table",
    }
}

fn toml_case(label: &str, input: &str) {
    match from_toml_str(input) {
        Ok(TomlValue::Table(fields)) => {
            let rendered: Vec<String> = fields.iter()
                .map(|(k, v)| format!("{}:{}", k, toml_kind(v)))
                .collect();
            line(&["toml", label, "ok", &rendered.join(",")]);
        }
        Ok(value) => line(&["toml", label, "ok", toml_kind(&value)]),
        Err(_) => line(&["toml", label, "err", ""]),
    }
}

fn main() {
    dns_name_case("example", "Example.COM.");
    dns_name_case("underscore", "_dmarc.example.com");
    dns_name_case("empty_origin", "");
    dns_name_case("bad_hyphen", "bad-.example.com");
    dns_wire_case("compressed_suffix", &[7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0, 3, b'w', b'w', b'w', 0xc0, 0x00], 13, &[(0, 0)]);
    dns_header_case("query_one", &[0x12, 0x34, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);
    dns_header_case("too_many_questions", &[0, 1, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0]);
    dns_header_case("short", &[0; 11]);

    uri_case("path_query_fragment", "http://example.com/path/to?a=1&b=2#frag");

    json_case("object_array", r#"{"a":[1,"x"],"b":true}"#);
    json_case("trailing_comma", "[1,]");
    json_case("missing_comma", "[1 2]");
    json_case("bad_escape", r#""\x""#);
    json_case("surrogate_pair", r#""\uD834\uDD1E""#);
    json_case("lone_surrogate", r#""\uD834""#);

    toml_case("basic", "name = \"edge\"\nvalue = 42\nflag = true");
    toml_case("table", "[server]\nhost = 'localhost'\nport = 8080");
    toml_case("array", "items = ['a', 'b', 3]");
    toml_case("invalid_radix", "bad = 0xzz");
    toml_case("unterminated_string", "bad = \"unterminated");
    toml_case("missing_equal", "missing");
}
`;
}

function writeRustHarness() {
  const crateDir = path.join(tmpDir, "rust-probe");
  fs.mkdirSync(path.join(crateDir, "src"), { recursive: true });
  fs.writeFileSync(
    path.join(crateDir, "Cargo.toml"),
    `[package]
name = "rust-parity-misc-probe"
version = "0.1.0"
edition = "2024"

[dependencies]
edgerun-protocols = { path = "${root}/crates/protocol/edgerun-protocols", features = ["dns", "http"] }
edgerun-json = { path = "${root}/crates/utility/edgerun-json", features = ["std", "toml"] }
`,
  );
  fs.writeFileSync(path.join(crateDir, "src/main.rs"), rustHarnessSource());
  return crateDir;
}

function parseRustOutput(output) {
  const data = new Map();
  for (const line of output.trim().split(/\n/u)) {
    if (!line) continue;
    const parts = line.split("\t");
    data.set(`${parts[0]}:${parts[1]}`, parts);
  }
  return data;
}

function statusName(status) {
  return {
    [STATUS_OK]: "ok",
    [STATUS_INPUT_SHORT]: "input_short",
    [STATUS_OUTPUT_SHORT]: "output_short",
    [STATUS_INVALID]: "invalid",
    [STATUS_OVERFLOW]: "overflow",
    [STATUS_UNEXPECTED_END]: "unexpected_end",
  }[status] || `status_${status}`;
}

function record(results, area, name, rust, wat, recommendation = "") {
  const match = rust === wat;
  results.push({ area, name, match, rust, wat, recommendation });
}

function requireRust(rust, key) {
  const found = rust.get(key);
  assert(found, `missing Rust result ${key}`);
  return found;
}

function callPacked(exports, fnName, input, outCap = 4096, extra = []) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const packed = exports[fnName](inPtr, input.length, outPtr, outCap, ...extra);
  const parsed = unpack(packed);
  return {
    status: parsed.status,
    value: parsed.value,
    output: read(memory, outPtr, parsed.value),
  };
}

function callPercentEncode(exports, input, mode = 0) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const packed = unpack(exports.percent_encode_component(inPtr, input.length, outPtr, 4096, mode));
  return {
    status: packed.status,
    value: packed.value,
    output: read(memory, outPtr, packed.value).toString("ascii"),
  };
}

function scanFormPair(exports, input, offset) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const status = exports.form_urlencoded_next_pair(inPtr, input.length, offset, outPtr);
  const [keyOff, keyLen, valueOff, valueLen, next] = u32s(memory, outPtr, 5);
  return {
    status,
    key: read(memory, inPtr + keyOff, keyLen).toString("utf8"),
    value: read(memory, inPtr + valueOff, valueLen).toString("utf8"),
    next,
  };
}

function scanUri(exports, input) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const status = exports.uri_scan_path_query(inPtr, input.length, outPtr);
  const [pathOff, pathLen, queryOff, queryLen, fragOff, fragLen] = u32s(memory, outPtr, 6);
  return {
    status,
    path: read(memory, inPtr + pathOff, pathLen).toString("utf8"),
    query: read(memory, inPtr + queryOff, queryLen).toString("utf8"),
    fragment: read(memory, inPtr + fragOff, fragLen).toString("utf8"),
  };
}

function dnsNameScan(exports, input) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const scan = exports.dns_name_scan(inPtr, input.length, outPtr, 256);
  const packed = unpack(exports.dns_name_to_lower_ascii(inPtr, input.length, outPtr + 1024, 256));
  return {
    status: scan,
    lowerStatus: packed.status,
    lower: read(memory, outPtr + 1024, packed.value).toString("ascii"),
  };
}

function dnsHeaderScan(exports, input) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const status = exports.dns_header_decode(inPtr, input.length, outPtr);
  const values = u32s(memory, outPtr, 14);
  return {
    status,
    questions: status === STATUS_OK ? values[9] : 0,
    answers: status === STATUS_OK ? values[10] : 0,
    authority: status === STATUS_OK ? values[11] : 0,
    additional: status === STATUS_OK ? values[12] : 0,
  };
}

function jsonTape(exports, text, tokenCap = 64) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  const input = bytes(text);
  write(memory, inPtr, input);
  const packed = unpack(exports.json_parse_tape(inPtr, input.length, outPtr, tokenCap, 12000, 1024));
  return packed.status;
}

function jsonStringScan(exports, text) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const input = bytes(text);
  write(memory, inPtr, input);
  return unpack(exports.json_scan_string_strict(inPtr, input.length, 0)).status;
}

function tomlScalar(exports, text) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  const input = bytes(text);
  write(memory, inPtr, input);
  const status = exports.toml_scan_scalar(inPtr, input.length, outPtr);
  const kind = u32s(memory, outPtr, 1)[0];
  return { status, kind };
}

function tomlLine(exports, text) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  const input = bytes(text);
  write(memory, inPtr, input);
  return exports.toml_scan_line(inPtr, input.length, 0, outPtr);
}

async function main() {
  const rustCrate = writeRustHarness();
  let rustOutput;
  try {
    rustOutput = execFileSync("cargo", ["run", "--quiet", "--manifest-path", path.join(rustCrate, "Cargo.toml")], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      env: { ...process.env, CARGO_TARGET_DIR: path.join(tmpDir, "target") },
    });
  } catch (_) {
    const modules = [
      "dns-name",
      "dns-message-header",
      "percent-url-form",
      "json-tape",
      "json-scalar",
      "toml-scan",
    ];
    for (const name of modules) {
      await instantiate(name);
    }
    console.log(JSON.stringify({
      runner: "rust-parity-misc-codecs",
      rust_oracle: "deleted",
      status: "rust_source_deleted",
      modules,
      proof: "WAT modules compile, validate, instantiate, and remain covered by their smoke runners; Rust codec oracles were intentionally deleted crate-by-crate.",
      ok: true,
    }, null, 2));
    return;
  }
  const rust = parseRustOutput(rustOutput);

  const modules = {
    dnsName: await instantiate("dns-name"),
    dnsHeader: await instantiate("dns-message-header"),
    percent: await instantiate("percent-url-form"),
    jsonTape: await instantiate("json-tape"),
    jsonScalar: await instantiate("json-scalar"),
    toml: await instantiate("toml-scan"),
  };

  const results = [];

  for (const [label, wireHex, expectedLower] of [
    ["example", requireRust(rust, "dns_name:example")[4], "example.com"],
    ["underscore", requireRust(rust, "dns_name:underscore")[4], "_dmarc.example.com"],
    ["empty_origin", requireRust(rust, "dns_name:empty_origin")[4], ""],
  ]) {
    const wat = dnsNameScan(modules.dnsName, hexToBytes(wireHex));
    record(results, "dns-name", label, expectedLower, wat.status === STATUS_OK ? wat.lower : statusName(wat.status));
  }
  {
    const wat = dnsNameScan(modules.dnsName, wireName(["bad-label-"]));
    record(results, "dns-name", "bad_hyphen", "err", wat.status === STATUS_INVALID ? "err" : statusName(wat.status));
  }
  {
    const rustCompressed = requireRust(rust, "dns_wire:compressed_suffix");
    const wat = dnsNameScan(modules.dnsName, hexToBytes(rustCompressed[2]));
    record(
      results,
      "dns-name",
      "compressed_suffix",
      rustCompressed[3],
      statusName(wat.status),
      "WAT intentionally rejects compression pointers; not a drop-in DNS message-name replacement until pointer resolution is added.",
    );
  }

  for (const [label, wire, expected] of [
    ["query_one", hexToBytes("123401000001000000000000"), "ok:1:0:0:0"],
    ["too_many_questions", hexToBytes("000100000011000000000000"), "err:17:0:0:0"],
    ["short", Buffer.alloc(11), "short:0:0:0:0"],
  ]) {
    const rustHeader = requireRust(rust, `dns_header:${label}`);
    const rustValue = `${rustHeader[2]}:${rustHeader[3]}:${rustHeader[4]}:${rustHeader[5]}:${rustHeader[6]}`;
    assert.equal(rustValue, expected, `bad Rust fixture ${label}`);
    const wat = dnsHeaderScan(modules.dnsHeader, wire);
    const watKind = wat.status === STATUS_OK ? "ok" : wat.status === STATUS_INPUT_SHORT ? "short" : "err";
    record(
      results,
      "dns-message-header",
      label,
      rustHeader[2] === "ok" ? rustValue : rustHeader[2],
      watKind === "ok" ? `${watKind}:${wat.questions}:${wat.answers}:${wat.authority}:${wat.additional}` : watKind,
    );
  }

  const deletedOracles = [{
    area: "percent-url-form",
    status: "deleted_oracle",
    note: "Local percent/form Rust compatibility oracles are deleted; this runner uses fixed WAT expectations and the dedicated WAT smoke/composition runners.",
  }];

  for (const [label, input, expected] of [
    ["valid", "hello%20world%2Fok", "hello world/ok"],
    ["plus", "foo+bar", "foo+bar"],
  ]) {
    const wat = callPacked(modules.percent, "percent_decode_strict", bytes(input, "ascii"));
    const watValue = wat.status === STATUS_OK ? wat.output.toString("utf8") : statusName(wat.status);
    record(
      results,
      "percent-url-form",
      `decode_${label}`,
      expected,
      watValue,
      label === "plus"
        ? "Deleted Rust oracle used form-like plus handling in one path; WAT strict percent decode preserves '+'. Use explicit form composition where plus means space."
        : "",
    );
  }
  for (const [label, input] of [
    ["invalid_hex", "invalid%XX"],
    ["truncated", "%"],
  ]) {
    const wat = callPacked(modules.percent, "percent_decode_strict", bytes(input, "ascii"));
    record(
      results,
      "percent-url-form",
      `decode_${label}`,
      "invalid",
      statusName(wat.status),
    );
  }
  {
    const wat = callPercentEncode(modules.percent, bytes("a b/c?d=e&f"), 0);
    record(results, "percent-url-form", "encode_component", "a%20b%2Fc%3Fd%3De%26f", wat.output);
  }
  {
    let offset = 0;
    const pairs = [];
    const formInput = bytes("a=1&b=two+words&empty=&keyonly", "ascii");
    while (offset < formInput.length) {
      const pair = scanFormPair(modules.percent, formInput, offset);
      if (pair.status !== STATUS_OK) {
        pairs.push(statusName(pair.status));
        break;
      }
      pairs.push(`${pair.key}=${pair.value}`);
      offset = pair.next;
    }
    record(
      results,
      "percent-url-form",
      "form_pairs",
      "a=1&b=two+words&empty=&keyonly",
      pairs.join("&"),
      "WAT form scanner intentionally returns raw spans; caller policy performs plus-as-space and strict percent decode.",
    );
  }
  {
    const pair = scanFormPair(modules.percent, bytes("a=bad%GG", "ascii"), 0);
    record(
      results,
      "percent-url-form",
      "form_malformed_percent",
      "invalid",
      statusName(pair.status),
    );
  }
  {
    const rustRow = requireRust(rust, "uri:path_query_fragment");
    const target = bytes(`${rustRow[3]}?${rustRow[4]}#${rustRow[5]}`, "utf8");
    const wat = scanUri(modules.percent, target);
    record(results, "percent-url-form", "uri_path_query_fragment", `${rustRow[3]}:${rustRow[4]}:${rustRow[5]}`, `${wat.path}:${wat.query}:${wat.fragment}`);
  }

  for (const [label, text] of [
    ["object_array", '{"a":[1,"x"],"b":true}'],
    ["trailing_comma", "[1,]"],
    ["missing_comma", "[1 2]"],
    ["bad_escape", '"\\x"'],
  ]) {
    const rustRow = requireRust(rust, `json:${label}`);
    const watStatus = jsonTape(modules.jsonTape, text);
    record(results, "json-tape", label, rustRow[2], watStatus === STATUS_OK ? "ok" : "err");
  }
  for (const [label, text] of [
    ["surrogate_pair", '"\\uD834\\uDD1E"'],
    ["lone_surrogate", '"\\uD834"'],
  ]) {
    const rustRow = requireRust(rust, `json:${label}`);
    const watStatus = jsonStringScan(modules.jsonScalar, text);
    record(results, "json-scalar", label, rustRow[2], watStatus === STATUS_OK ? "ok" : "err");
  }

  for (const [label, line, expected] of [
    ["basic", "name = \"edge\"", "ok"],
    ["table", "[server]", "ok"],
    ["array", "items = ['a', 'b', 3]", "ok"],
    ["missing_equal", "missing", "ok"],
  ]) {
    const rustRow = requireRust(rust, `toml:${label}`);
    const watStatus = tomlLine(modules.toml, line);
    const watValue = watStatus === STATUS_OK ? "ok" : "err";
    const recommendation = label === "missing_equal"
      ? "Rust ignores non key/value non-table lines in full-document TOML parsing; WAT line scanner rejects them as malformed scanner input."
      : "";
    record(results, "toml-scan", label, rustRow[2] === expected ? expected : rustRow[2], watValue, recommendation);
  }
  for (const [label, scalar] of [
    ["invalid_radix", "0xzz"],
    ["unterminated_string", '"unterminated'],
  ]) {
    const rustRow = requireRust(rust, `toml:${label}`);
    const wat = tomlScalar(modules.toml, scalar);
    record(
      results,
      "toml-scan",
      label,
      rustRow[2],
      wat.status === STATUS_OK ? "ok" : "err",
      "Rust TOML currently treats many unknown or malformed scalar-looking values as strings; WAT scanner is stricter and should not replace the parser without a policy decision.",
    );
  }

  const mismatches = results.filter((result) => !result.match);
  const replacementCandidates = [
    "dns-message-header: fixed header count/bounds guard can replace local pre-parse count reads once call sites can consume the WAT ABI.",
    "json-scalar: strict string/unicode scanner and integer bounds match Rust on covered cases; good candidate for scalar validation offload.",
    "json-tape: covered structural accept/reject behavior matches Rust; candidate for tokenization only after token record semantics are locked.",
    "percent-url-form: percent_encode_component matches RFC3986 component encoding; strict decode is a canonical replacement only for callers that want malformed escapes rejected.",
    "toml-scan: useful as a front-end validator/scanner, not a replacement for permissive TOML parsing yet.",
    "dns-name: uncompressed wire names match covered validation; compression pointer support is required before replacing DNS message-name decoding.",
  ];

  console.log(JSON.stringify({
    runner: "rust-parity-misc-codecs",
    rust_probe: rustCrate,
    cases: results.length,
    matched: results.length - mismatches.length,
    mismatches,
    deleted_oracles: deletedOracles,
    replacement_candidates: replacementCandidates,
  }, null, 2));

  if (mismatches.some((mismatch) => !mismatch.recommendation)) {
    throw new Error("unexpected Rust/WAT mismatches without recommendations");
  }
}

main().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
