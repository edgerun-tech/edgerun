#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] || "standards/build/wasm/codec-primitives/oci-runtime-state.wat";

const STATUS_CREATING = 0;
const STATUS_CREATED = 1;
const STATUS_RUNNING = 2;
const STATUS_STOPPED = 3;
const STATUS_DELETED = 4;
const STATUS_NONE = 255;

const EVENT_CREATE_BEGIN = 1;
const EVENT_CREATE_COMMIT = 2;
const EVENT_START = 3;
const EVENT_PROCESS_EXIT = 4;
const EVENT_DELETE = 5;

const CLASS_UNKNOWN = 0;
const CLASS_NAMESPACE = 1;
const CLASS_MOUNT = 2;
const CLASS_SECURITY = 3;
const CLASS_RESOURCE = 4;
const CLASS_PROCESS = 5;
const CLASS_EBPF = 6;

const ARCH_X86_64 = 1;
const ARCH_AARCH64 = 2;

function compileWat(file) {
  const wasmPath = path.join(os.tmpdir(), `edgerun-${path.basename(file)}-${process.pid}.wasm`);
  execFileSync("wat2wasm", [file, "-o", wasmPath], { stdio: "pipe" });
  execFileSync("wasm-validate", [wasmPath], { stdio: "pipe" });
  const bytes = fs.readFileSync(wasmPath);
  fs.unlinkSync(wasmPath);
  return bytes;
}

function write(memory, text, ptr = 2048) {
  const bytes = Buffer.from(text, "utf8");
  memory.fill(0, ptr, ptr + bytes.length + 16);
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

(async () => {
  const { instance } = await WebAssembly.instantiate(compileWat(watPath), {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300093);

  for (const status of [
    STATUS_CREATING,
    STATUS_CREATED,
    STATUS_RUNNING,
    STATUS_STOPPED,
    STATUS_DELETED,
  ]) {
    assert.strictEqual(e.oci_status_valid(status), 1);
  }
  for (const status of [-1, 5, 99]) {
    assert.strictEqual(e.oci_status_valid(status), 0);
  }

  for (const [name, code] of [
    ["creating", STATUS_CREATING],
    ["created", STATUS_CREATED],
    ["running", STATUS_RUNNING],
    ["stopped", STATUS_STOPPED],
    ["deleted", STATUS_DELETED],
  ]) {
    const [ptr, len] = write(memory, name);
    assert.strictEqual(e.oci_status_code(ptr, len), code);
  }
  let [ptr, len] = write(memory, "paused");
  assert.strictEqual(e.oci_status_code(ptr, len), -1);

  assert.strictEqual(e.oci_status_terminal(STATUS_RUNNING), 0);
  assert.strictEqual(e.oci_status_terminal(STATUS_STOPPED), 1);
  assert.strictEqual(e.oci_status_terminal(STATUS_DELETED), 1);

  assert.strictEqual(
    e.oci_lifecycle_transition(STATUS_NONE, EVENT_CREATE_BEGIN),
    STATUS_CREATING,
  );
  assert.strictEqual(
    e.oci_lifecycle_transition(STATUS_CREATING, EVENT_CREATE_COMMIT),
    STATUS_CREATED,
  );
  assert.strictEqual(e.oci_lifecycle_transition(STATUS_CREATED, EVENT_START), STATUS_RUNNING);
  assert.strictEqual(
    e.oci_lifecycle_transition(STATUS_RUNNING, EVENT_PROCESS_EXIT),
    STATUS_STOPPED,
  );
  assert.strictEqual(e.oci_lifecycle_transition(STATUS_STOPPED, EVENT_DELETE), STATUS_DELETED);
  assert.strictEqual(e.oci_lifecycle_can_transition(STATUS_RUNNING, EVENT_DELETE), 0);
  assert.strictEqual(e.oci_lifecycle_transition(STATUS_CREATED, EVENT_PROCESS_EXIT), -1);

  for (const op of [1, 2, 3, 24]) {
    assert.strictEqual(e.oci_syscall_op_class(op), CLASS_NAMESPACE);
  }
  for (const op of [4, 5, 6, 7, 8, 9]) {
    assert.strictEqual(e.oci_syscall_op_class(op), CLASS_MOUNT);
  }
  for (const op of [10, 11, 12, 13, 14, 15]) {
    assert.strictEqual(e.oci_syscall_op_class(op), CLASS_SECURITY);
  }
  for (const op of [16, 17, 18, 19]) {
    assert.strictEqual(e.oci_syscall_op_class(op), CLASS_RESOURCE);
  }
  assert.strictEqual(e.oci_syscall_op_class(20), CLASS_EBPF);
  for (const op of [21, 22, 23, 25]) {
    assert.strictEqual(e.oci_syscall_op_class(op), CLASS_PROCESS);
  }
  assert.strictEqual(e.oci_syscall_op_class(999), CLASS_UNKNOWN);

  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 272), CLASS_NAMESPACE);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 428), CLASS_MOUNT);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 317), CLASS_SECURITY);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 302), CLASS_RESOURCE);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 62), CLASS_PROCESS);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_X86_64, 321), CLASS_EBPF);

  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 97), CLASS_NAMESPACE);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 40), CLASS_MOUNT);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 277), CLASS_SECURITY);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 267), CLASS_RESOURCE);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 129), CLASS_PROCESS);
  assert.strictEqual(e.oci_linux_syscall_class(ARCH_AARCH64, 280), CLASS_EBPF);
  assert.strictEqual(e.oci_linux_syscall_class(99, 272), CLASS_UNKNOWN);

  const exited42 = e.oci_pack_exited_wait_status(42);
  assert.strictEqual(exited42, 42 << 8);
  assert.strictEqual(e.oci_wait_status_exited(exited42), 1);
  assert.strictEqual(e.oci_wait_status_signaled(exited42), 0);
  assert.strictEqual(e.oci_exit_code_from_wait_status(exited42), 42);

  const signaled9 = e.oci_pack_signaled_wait_status(9);
  assert.strictEqual(signaled9, 9);
  assert.strictEqual(e.oci_wait_status_exited(signaled9), 0);
  assert.strictEqual(e.oci_wait_status_signaled(signaled9), 1);
  assert.strictEqual(e.oci_exit_code_from_wait_status(signaled9), 137);

  const packed = e.oci_pack_state_exit(STATUS_STOPPED, 137);
  assert.strictEqual(e.oci_packed_state_status(packed), STATUS_STOPPED);
  assert.strictEqual(e.oci_packed_state_exit(packed), 137);

  console.log(
    JSON.stringify({
      unit: "oci-runtime-state",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "status_valid_set",
        "status_string_codes",
        "terminal_statuses",
        "deterministic_lifecycle_transitions",
        "invalid_lifecycle_transitions",
        "portable_syscall_operation_classes",
        "x86_64_syscall_number_classes",
        "aarch64_syscall_number_classes",
        "wait_status_exited",
        "wait_status_signaled",
        "state_exit_pack",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
