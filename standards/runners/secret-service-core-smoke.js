#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/secret-service-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, ptr, text) {
  const bytes = Buffer.from(text, "ascii");
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function callText(exports, memory, name, text, ptr = 1024) {
  return exports[name](...put(memory, ptr, text));
}

function action(exports, memory, iface, member) {
  const i = put(memory, 1024, iface);
  const m = put(memory, 2048, member);
  return exports.secret_dbus_action_code(i[0], i[1], m[0], m[1]);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300109);

  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.Secret.Service"), 1);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.Secret.Collection"), 2);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.Secret.Item"), 3);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.Secret.Session"), 4);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.DBus.Introspectable"), 5);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.DBus.Properties"), 6);
  assert.strictEqual(callText(exports, memory, "secret_interface_code", "org.freedesktop.Secret.Unknown"), 0);

  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "OpenSession"), 1);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "CreateCollection"), 2);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "SearchItems"), 3);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "Unlock"), 4);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "Lock"), 5);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "GetSecrets"), 6);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "ReadAlias"), 7);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "SetAlias"), 8);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Collection", "ListItems"), 9);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Collection", "CreateItem"), 10);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Collection", "Delete"), 11);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Item", "GetSecret"), 12);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Item", "Delete"), 13);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Session", "Close"), 14);
  assert.strictEqual(action(exports, memory, "org.freedesktop.DBus.Introspectable", "Introspect"), 15);
  assert.strictEqual(action(exports, memory, "org.freedesktop.DBus.Properties", "GetAll"), 16);
  assert.strictEqual(action(exports, memory, "org.freedesktop.Secret.Service", "Delete"), 0);

  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/"), 5);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets"), 1);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets/collections/default"), 2);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets/collections/default/my_key"), 3);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets/session/s42"), 4);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets/session/42"), 0);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/org/freedesktop/secrets/collections/default/"), 0);
  assert.strictEqual(callText(exports, memory, "secret_path_classify", "/some/other/path"), 0);

  assert.strictEqual(callText(exports, memory, "secret_component_id_valid", "default"), 1);
  assert.strictEqual(callText(exports, memory, "secret_component_id_valid", "my_key_01"), 1);
  assert.strictEqual(callText(exports, memory, "secret_component_id_valid", "bad/key"), 0);
  assert.strictEqual(callText(exports, memory, "secret_component_id_valid", ""), 0);
  assert.strictEqual(callText(exports, memory, "secret_session_id_valid", "s1"), 1);
  assert.strictEqual(callText(exports, memory, "secret_session_id_valid", "s001"), 1);
  assert.strictEqual(callText(exports, memory, "secret_session_id_valid", "x1"), 0);
  assert.strictEqual(callText(exports, memory, "secret_session_id_valid", "s"), 0);

  assert.strictEqual(exports.secret_action_valid_for_path(1, 1), 1);
  assert.strictEqual(exports.secret_action_valid_for_path(1, 8), 1);
  assert.strictEqual(exports.secret_action_valid_for_path(1, 9), 0);
  assert.strictEqual(exports.secret_action_valid_for_path(2, 10), 1);
  assert.strictEqual(exports.secret_action_valid_for_path(3, 12), 1);
  assert.strictEqual(exports.secret_action_valid_for_path(4, 14), 1);
  assert.strictEqual(exports.secret_action_valid_for_path(4, 13), 0);

  for (let code = 1; code <= 9; code += 1) assert.strictEqual(exports.secret_request_code_valid(code), 1);
  assert.strictEqual(exports.secret_request_code_valid(0), 0);
  assert.strictEqual(exports.secret_request_code_valid(10), 0);
  for (let code = 1; code <= 7; code += 1) assert.strictEqual(exports.secret_response_code_valid(code), 1);
  assert.strictEqual(exports.secret_response_code_valid(8), 0);

  assert.strictEqual(exports.secret_backend_item_status(1, 1, 1, 1), 0);
  assert.strictEqual(exports.secret_backend_item_status(0, 1, 1, 1), 1);
  assert.strictEqual(exports.secret_backend_item_status(1, 0, 1, 1), 2);
  assert.strictEqual(exports.secret_backend_item_status(1, 1, 0, 1), 3);
  assert.strictEqual(exports.secret_backend_item_status(1, 1, 1, 0), 4);

  assert.strictEqual(exports.secret_session_access_status(1, 0, 1, 1, 2000n, 1000n, 1000n), 0);
  assert.strictEqual(exports.secret_session_access_status(0, 0, 1, 1, 2000n, 1000n, 1000n), 1);
  assert.strictEqual(exports.secret_session_access_status(1, 1, 1, 1, 2000n, 1000n, 1000n), 2);
  assert.strictEqual(exports.secret_session_access_status(1, 0, 0, 1, 2000n, 1000n, 1000n), 3);
  assert.strictEqual(exports.secret_session_access_status(1, 0, 1, 0, 2000n, 1000n, 1000n), 3);
  assert.strictEqual(exports.secret_session_access_status(1, 0, 1, 1, 2001n, 1000n, 1000n), 4);

  assert.strictEqual(exports.secret_unlock_status(1, 1), 0);
  assert.strictEqual(exports.secret_unlock_status(0, 1), 3);
  assert.strictEqual(exports.secret_unlock_status(1, 0), 3);
  assert.strictEqual(exports.secret_lock_status(1, 0), 0);
  assert.strictEqual(exports.secret_lock_status(0, 0), 1);
  assert.strictEqual(exports.secret_lock_status(1, 1), 2);

  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.DBus.Error.InvalidArgs"), 1);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.DBus.Error.UnknownMethod"), 2);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.Secret.Error.NoSuchSession"), 3);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.Secret.Error.IsLocked"), 4);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.Secret.Error.Failed"), 5);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.Secret.Error.NoSuchItem"), 6);
  assert.strictEqual(callText(exports, memory, "secret_error_code", "org.freedesktop.Secret.Error.Other"), 0);

  console.log(
    JSON.stringify({
      unit: "secret-service-core",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "dbus_interface_codes",
        "dbus_member_action_codes",
        "dbus_path_classification",
        "collection_item_session_id_validation",
        "action_path_compatibility",
        "secret_request_response_code_ranges",
        "backend_item_status_mapping",
        "session_lock_unlock_status_mapping",
        "dbus_error_name_mapping",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
