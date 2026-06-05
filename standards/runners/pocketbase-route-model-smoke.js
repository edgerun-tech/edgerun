#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/pocketbase-route-model.wat");

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

function route(exports, memory, view, method, target) {
  const m = put(memory, 1024, method);
  const p = put(memory, 2048, target);
  const out = 4096;
  const code = exports.pocketbase_classify_route(m[0], m[1], p[0], p[1], out);
  return {
    code,
    method: view.getUint32(out + 4, true),
    allowed: view.getUint32(out + 8, true),
    auth: view.getUint32(out + 12, true),
    status: view.getUint32(out + 16, true),
    activity: view.getUint32(out + 20, true),
  };
}

function strCall(exports, memory, name, text) {
  return exports[name](...put(memory, 1024, text));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const exports = instance.exports;
  const memory = new Uint8Array(exports.memory.buffer);
  const view = new DataView(exports.memory.buffer);

  assert.strictEqual(exports.proto_abi_version(), 2);
  assert.strictEqual(exports.proto_standard_id(), 300096);

  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "GET"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "POST"), 2);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "PATCH"), 3);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "PUT"), 4);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "DELETE"), 5);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "OPTIONS"), 6);
  assert.strictEqual(strCall(exports, memory, "pocketbase_method_code", "HEAD"), 0);

  assert.deepStrictEqual(route(exports, memory, view, "GET", "/api/health"), {
    code: 3,
    method: 1,
    allowed: 1,
    auth: 0,
    status: 200,
    activity: 0,
  });
  assert.deepStrictEqual(route(exports, memory, view, "POST", "/api/collections"), {
    code: 10,
    method: 2,
    allowed: 1,
    auth: 2,
    status: 200,
    activity: 1,
  });
  assert.deepStrictEqual(route(exports, memory, view, "GET", "/api/collections/users/records?page=1"), {
    code: 15,
    method: 1,
    allowed: 1,
    auth: 4,
    status: 200,
    activity: 1,
  });
  assert.deepStrictEqual(route(exports, memory, view, "DELETE", "/api/collections/users/records/abc"), {
    code: 16,
    method: 5,
    allowed: 1,
    auth: 4,
    status: 200,
    activity: 1,
  });
  assert.strictEqual(route(exports, memory, view, "GET", "/api/collections/users/auth-refresh").status, 405);
  assert.deepStrictEqual(route(exports, memory, view, "POST", "/api/collections/users/auth-refresh"), {
    code: 21,
    method: 2,
    allowed: 1,
    auth: 3,
    status: 200,
    activity: 1,
  });
  assert.deepStrictEqual(route(exports, memory, view, "GET", "/api/files/docs/rec1/report.pdf"), {
    code: 17,
    method: 1,
    allowed: 1,
    auth: 4,
    status: 200,
    activity: 1,
  });
  assert.deepStrictEqual(route(exports, memory, view, "POST", "/api/admins/auth-with-password"), {
    code: 31,
    method: 2,
    allowed: 1,
    auth: 0,
    status: 200,
    activity: 1,
  });
  assert.deepStrictEqual(route(exports, memory, view, "GET", "/_/assets/tabler-alpha.bin"), {
    code: 2,
    method: 1,
    allowed: 1,
    auth: 0,
    status: 200,
    activity: 0,
  });
  assert.deepStrictEqual(route(exports, memory, view, "GET", "/.well-known/acme-challenge/tok"), {
    code: 41,
    method: 1,
    allowed: 1,
    auth: 0,
    status: 200,
    activity: 1,
  });
  assert.strictEqual(route(exports, memory, view, "GET", "/missing").status, 404);

  assert.strictEqual(strCall(exports, memory, "pocketbase_valid_name", "users_2026"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_valid_name", "bad-name"), 0);
  assert.strictEqual(strCall(exports, memory, "pocketbase_valid_name", ""), 0);
  assert.strictEqual(strCall(exports, memory, "pocketbase_collection_kind_code", "auth"), 2);
  assert.strictEqual(strCall(exports, memory, "pocketbase_collection_kind_code", "view"), 1);

  assert.strictEqual(strCall(exports, memory, "pocketbase_actor_kind_code", "admin"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_actor_kind_code", "record"), 2);
  assert.strictEqual(strCall(exports, memory, "pocketbase_actor_kind_code", "file"), 3);
  assert.strictEqual(exports.pocketbase_auth_status(2, 1, 1), 200);
  assert.strictEqual(exports.pocketbase_auth_status(2, 2, 1), 403);
  assert.strictEqual(exports.pocketbase_auth_status(3, 2, 1), 200);
  assert.strictEqual(exports.pocketbase_auth_status(3, 0, 1), 401);
  assert.strictEqual(exports.pocketbase_auth_status(5, 0, 0), 200);

  assert.strictEqual(strCall(exports, memory, "pocketbase_field_type_code", "text"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_field_type_code", "email"), 4);
  assert.strictEqual(strCall(exports, memory, "pocketbase_field_type_code", "number"), 5);
  assert.strictEqual(strCall(exports, memory, "pocketbase_field_type_code", "geoPoint"), 13);
  assert.strictEqual(strCall(exports, memory, "pocketbase_field_type_code", "password"), 14);
  assert.strictEqual(exports.pocketbase_field_category(4), 1);
  assert.strictEqual(exports.pocketbase_field_category(5), 2);
  assert.strictEqual(exports.pocketbase_field_category(9), 5);
  assert.strictEqual(exports.pocketbase_field_category(13), 9);

  assert.strictEqual(strCall(exports, memory, "pocketbase_email_shape", "a@example.com"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_email_shape", "a@example."), 0);
  assert.strictEqual(strCall(exports, memory, "pocketbase_url_shape", "https://example.com"), 1);
  assert.strictEqual(strCall(exports, memory, "pocketbase_url_shape", "ftp://example.com"), 0);

  assert.strictEqual(exports.pocketbase_pack_status(200), (2 << 16) | 200);
  assert.strictEqual(exports.pocketbase_pack_status(404), (4 << 16) | 404);
  assert.strictEqual(exports.pocketbase_pack_status(42), (5 << 16) | 500);

  console.log(
    JSON.stringify({
      unit: "pocketbase-route-model",
      abi_version: exports.proto_abi_version(),
      standard_id: exports.proto_standard_id(),
      ok: true,
      cases: [
        "method_code",
        "public_health",
        "superuser_collections",
        "rule_bound_records",
        "record_auth_refresh",
        "file_route",
        "admin_auth_public",
        "admin_assets_not_logged",
        "acme_well_known",
        "not_found",
        "valid_name",
        "collection_kind",
        "actor_auth_status",
        "field_type_category",
        "email_url_shape",
        "status_pack",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
