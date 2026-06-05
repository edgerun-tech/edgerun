#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/dhcpv6-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300111);

  assert.strictEqual(e.dhcpv6_message_type(1), 1);
  assert.strictEqual(e.dhcpv6_message_type(13), 13);
  assert.strictEqual(e.dhcpv6_message_type(14), 0);
  assert.strictEqual(e.dhcpv6_message_role(1), 1);
  assert.strictEqual(e.dhcpv6_message_role(7), 2);
  assert.strictEqual(e.dhcpv6_message_role(12), 3);
  assert.strictEqual(e.dhcpv6_transaction_id(0x12, 0x34, 0x56), 0x123456);

  assert.strictEqual(e.dhcpv6_option_class(1), 1);
  assert.strictEqual(e.dhcpv6_option_class(3), 2);
  assert.strictEqual(e.dhcpv6_option_class(23), 3);
  assert.strictEqual(e.dhcpv6_option_class(13), 4);
  assert.strictEqual(e.dhcpv6_option_class(9), 5);

  assert.strictEqual(e.dhcpv6_option_length_status(3, 12), 1);
  assert.strictEqual(e.dhcpv6_option_length_status(3, 11), 0);
  assert.strictEqual(e.dhcpv6_option_length_status(5, 24), 1);
  assert.strictEqual(e.dhcpv6_option_length_status(6, 6), 1);
  assert.strictEqual(e.dhcpv6_option_length_status(6, 5), 0);
  assert.strictEqual(e.dhcpv6_option_length_status(23, 32), 1);
  assert.strictEqual(e.dhcpv6_option_length_status(23, 31), 0);

  assert.strictEqual(e.dhcpv6_status_code(0), 1);
  assert.strictEqual(e.dhcpv6_status_code(6), 1);
  assert.strictEqual(e.dhcpv6_status_code(8), 2);
  assert.strictEqual(e.dhcpv6_status_code(99), 0);
  assert.strictEqual(e.dhcpv6_duid_type(1, 8), 1);
  assert.strictEqual(e.dhcpv6_duid_type(3, 4), 1);
  assert.strictEqual(e.dhcpv6_duid_type(4, 18), 1);
  assert.strictEqual(e.dhcpv6_duid_type(4, 17), 0);

  assert.strictEqual(e.dhcpv6_server_response(1, 1, 1, 0), 2);
  assert.strictEqual(e.dhcpv6_server_response(1, 1, 1, 1), 7);
  assert.strictEqual(e.dhcpv6_server_response(1, 1, 0, 0), 0);
  assert.strictEqual(e.dhcpv6_server_response(3, 1, 1, 0), 7);
  assert.strictEqual(e.dhcpv6_default_lifetime(1), 3600);
  assert.strictEqual(e.dhcpv6_default_lifetime(3), 86400);
  assert.strictEqual(e.dhcpv6_default_lifetime(5), 2700);

  console.log(
    JSON.stringify({
      unit: "dhcpv6-core",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "message_type_role",
        "transaction_id",
        "option_class",
        "option_length_status",
        "status_code",
        "duid_shape",
        "server_response",
        "default_lifetime_policy",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
