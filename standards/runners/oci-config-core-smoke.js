#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/oci-config-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, text, ptr = 1024) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function callText(exports, memory, fn, text) {
  return exports[fn](...put(memory, text));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300100);

  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "mount"), 1);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "pid"), 2);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "network"), 3);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "ipc"), 4);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "uts"), 5);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "user"), 6);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "cgroup"), 7);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "time"), 8);
  assert.strictEqual(callText(e, memory, "oci_namespace_kind", "weird"), 0);
  assert.strictEqual(e.oci_namespace_flag(1) | e.oci_namespace_flag(2) | e.oci_namespace_flag(3), 7);
  assert.strictEqual(e.oci_default_namespace_mask(), 31);

  assert.strictEqual(callText(e, memory, "oci_pull_policy", "missing"), 1);
  assert.strictEqual(callText(e, memory, "oci_pull_policy", "if-missing"), 1);
  assert.strictEqual(callText(e, memory, "oci_pull_policy", "always"), 2);
  assert.strictEqual(callText(e, memory, "oci_pull_policy", "never"), 3);
  assert.strictEqual(callText(e, memory, "oci_pull_policy", "sometimes"), 0);

  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "ro"), 1);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "rw"), 2);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "bind"), 4);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "rbind"), 12);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "nosuid"), 16);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "nodev"), 32);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "noexec"), 64);
  assert.strictEqual(callText(e, memory, "oci_mount_option_flag", "private"), 1024);

  assert.strictEqual(callText(e, memory, "oci_path_absolute", "/var/lib"), 1);
  assert.strictEqual(callText(e, memory, "oci_path_absolute", "var/lib"), 0);
  assert.strictEqual(callText(e, memory, "oci_rootfs_relative_path_valid", "/app/data"), 1);
  assert.strictEqual(callText(e, memory, "oci_rootfs_relative_path_valid", "app/../escape"), 0);
  assert.strictEqual(callText(e, memory, "oci_rootfs_relative_path_valid", "../escape"), 0);

  assert.strictEqual(callText(e, memory, "oci_container_id_valid", "ctr-01_ok.name"), 1);
  assert.strictEqual(callText(e, memory, "oci_container_id_valid", "-bad"), 0);
  assert.strictEqual(callText(e, memory, "oci_container_id_valid", "bad space"), 0);
  assert.strictEqual(callText(e, memory, "oci_hostname_valid", "edge.node-1"), 1);
  assert.strictEqual(callText(e, memory, "oci_hostname_valid", "bad_host"), 0);
  assert.strictEqual(callText(e, memory, "oci_dns_server_valid", "1.1.1.1"), 1);
  assert.strictEqual(callText(e, memory, "oci_dns_server_valid", "2001:db8::1"), 1);
  assert.strictEqual(callText(e, memory, "oci_dns_server_valid", "not-an-ip"), 0);

  console.log(
    JSON.stringify({
      unit: "oci-config-core",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "namespace_kind_mapping",
        "namespace_flag_mask",
        "default_namespace_mask",
        "pull_policy",
        "mount_option_flags",
        "absolute_path",
        "rootfs_relative_escape_reject",
        "container_id_validity",
        "hostname_validity",
        "dns_server_validity",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
