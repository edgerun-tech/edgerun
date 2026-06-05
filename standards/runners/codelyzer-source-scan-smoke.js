#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/codelyzer-source-scan.wat";

const OK = 0;
const NO_FUNCTION = 2;
const INVALID = 3;
const LANG_RUST = 1;
const LANG_JS = 2;
const LANG_PY = 4;
const LANG_GO = 5;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, ptr, text) {
  const bytes = Buffer.from(text);
  memory.fill(0, ptr, ptr + bytes.length + 64);
  memory.set(bytes, ptr);
  return bytes.length;
}

function packedStatus(value) {
  return Number(value & 0xffffffffn);
}

function packedCount(value) {
  return Number(value >> 32n);
}

(async () => {
  const module = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = module.instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const pathPtr = 1024;
  const srcPtr = 2048;
  const outPtr = 8192;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300089);

  let pathLen = write(memory, pathPtr, "src/lib.rs");
  let src = [
    "// fn fake_comment() {}",
    "const S: &str = \"fn fake_string() {}\";",
    "pub fn real() { helper(); }",
    "fn helper() {}",
  ].join("\n");
  let srcLen = write(memory, srcPtr, src);
  assert.equal(e.codelyzer_lang_for_path(pathPtr, pathLen), LANG_RUST);
  let packed = e.codelyzer_count_functions(pathPtr, pathLen, srcPtr, srcLen);
  assert.equal(packedStatus(packed), OK);
  assert.equal(packedCount(packed), 2);
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), OK);
  assert.equal(view.getUint32(outPtr + 20, true), LANG_RUST);
  assert.equal(src.slice(view.getUint32(outPtr, true), view.getUint32(outPtr, true) + view.getUint32(outPtr + 4, true)), "real");
  assert.equal(view.getUint32(outPtr + 8, true), src.indexOf("fn real"));

  pathLen = write(memory, pathPtr, "tools/job.py");
  src = "# def fake(): pass\nname = \"def hidden(): pass\"\ndef build():\n    return 1\n";
  srcLen = write(memory, srcPtr, src);
  assert.equal(e.codelyzer_lang_for_path(pathPtr, pathLen), LANG_PY);
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), OK);
  assert.equal(src.slice(view.getUint32(outPtr, true), view.getUint32(outPtr, true) + view.getUint32(outPtr + 4, true)), "build");

  pathLen = write(memory, pathPtr, "web/app.ts");
  src = "/* function nope() {} */\nconst s = `function hidden() {}`;\nexport function send() { return socket.write(); }\n";
  srcLen = write(memory, srcPtr, src);
  assert.equal(e.codelyzer_lang_for_path(pathPtr, pathLen), LANG_JS);
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), OK);
  assert.equal(src.slice(view.getUint32(outPtr, true), view.getUint32(outPtr, true) + view.getUint32(outPtr + 4, true)), "send");

  pathLen = write(memory, pathPtr, "cmd/main.go");
  src = "// func fake() {}\nfunc Serve() { run() }\n";
  srcLen = write(memory, srcPtr, src);
  assert.equal(e.codelyzer_lang_for_path(pathPtr, pathLen), LANG_GO);
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), OK);
  assert.equal(src.slice(view.getUint32(outPtr, true), view.getUint32(outPtr, true) + view.getUint32(outPtr + 4, true)), "Serve");

  pathLen = write(memory, pathPtr, "README.md");
  srcLen = write(memory, srcPtr, "fn ignored() {}\n");
  assert.equal(e.codelyzer_lang_for_path(pathPtr, pathLen), 0);
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), INVALID);
  packed = e.codelyzer_count_functions(pathPtr, pathLen, srcPtr, srcLen);
  assert.equal(packedStatus(packed), INVALID);

  pathLen = write(memory, pathPtr, "empty.rs");
  srcLen = write(memory, srcPtr, "// fn fake() {}\n");
  assert.equal(e.codelyzer_scan_first_function(pathPtr, pathLen, srcPtr, srcLen, outPtr), NO_FUNCTION);
  packed = e.codelyzer_count_functions(pathPtr, pathLen, srcPtr, srcLen);
  assert.equal(packedStatus(packed), OK);
  assert.equal(packedCount(packed), 0);

  console.log(
    JSON.stringify({
      unit: "codelyzer-source-scan",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "rust_comment_string_ignored",
        "python_def",
        "js_function",
        "go_func",
        "unknown_ext",
        "no_function",
      ],
    })
  );
})();
