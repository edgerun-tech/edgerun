#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const root = path.resolve(__dirname, "..", "..");
const watPath =
  process.argv[2] || path.join(root, "standards/build/wasm/codec-primitives/http-node-state.wat");
const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "http-node-state-"));
const wasmPath = path.join(tmpDir, "http-node-state.wasm");

process.on("exit", () => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

execFileSync("wat2wasm", [watPath, "-o", wasmPath], { stdio: "inherit" });
execFileSync("wasm-validate", [wasmPath], { stdio: "inherit" });

function write(mem, offset, text) {
  const bytes = Buffer.from(text, "ascii");
  mem.fill(0, offset, offset + Math.max(64, bytes.length + 8));
  mem.set(bytes, offset);
  return bytes.length;
}

(async () => {
  const wasm = fs.readFileSync(wasmPath);
  const { instance } = await WebAssembly.instantiate(wasm);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const ptr = 1024;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300107);

  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "GET")), 1);
  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "head")), 2);
  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "POST")), 3);
  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "PATCH")), 9);
  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "PROPFIND")), 255);
  assert.equal(e.http_method_classify(ptr, write(mem, ptr, "BAD METHOD")), 0);
  assert.equal(e.http_method_body_flags(1), 2);
  assert.equal(e.http_method_body_flags(2), 0);
  assert.equal(e.http_method_body_flags(3), 3);
  assert.equal(e.http_method_body_flags(255), 3);

  assert.equal(e.http_status_classify(99), 0);
  assert.equal(e.http_status_classify(100), 1);
  assert.equal(e.http_status_classify(204), 2);
  assert.equal(e.http_status_classify(302), 3);
  assert.equal(e.http_status_classify(404), 4);
  assert.equal(e.http_status_classify(503), 5);
  assert.equal(e.http_status_classify(600), 0);

  assert.equal(e.http_header_name_valid(ptr, write(mem, ptr, "Content-Length")), 1);
  assert.equal(e.http_header_name_valid(ptr, write(mem, ptr, "Bad/Name")), 0);
  assert.equal(e.http_header_value_valid(ptr, write(mem, ptr, "text/plain\tcharset=us-ascii")), 1);
  mem[ptr] = 0x1f;
  assert.equal(e.http_header_value_valid(ptr, 1), 0);

  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "/api/data?x=1")), 1);
  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "https://example.test/")), 2);
  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "*")), 3);
  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "relative")), 4);
  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "/bad path")), 0);
  assert.equal(e.http_request_target_classify(ptr, write(mem, ptr, "/frag#nope")), 0);

  assert.equal(e.http1_body_state(1, 200, 0, 0, 0), 4);
  assert.equal(e.http1_body_state(2, 200, 0, 1, 10), 0);
  assert.equal(e.http1_body_state(1, 204, 0, 1, 10), 0);
  assert.equal(e.http1_body_state(1, 200, 0, 1, 0), 1);
  assert.equal(e.http1_body_state(1, 200, 0, 1, 12), 2);
  assert.equal(e.http1_body_state(1, 200, 1, 1, 12), 3);

  assert.equal(e.http2_frame_type_classify(0), 0);
  assert.equal(e.http2_frame_type_classify(9), 9);
  assert.equal(e.http2_frame_type_classify(10), 255);
  assert.equal(e.http2_error_classify(0), 0);
  assert.equal(e.http2_error_classify(13), 13);
  assert.equal(e.http2_error_classify(99), 2);
  assert.equal(e.http2_frame_semantics(0, 0, 1, 0, 0), 0);
  assert.equal(e.http2_frame_semantics(0, 0, 0, 0, 0), 1);
  assert.equal(e.http2_frame_semantics(4, 1, 0, 1, 0), 6);
  assert.equal(e.http2_frame_semantics(4, 0, 3, 0, 0), 1);
  assert.equal(e.http2_frame_semantics(6, 0, 0, 7, 0), 6);
  assert.equal(e.http2_frame_semantics(8, 0, 1, 4, 0), 1);
  assert.equal(e.http2_frame_semantics(5, 0, 1, 4, 2), 1);
  assert.equal(e.http2_frame_semantics(5, 0, 1, 4, 3), 0);

  assert.equal(e.http3_frame_type_classify(0n), 0);
  assert.equal(e.http3_frame_type_classify(1n), 1);
  assert.equal(e.http3_frame_type_classify(4n), 2);
  assert.equal(e.http3_frame_type_classify(3n), 3);
  assert.equal(e.http3_frame_type_classify(5n), 4);
  assert.equal(e.http3_frame_type_classify(7n), 5);
  assert.equal(e.http3_frame_type_classify(8n), 6);
  assert.equal(e.http3_frame_type_classify(9n), 7);
  assert.equal(e.http3_frame_type_classify(0x21n), 8);
  assert.equal(e.http3_frame_type_classify(0x41n), 9);
  assert.equal(e.http3_frame_stream_semantics(0, 2), 0);
  assert.equal(e.http3_frame_stream_semantics(0, 0), 10);
  assert.equal(e.http3_frame_stream_semantics(1, 0), 0);
  assert.equal(e.http3_frame_stream_semantics(1, 6), 10);
  assert.equal(e.http3_route_state(0, 0, 0n, 0n), 0);
  assert.equal(e.http3_route_state(1, 0, 12n, 0n), 1);
  assert.equal(e.http3_route_state(1, 1, 4n, 4n), 2);
  assert.equal(e.http3_route_state(1, 1, 8n, 4n), 3);
  assert.equal(e.http3_status_classify(200), 2);
  assert.equal(e.http3_status_classify(700), 0);

  console.log(
    JSON.stringify(
      {
        unit: "http-node-state",
        abi_version: e.proto_abi_version(),
        standard_id: e.proto_standard_id(),
        ok: true,
        cases: [
          "method-status-header-body-classification",
          "http1-body-state",
          "http2-frame-type-error-semantics",
          "http3-frame-route-status-semantics",
          "request-target-validity",
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
