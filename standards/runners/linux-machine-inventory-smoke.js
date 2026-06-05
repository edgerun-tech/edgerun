#!/usr/bin/env node

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { execFileSync } = require("node:child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/linux-machine-inventory.wat";

const SM_SYSTEMD = 1 << 0;
const SM_OPENRC = 1 << 1;
const SM_RUNIT = 1 << 2;
const SM_S6 = 1 << 3;
const SM_DINIT = 1 << 4;
const SM_SUPERVISOR = 1 << 5;
const SM_CRON = 1 << 6;
const SM_RC_LOCAL = 1 << 7;

const RT_DOCKER = 1 << 0;
const RT_PODMAN = 1 << 1;
const RT_CONTAINERD = 1 << 2;

const ACCESS_ROOT = 1 << 0;
const ACCESS_SUDO = 1 << 1;
const ACCESS_DOAS = 1 << 2;
const ACCESS_LOW_PORT = 1 << 3;

const MODE_FOREGROUND = 1 << 0;
const MODE_USER_SYSTEMD = 1 << 1;
const MODE_SYSTEM_SYSTEMD = 1 << 2;
const MODE_SYSTEM_OPENRC = 1 << 3;
const MODE_SYSTEM_RUNIT = 1 << 4;
const MODE_SYSTEM_S6 = 1 << 5;
const MODE_SYSTEM_DINIT = 1 << 6;
const MODE_USER_SUPERVISOR = 1 << 7;
const MODE_USER_CRON = 1 << 8;
const MODE_SYSTEM_RC_LOCAL = 1 << 9;
const MODE_CONTAINER_DOCKER = 1 << 10;
const MODE_CONTAINER_PODMAN = 1 << 11;
const MODE_CONTAINER_CONTAINERD = 1 << 12;
const MODE_INIT = 1 << 13;
const MODE_BARE_METAL = 1 << 14;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, text, ptr = 2048) {
  const bytes = Buffer.from(text, "ascii");
  memory.fill(0, ptr, ptr + Math.max(256, bytes.length + 32));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function spans(view, out) {
  return {
    keyOff: view.getUint32(out, true),
    keyLen: view.getUint32(out + 4, true),
    valueOff: view.getUint32(out + 8, true),
    valueLen: view.getUint32(out + 12, true),
  };
}

function slice(memory, base, off, len) {
  return Buffer.from(memory.slice(base + off, base + off + len)).toString("ascii");
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);
  const out = 4096;

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300097);

  let pair = write(memory, "linux");
  assert.equal(e.linux_os_code(...pair), 1);
  pair = write(memory, "freebsd");
  assert.equal(e.linux_os_code(...pair), 0);

  for (const [arch, code] of [
    ["x86_64", 1],
    ["amd64", 1],
    ["aarch64", 2],
    ["arm64", 2],
    ["arm", 3],
    ["riscv64", 4],
  ]) {
    pair = write(memory, arch);
    assert.equal(e.linux_arch_code(...pair), code, arch);
  }

  pair = write(memory, "musl libc (x86_64)");
  assert.equal(e.linux_libc_code(...pair), 1);
  pair = write(memory, "ldd (Debian GLIBC 2.36-9) 2.36");
  assert.equal(e.linux_libc_code(...pair), 2);
  pair = write(memory, "ldd (GNU libc) 2.39");
  assert.equal(e.linux_libc_code(...pair), 2);
  pair = write(memory, "unknown libc");
  assert.equal(e.linux_libc_code(...pair), 0);

  for (const [label, code] of [
    ["systemd", 1],
    ["openrc", 2],
    ["runit", 3],
    ["s6", 4],
    ["dinit", 5],
    ["supervisor", 6],
    ["supervisord", 6],
    ["cron", 7],
    ["rc.local", 8],
  ]) {
    pair = write(memory, label);
    assert.equal(e.linux_service_manager_code(...pair), code, label);
    assert.equal(e.linux_service_manager_bit(code), 1 << (code - 1));
  }
  pair = write(memory, "systemd");
  assert.equal(e.linux_pid1_service_manager_code(...pair), 1);

  for (const [label, code] of [
    ["docker", 1],
    ["podman", 2],
    ["containerd", 3],
    ["ctr", 3],
  ]) {
    pair = write(memory, label);
    assert.equal(e.linux_container_runtime_code(...pair), code, label);
    assert.equal(e.linux_container_runtime_bit(code), 1 << (code - 1));
  }

  for (const truthy of ["yes", "true", "1"]) {
    pair = write(memory, truthy);
    assert.equal(e.linux_bool_code(...pair), 1);
  }
  for (const falsey of ["no", "false", "0"]) {
    pair = write(memory, falsey);
    assert.equal(e.linux_bool_code(...pair), 0);
  }
  pair = write(memory, "maybe");
  assert.equal(e.linux_bool_code(...pair), -1);

  assert.equal(e.linux_low_port_bind_allowed(0, 1024), 1);
  assert.equal(e.linux_low_port_bind_allowed(1000, 0), 1);
  assert.equal(e.linux_low_port_bind_allowed(1000, 1024), 0);

  for (const [key, code] of [
    ["os", 1],
    ["arch", 2],
    ["kernel", 3],
    ["libc", 4],
    ["pid1", 5],
    ["uid", 6],
    ["root", 7],
    ["sudo", 8],
    ["doas", 9],
    ["low_port_bind", 10],
    ["writable_paths", 11],
    ["service_managers", 12],
    ["container_runtimes", 13],
    ["available_modes", 14],
  ]) {
    pair = write(memory, ` ${key} `);
    assert.equal(e.linux_inventory_key_code(...pair), code, key);
  }

  pair = write(memory, " service_managers = systemd, cron ");
  assert.equal(e.linux_inventory_line_scan(pair[0], pair[1], out), 12);
  let rec = spans(view, out);
  assert.equal(slice(memory, pair[0], rec.keyOff, rec.keyLen), "service_managers");
  assert.equal(slice(memory, pair[0], rec.valueOff, rec.valueLen), "systemd, cron");

  pair = write(memory, "root: yes");
  assert.equal(e.linux_inventory_line_scan(pair[0], pair[1], out), 7);
  rec = spans(view, out);
  assert.equal(slice(memory, pair[0], rec.keyOff, rec.keyLen), "root");
  assert.equal(slice(memory, pair[0], rec.valueOff, rec.valueLen), "yes");

  pair = write(memory, "no separator");
  assert.equal(e.linux_inventory_line_scan(pair[0], pair[1], out), -1);

  assert.equal(
    e.linux_deployment_mode_bitmask(SM_SYSTEMD | SM_CRON, 0, 0),
    MODE_FOREGROUND | MODE_USER_SYSTEMD | MODE_USER_CRON,
  );
  assert.equal(
    e.linux_deployment_mode_bitmask(SM_SYSTEMD | SM_OPENRC | SM_RUNIT, RT_DOCKER, ACCESS_SUDO),
    MODE_FOREGROUND |
      MODE_USER_SYSTEMD |
      MODE_SYSTEM_SYSTEMD |
      MODE_SYSTEM_OPENRC |
      MODE_SYSTEM_RUNIT |
      MODE_CONTAINER_DOCKER,
  );
  assert.equal(
    e.linux_deployment_mode_bitmask(
      SM_SYSTEMD | SM_S6 | SM_DINIT | SM_SUPERVISOR | SM_RC_LOCAL,
      RT_DOCKER | RT_PODMAN | RT_CONTAINERD,
      ACCESS_ROOT | ACCESS_LOW_PORT,
    ),
    MODE_FOREGROUND |
      MODE_USER_SYSTEMD |
      MODE_SYSTEM_SYSTEMD |
      MODE_SYSTEM_S6 |
      MODE_SYSTEM_DINIT |
      MODE_USER_SUPERVISOR |
      MODE_SYSTEM_RC_LOCAL |
      MODE_CONTAINER_DOCKER |
      MODE_CONTAINER_PODMAN |
      MODE_CONTAINER_CONTAINERD |
      MODE_INIT |
      MODE_BARE_METAL,
  );
  assert.equal(
    e.linux_deployment_mode_bitmask(SM_OPENRC | SM_RUNIT | SM_RC_LOCAL, 0, ACCESS_LOW_PORT),
    MODE_FOREGROUND,
  );

  console.log(
    JSON.stringify({
      unit: "linux-machine-inventory",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "os_arch_libc_labels",
        "pid1_service_manager_labels",
        "container_runtime_labels",
        "root_sudo_doas_low_port_flags",
        "deployment_mode_bitmask_policy",
        "inventory_key_line_parsing",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
