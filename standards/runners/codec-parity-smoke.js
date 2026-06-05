#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const codecRoot = path.join(root, "standards/build/wasm/codec-primitives");

const STATUS_OK = 0;
const STATUS_INPUT_SHORT = 1;
const STATUS_OUTPUT_SHORT = 2;
const STATUS_INVALID = 3;

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "codec-parity-"));
process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

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

function unpackLow32High32(value) {
  return {
    status: Number(value & 0xffff_ffffn),
    value: Number((value >> 32n) & 0xffff_ffffn),
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

function write(memory, ptr, bytes) {
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
}

function callBytes(exports, name, input, outCap = 4096) {
  const memory = new Uint8Array(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 8192;
  write(memory, inPtr, input);
  const packed = unpackLow32High32(exports[name](inPtr, input.length, outPtr, outCap));
  return {
    status: packed.status,
    written: packed.value,
    output: Buffer.from(memory.slice(outPtr, outPtr + packed.value)),
  };
}

function refCrc32(bytes) {
  let crc = 0xffff_ffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb8_8320 : 0);
    }
  }
  return (crc ^ 0xffff_ffff) >>> 0;
}

function refAdler32(bytes) {
  let a = 1;
  let b = 0;
  for (const byte of bytes) {
    a = (a + byte) % 65521;
    b = (b + a) % 65521;
  }
  return (((b << 16) | a) >>> 0);
}

function refHexEncodeLower(bytes) {
  return Buffer.from(bytes).toString("hex");
}

function refHexDecodeStrict(text) {
  if (text.length % 2 !== 0) {
    return null;
  }
  if (!/^[0-9a-fA-F]*$/.test(text)) {
    return null;
  }
  return Buffer.from(text, "hex");
}

const B64URL_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

function refBase64urlNoPadEncode(bytes) {
  return Buffer.from(bytes).toString("base64").replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/u, "");
}

function refBase64urlNoPadDecodeCanonical(text) {
  if (text.length % 4 === 1 || /[^A-Za-z0-9_-]/u.test(text)) {
    return null;
  }
  if (text.length % 4 === 2) {
    const b = B64URL_ALPHABET.indexOf(text[text.length - 1]);
    if (b < 0 || (b & 0x0f) !== 0) {
      return null;
    }
  }
  if (text.length % 4 === 3) {
    const c = B64URL_ALPHABET.indexOf(text[text.length - 1]);
    if (c < 0 || (c & 0x03) !== 0) {
      return null;
    }
  }
  const standard = text.replace(/-/g, "+").replace(/_/g, "/");
  return Buffer.from(standard.padEnd(Math.ceil(standard.length / 4) * 4, "="), "base64");
}

function refHttpHeaderNameValid(text) {
  if (text.length === 0) {
    return false;
  }
  return /^[A-Za-z0-9!#$%&'*+\-.^_`|~]+$/u.test(text);
}

function refHttpHeaderValueValid(bytes) {
  return bytes.every((byte) => byte === 0x09 || byte === 0x20 || (byte >= 0x21 && byte <= 0x7e));
}

function refWsPrefix(header) {
  if (header.length < 2) {
    return { status: STATUS_INPUT_SHORT };
  }
  const fin = (header[0] & 0x80) !== 0;
  const opcode = header[0] & 0x0f;
  const masked = (header[1] & 0x80) !== 0;
  const payloadLenCode = header[1] & 0x7f;
  const allowedOpcode = [0, 1, 2, 8, 9, 10].includes(opcode);
  const control = opcode >= 8;
  if (!allowedOpcode || (control && !fin) || (control && payloadLenCode > 125)) {
    return { status: STATUS_INVALID };
  }
  return {
    status: STATUS_OK,
    fin: fin ? 1 : 0,
    opcode,
    masked: masked ? 1 : 0,
    payloadLenCode,
    extendedLenBytes: payloadLenCode === 126 ? 2 : payloadLenCode === 127 ? 8 : 0,
  };
}

function refWsPayloadLen(code, bytes, maxLen) {
  let value;
  let extra;
  if (code < 126) {
    value = code;
    extra = 0;
  } else if (code === 126) {
    if (bytes.length < 2) {
      return { status: STATUS_INPUT_SHORT };
    }
    value = (bytes[0] << 8) | bytes[1];
    extra = 2;
    if (value < 126) {
      return { status: STATUS_INVALID };
    }
  } else if (code === 127) {
    if (bytes.length < 8) {
      return { status: STATUS_INPUT_SHORT };
    }
    if ((bytes[0] & 0x80) !== 0) {
      return { status: STATUS_INVALID };
    }
    value = bytes.readBigUInt64BE(0);
    extra = 8;
    if (value <= 0xffffn || value > BigInt(maxLen)) {
      return { status: value <= 0xffffn ? STATUS_INVALID : 4 };
    }
    return {
      status: STATUS_OK,
      low: Number(value & 0xffff_ffffn),
      high: Number((value >> 32n) & 0xffff_ffffn),
      extra,
    };
  } else {
    return { status: STATUS_INVALID };
  }
  if (value > maxLen) {
    return { status: 4 };
  }
  return { status: STATUS_OK, low: value, high: 0, extra };
}

function refWsServerHeader(opcode, payloadLen) {
  if (![1, 2, 8, 9, 10].includes(opcode)) {
    return { status: STATUS_INVALID };
  }
  if ([8, 9, 10].includes(opcode) && payloadLen > 125n) {
    return { status: STATUS_INVALID };
  }
  const bytes = [0x80 | opcode];
  if (payloadLen < 126n) {
    bytes.push(Number(payloadLen));
  } else if (payloadLen <= 0xffffn) {
    bytes.push(126, Number((payloadLen >> 8n) & 0xffn), Number(payloadLen & 0xffn));
  } else {
    bytes.push(127);
    for (let shift = 56n; shift >= 0n; shift -= 8n) {
      bytes.push(Number((payloadLen >> shift) & 0xffn));
    }
  }
  return { status: STATUS_OK, bytes: Buffer.from(bytes) };
}

function refTlsRecordHeader(bytes) {
  if (bytes.length < 5) {
    return { status: STATUS_INPUT_SHORT };
  }
  const contentType = bytes[0];
  const version = (bytes[1] << 8) | bytes[2];
  const fragmentLen = (bytes[3] << 8) | bytes[4];
  if (![20, 21, 22, 23].includes(contentType) || version < 0x0300 || version > 0x0304 || fragmentLen > 16384) {
    return { status: STATUS_INVALID };
  }
  return { status: STATUS_OK, contentType, version, fragmentLen };
}

function refTlsRecordHeaderEncode(contentType, version, fragmentLen) {
  const decoded = refTlsRecordHeader(Buffer.from([
    contentType,
    (version >> 8) & 0xff,
    version & 0xff,
    (fragmentLen >> 8) & 0xff,
    fragmentLen & 0xff,
  ]));
  if (decoded.status !== STATUS_OK) {
    return { status: decoded.status };
  }
  return {
    status: STATUS_OK,
    bytes: Buffer.from([
      contentType,
      (version >> 8) & 0xff,
      version & 0xff,
      (fragmentLen >> 8) & 0xff,
      fragmentLen & 0xff,
    ]),
  };
}

async function checkEncodingCore() {
  const exports = await instantiate("encoding-core");
  const memory = new Uint8Array(exports.memory.buffer);
  assert.equal(exports.proto_abi_version(), 2);

  for (const sample of ["", "hello", "123456789", "Wikipedia"]) {
    const bytes = Buffer.from(sample, "ascii");
    write(memory, 1024, bytes);
    assert.equal(exports.crc32(1024, bytes.length) >>> 0, refCrc32(bytes), `crc32 ${sample}`);
    assert.equal(exports.adler32(1024, bytes.length) >>> 0, refAdler32(bytes), `adler32 ${sample}`);
  }

  return 8;
}

async function checkEncodingText() {
  const exports = await instantiate("encoding-text");
  assert.equal(exports.proto_abi_version(), 2);

  for (const bytes of [Buffer.alloc(0), Buffer.from([0, 1, 0xab, 0xff]), Buffer.from("EdgeRun")]) {
    const encoded = callBytes(exports, "hex_encode_lower", bytes);
    assert.equal(encoded.status, STATUS_OK, "hex encode status");
    assert.equal(encoded.output.toString("ascii"), refHexEncodeLower(bytes), "hex encode output");

    const decoded = callBytes(exports, "hex_decode_strict", encoded.output);
    assert.equal(decoded.status, STATUS_OK, "hex decode status");
    assert.deepEqual(decoded.output, refHexDecodeStrict(encoded.output.toString("ascii")), "hex decode output");
  }

  for (const text of ["0", "zz"]) {
    const decoded = callBytes(exports, "hex_decode_strict", Buffer.from(text, "ascii"));
    assert.equal(decoded.status, STATUS_INVALID, `hex strict invalid ${text}`);
  }

  for (const bytes of [Buffer.alloc(0), Buffer.from("f"), Buffer.from("fo"), Buffer.from("foo"), Buffer.from([0xfb, 0xff])]) {
    const encoded = callBytes(exports, "base64url_nopad_encode", bytes);
    assert.equal(encoded.status, STATUS_OK, "base64url encode status");
    assert.equal(encoded.output.toString("ascii"), refBase64urlNoPadEncode(bytes), "base64url encode output");

    const decoded = callBytes(exports, "base64url_nopad_decode", encoded.output);
    assert.equal(decoded.status, STATUS_OK, "base64url decode status");
    assert.deepEqual(decoded.output, refBase64urlNoPadDecodeCanonical(encoded.output.toString("ascii")), "base64url decode output");
  }

  for (const text of ["A", "AB", "AAB", "Zm=v"]) {
    assert.equal(refBase64urlNoPadDecodeCanonical(text), null, `reference rejects ${text}`);
    assert.equal(callBytes(exports, "base64url_nopad_decode", Buffer.from(text, "ascii")).status, STATUS_INVALID, `WAT rejects ${text}`);
  }

  return 18;
}

async function checkHttp1Scan() {
  const exports = await instantiate("http1-scan");
  const memory = new Uint8Array(exports.memory.buffer);
  assert.equal(exports.proto_abi_version(), 2);

  const names = ["Host", "x-edgerun_1", "!#$%&'*+-.^_`|~"];
  const badNames = ["", "bad/name", "bad=name", "bad?name", "bad name"];
  for (const name of names) {
    const bytes = Buffer.from(name, "ascii");
    write(memory, 1024, bytes);
    assert.equal(exports.http_validate_header_name(1024, bytes.length), refHttpHeaderNameValid(name) ? STATUS_OK : STATUS_INVALID, `header name ${name}`);
  }
  for (const name of badNames) {
    const bytes = Buffer.from(name, "ascii");
    write(memory, 1024, bytes);
    assert.equal(exports.http_validate_header_name(1024, bytes.length), refHttpHeaderNameValid(name) ? STATUS_OK : STATUS_INVALID, `bad header name ${name}`);
  }

  for (const bytes of [Buffer.from("hello world\tok", "ascii"), Buffer.from([0x21, 0x7e])]) {
    write(memory, 2048, bytes);
    assert.equal(exports.http_validate_header_value(2048, bytes.length), refHttpHeaderValueValid([...bytes]) ? STATUS_OK : STATUS_INVALID, "header value valid");
  }
  for (const bytes of [Buffer.from([0x00]), Buffer.from([0x1f]), Buffer.from([0x80])]) {
    write(memory, 2048, bytes);
    assert.equal(exports.http_validate_header_value(2048, bytes.length), refHttpHeaderValueValid([...bytes]) ? STATUS_OK : STATUS_INVALID, "header value invalid");
  }

  return 14;
}

async function checkWsFrame() {
  const exports = await instantiate("ws-frame");
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);
  const inPtr = 1024;
  const outPtr = 4096;
  assert.equal(exports.proto_abi_version(), 2);

  for (const header of [Buffer.from([0x82, 0x03]), Buffer.from([0x89, 0x82])]) {
    write(memory, inPtr, header);
    const expected = refWsPrefix(header);
    assert.equal(exports.ws_decode_prefix(inPtr, header.length, outPtr), expected.status, "ws prefix status");
    assert.equal(view.getUint32(outPtr, true), expected.fin, "ws prefix fin");
    assert.equal(view.getUint32(outPtr + 4, true), expected.opcode, "ws prefix opcode");
    assert.equal(view.getUint32(outPtr + 8, true), expected.masked, "ws prefix masked");
    assert.equal(view.getUint32(outPtr + 12, true), expected.payloadLenCode, "ws prefix code");
    assert.equal(view.getUint32(outPtr + 16, true), expected.extendedLenBytes, "ws prefix extra bytes");
  }

  for (const header of [Buffer.from([0x83, 0x00]), Buffer.from([0x09, 0x00]), Buffer.from([0x89, 0x7e])]) {
    write(memory, inPtr, header);
    assert.equal(exports.ws_decode_prefix(inPtr, header.length, outPtr), refWsPrefix(header).status, "ws invalid prefix");
  }

  for (const vector of [
    { code: 3, bytes: Buffer.alloc(0), max: 1024 },
    { code: 126, bytes: Buffer.from([0x01, 0x00]), max: 1024 },
    { code: 127, bytes: Buffer.from([0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]), max: 0xffff_ffff },
  ]) {
    write(memory, inPtr, vector.bytes);
    const expected = refWsPayloadLen(vector.code, vector.bytes, vector.max);
    assert.equal(exports.ws_decode_payload_len(inPtr, vector.bytes.length, vector.code, vector.max, outPtr), expected.status, `ws len ${vector.code}`);
    if (expected.status === STATUS_OK) {
      assert.equal(view.getUint32(outPtr, true), expected.low, "ws len low");
      assert.equal(view.getUint32(outPtr + 4, true), expected.high, "ws len high");
      assert.equal(view.getUint32(outPtr + 8, true), expected.extra, "ws len extra");
    }
  }

  for (const vector of [
    { opcode: 2, len: 3n },
    { opcode: 2, len: 256n },
    { opcode: 9, len: 2n },
  ]) {
    const expected = refWsServerHeader(vector.opcode, vector.len);
    const packed = unpackLow32High32(exports.ws_write_server_frame_header(vector.opcode, Number(vector.len & 0xffff_ffffn), Number(vector.len >> 32n), outPtr, 16));
    assert.equal(packed.status, expected.status, "ws write status");
    assert.equal(packed.value, expected.bytes.length, "ws write length");
    assert.deepEqual(Buffer.from(memory.slice(outPtr, outPtr + packed.value)), expected.bytes, "ws write header bytes");
  }

  return 12;
}

async function checkTlsFrame() {
  const exports = await instantiate("tls-frame");
  const memory = new Uint8Array(exports.memory.buffer);
  assert.equal(exports.proto_abi_version(), 2);

  for (const bytes of [Buffer.from([0x16, 0x03, 0x03, 0x00, 0x20]), Buffer.from([0x17, 0x03, 0x04, 0x40, 0x00])]) {
    write(memory, 1024, bytes);
    const expected = refTlsRecordHeader(bytes);
    assert.deepEqual(tlsRecordParts(exports.tls_record_header_decode(1024, bytes.length)), expected, "tls record decode");

    const encoded = refTlsRecordHeaderEncode(expected.contentType, expected.version, expected.fragmentLen);
    const packed = unpackLow32High32(exports.tls_record_header_encode(expected.contentType, expected.version, expected.fragmentLen, 2048, 5));
    assert.equal(packed.status, encoded.status, "tls encode status");
    assert.equal(packed.value, encoded.bytes.length, "tls encode len");
    assert.deepEqual(Buffer.from(memory.slice(2048, 2048 + packed.value)), encoded.bytes, "tls encode bytes");
  }

  for (const bytes of [Buffer.from([0x13, 0x03, 0x03, 0x00, 0x05]), Buffer.from([0x17, 0x03, 0x05, 0x00, 0x05]), Buffer.from([0x17, 0x03, 0x03, 0x40, 0x01])]) {
    write(memory, 1024, bytes);
    assert.equal(tlsRecordParts(exports.tls_record_header_decode(1024, bytes.length)).status, refTlsRecordHeader(bytes).status, "tls invalid decode");
  }

  return 8;
}

(async () => {
  const results = [
    ["encoding-core", await checkEncodingCore()],
    ["encoding-text", await checkEncodingText()],
    ["http1-scan", await checkHttp1Scan()],
    ["ws-frame", await checkWsFrame()],
    ["tls-frame", await checkTlsFrame()],
  ];
  const cases = results.reduce((sum, [, count]) => sum + count, 0);
  console.log(JSON.stringify({
    unit: "codec-parity-smoke",
    ok: true,
    kind: "parity foundation",
    note: "Compares selected WAT codec primitives against small JS references derived from inspected Rust behavior; this does not execute Rust and is not complete parity coverage for all modules.",
    modules: results.map(([name]) => name),
    cases,
  }, null, 2));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
