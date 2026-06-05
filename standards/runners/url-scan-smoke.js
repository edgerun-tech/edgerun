#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/url-scan.wat";

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + bytes.length + 128);
  memory.set(bytes, ptr);
  return bytes.length;
}

function span(input, off, len) {
  return input.slice(off, off + len);
}

function record(view, outPtr) {
  return {
    schemeOff: view.getUint32(outPtr, true),
    schemeLen: view.getUint32(outPtr + 4, true),
    authorityOff: view.getUint32(outPtr + 8, true),
    authorityLen: view.getUint32(outPtr + 12, true),
    hostOff: view.getUint32(outPtr + 16, true),
    hostLen: view.getUint32(outPtr + 20, true),
    pathOff: view.getUint32(outPtr + 24, true),
    pathLen: view.getUint32(outPtr + 28, true),
    queryOff: view.getUint32(outPtr + 32, true),
    queryLen: view.getUint32(outPtr + 36, true),
    fragmentOff: view.getUint32(outPtr + 40, true),
    fragmentLen: view.getUint32(outPtr + 44, true),
    port: view.getUint32(outPtr + 48, true),
    hasPort: view.getUint32(outPtr + 52, true),
  };
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const exports = module.instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300104);

  const inPtr = 1024;
  const outPtr = 4096;

  function scan(input) {
    const len = write(memory, inPtr, input);
    const status = exports.url_scan(inPtr, len, outPtr);
    return { status, rec: record(view, outPtr), input };
  }

  function expectOk(input, expected) {
    const { status, rec } = scan(input);
    assert.strictEqual(status, 0, input);
    assert.strictEqual(span(input, rec.schemeOff, rec.schemeLen), expected.scheme, input);
    assert.strictEqual(span(input, rec.authorityOff, rec.authorityLen), expected.authority, input);
    assert.strictEqual(span(input, rec.hostOff, rec.hostLen), expected.host, input);
    assert.strictEqual(span(input, rec.pathOff, rec.pathLen), expected.path || "", input);
    assert.strictEqual(span(input, rec.queryOff, rec.queryLen), expected.query || "", input);
    assert.strictEqual(span(input, rec.fragmentOff, rec.fragmentLen), expected.fragment || "", input);
    assert.strictEqual(rec.port, expected.port || 0, input);
    assert.strictEqual(rec.hasPort, expected.hasPort ? 1 : 0, input);
  }

  function expectInvalid(input) {
    const { status } = scan(input);
    assert.strictEqual(status, 3, input);
  }

  expectOk("https://example.com", {
    scheme: "https",
    authority: "example.com",
    host: "example.com",
  });

  expectOk("https://example.com:8443", {
    scheme: "https",
    authority: "example.com:8443",
    host: "example.com",
    port: 8443,
    hasPort: true,
  });

  expectOk("https://chatgpt.com/backend-api/codex/responses", {
    scheme: "https",
    authority: "chatgpt.com",
    host: "chatgpt.com",
    path: "/backend-api/codex/responses",
  });

  expectOk("HtTp+v1.2://example.com/a/b?x=1&empty=#frag", {
    scheme: "HtTp+v1.2",
    authority: "example.com",
    host: "example.com",
    path: "/a/b",
    query: "x=1&empty=",
    fragment: "frag",
  });

  expectOk("  https://example.com/path?q=1#f  ", {
    scheme: "https",
    authority: "example.com",
    host: "example.com",
    path: "/path",
    query: "q=1",
    fragment: "f",
  });

  assert.strictEqual(exports.url_scheme_validate(inPtr, write(memory, inPtr, "web+edge.1")), 0);
  assert.strictEqual(exports.url_scheme_validate(inPtr, write(memory, inPtr, "")), 3);
  assert.strictEqual(exports.url_scheme_validate(inPtr, write(memory, inPtr, "bad_scheme")), 3);

  expectInvalid("example.com/path");
  expectInvalid("https:example.com/path");
  expectInvalid("https://");
  expectInvalid("https:///path");
  expectInvalid("https://:8443/path");
  expectInvalid("https://example.com:");
  expectInvalid("https://example.com:abc");
  expectInvalid("https://example.com:65536");
  expectInvalid("https://user@example.com/path");
  expectInvalid("https://[::1]/path");

  console.log(
    JSON.stringify(
      {
        unit: "url-scan",
        abi_version: exports.proto_abi_version(),
        standard_id: exports.proto_standard_id(),
        ok: true,
      },
      null,
      2,
    ),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
