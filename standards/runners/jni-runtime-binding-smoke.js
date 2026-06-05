#!/usr/bin/env node
const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/jni-runtime-binding.wat");
const wasm = path.join(os.tmpdir(), `jni-runtime-binding-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "inherit" });

(async () => {
  const bytes = fs.readFileSync(wasm);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  let cursor = 64;
  function put(s) {
    const p = cursor;
    const data = Buffer.from(s, "utf8");
    mem.set(data, p);
    cursor += data.length + 8;
    return [p, data.length];
  }
  function ns(s) {
    const [p, n] = put(s);
    return e.jni_namespace_status(p, n);
  }
  function camel(s) {
    const [p, n] = put(s);
    return e.jni_snake_camel_action(p, n);
  }

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300135);

  assert.strictEqual(ns("com.example.Foo"), 0);
  assert.strictEqual(ns("org.signal.client.internal.Native"), 0);
  assert.strictEqual(ns("com..example.Foo"), 4);
  assert.strictEqual(ns("com.example.1Foo"), 5);
  assert.strictEqual(ns("com/example/Foo"), 2);
  assert.strictEqual(ns(".com.example.Foo"), 3);

  assert.strictEqual(e.jni_identifier_mangle_kind("_".charCodeAt(0)), 2);
  assert.strictEqual(e.jni_identifier_mangle_kind(".".charCodeAt(0)), 3);
  assert.strictEqual(e.jni_identifier_mangle_kind("$".charCodeAt(0)), 4);
  assert.strictEqual(e.jni_signature_arg_mangle_kind(";".charCodeAt(0)), 5);
  assert.strictEqual(e.jni_signature_arg_mangle_kind("[".charCodeAt(0)), 6);
  assert.strictEqual(e.jni_signature_arg_mangle_kind("/".charCodeAt(0)), 3);

  assert.strictEqual(camel("say_hello"), 4);
  assert.strictEqual(camel("_private_method"), 3);
  assert.strictEqual(camel("XMLParser"), 1);
  assert.strictEqual(camel("plain"), 0);
  assert.strictEqual(camel("___"), 2);

  assert.strictEqual(e.jni_native_method_status(1, 1, 1, 0, 0, 0, 1, 1), 0);
  assert.strictEqual(e.jni_native_method_status(0, 1, 1, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.jni_native_method_status(1, 0, 1, 0, 0, 0, 0, 0), 2);
  assert.strictEqual(e.jni_native_method_status(1, 1, 0, 0, 0, 0, 0, 0), 3);
  assert.strictEqual(e.jni_native_method_status(1, 1, 1, 0, 0, 0, 1, 0), 4);
  assert.strictEqual(e.jni_native_method_status(1, 1, 1, 1, 1, 0, 0, 0), 5);
  assert.strictEqual(e.jni_native_method_status(1, 1, 1, 1, 0, 1, 0, 0), 6);

  assert.strictEqual(e.jni_native_wrapper_shape(1, 1, 0, 1), 33);
  assert.strictEqual(e.jni_native_wrapper_shape(0, 1, 1, 1), 50);
  assert.strictEqual(e.jni_native_wrapper_shape(0, 0, 2, 0), 19);

  assert.strictEqual(e.jni_call_guard_result(1, 0, 0, 0), 1);
  assert.strictEqual(e.jni_call_guard_result(0, 1, 0, 0), 2);
  assert.strictEqual(e.jni_call_guard_result(0, 1, 0, 1), 4);
  assert.strictEqual(e.jni_call_guard_result(0, 0, 1, 0), 3);
  assert.strictEqual(e.jni_call_guard_result(0, 0, 0, 0), 0);

  assert.strictEqual(e.jni_reference_category(1), 1);
  assert.strictEqual(e.jni_reference_category(3), 3);
  assert.strictEqual(e.jni_loader_plan(0), 9);
  assert.strictEqual(e.jni_loader_plan(1), 2);
  assert.strictEqual(e.jni_loader_plan(2), 13);

  assert.strictEqual(e.jni_jvalue_kind_from_descriptor("L".charCodeAt(0)), 1);
  assert.strictEqual(e.jni_jvalue_kind_from_descriptor("[".charCodeAt(0)), 2);
  assert.strictEqual(e.jni_jvalue_kind_from_descriptor("I".charCodeAt(0)), 3);
  assert.strictEqual(e.jni_jvalue_kind_from_descriptor("V".charCodeAt(0)), 4);

  assert.strictEqual(e.jni_error_code_category(0), 0);
  assert.strictEqual(e.jni_error_code_category(-2), 2);
  assert.strictEqual(e.jni_error_code_category(-4), 4);
  assert.strictEqual(e.jni_error_code_category(-99), 7);

  assert.strictEqual(e.jni_release_mode_category(0), 1);
  assert.strictEqual(e.jni_release_mode_category(2), 2);
  assert.strictEqual(e.jni_version_major(0x00010008), 1);
  assert.strictEqual(e.jni_version_minor(0x00010008), 8);
  assert.strictEqual(e.jni_version_major(0x00150000), 21);
  assert.strictEqual(e.jni_version_minor(0x00150000), 0);

  fs.unlinkSync(wasm);
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  console.error(err);
  process.exit(1);
});
