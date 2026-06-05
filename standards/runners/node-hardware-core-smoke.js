#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/node-hardware-core.wat");

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

function callText(e, memory, name, text, ptr = 1024) {
  return e[name](...put(memory, text, ptr));
}

function route(e, memory, method, path) {
  const m = put(memory, method, 1024);
  const p = put(memory, path, 2048);
  return e.node_health_route(m[0], m[1], p[0], p[1]);
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300105);

  assert.strictEqual(callText(e, memory, "node_hw_pci_address_valid", "0000:03:00.0"), 1);
  assert.strictEqual(callText(e, memory, "node_hw_pci_address_valid", "0000:zz:00.0"), 0);
  assert.strictEqual(callText(e, memory, "node_hw_pci_address_valid", "03:00.0"), 0);

  assert.strictEqual(e.node_hw_gpu_class(0x030000), 1);
  assert.strictEqual(e.node_hw_gpu_class(0x120000), 2);
  assert.strictEqual(e.node_hw_gpu_class(0x020000), 0);
  assert.strictEqual(e.node_hw_gpu_vendor(0x1002), 1);
  assert.strictEqual(e.node_hw_gpu_vendor(0x8086), 2);
  assert.strictEqual(e.node_hw_gpu_vendor(0x10de), 3);
  assert.strictEqual(e.node_hw_gpu_vendor(0x1af4), 4);
  assert.strictEqual(e.node_hw_gpu_capabilities(1, 0x030000, 1, 1, 1), 7);
  assert.strictEqual(e.node_hw_gpu_capabilities(0, 0x020000, 0, 0, 0), 0);

  assert.strictEqual(callText(e, memory, "node_hw_connector_status", "connected"), 1);
  assert.strictEqual(callText(e, memory, "node_hw_connector_status", "disconnected"), 2);
  assert.strictEqual(callText(e, memory, "node_hw_connector_status", "enabled"), 3);
  assert.strictEqual(callText(e, memory, "node_hw_connector_status", "disabled"), 4);

  let out = 4096;
  assert.strictEqual(e.node_hw_display_mode_parse(...put(memory, "1920x1080", 1024), out), 1);
  assert.strictEqual(view.getUint32(out, true), 1920);
  assert.strictEqual(view.getUint32(out + 4, true), 1080);
  assert.strictEqual(view.getUint32(out + 8, true), 60000);
  assert.strictEqual(e.node_hw_display_mode_parse(...put(memory, "1280x720p@120", 1024), out), 1);
  assert.strictEqual(view.getUint32(out + 8, true), 120000);
  assert.strictEqual(e.node_hw_display_mode_parse(...put(memory, "bad", 1024), out), 0);

  assert.strictEqual(callText(e, memory, "node_hw_npu_driver", "amdxdna"), 1);
  assert.strictEqual(callText(e, memory, "node_hw_npu_driver", "ivpu"), 2);
  assert.strictEqual(callText(e, memory, "node_hw_npu_driver", "intel_vpu"), 2);
  assert.strictEqual(callText(e, memory, "node_hw_npu_driver", "other"), 0);

  assert.strictEqual(route(e, memory, "OPTIONS", "/anything"), 200);
  assert.strictEqual(route(e, memory, "GET", "/health"), 200);
  assert.strictEqual(route(e, memory, "GET", "/protocol/node/status"), 200);
  assert.strictEqual(route(e, memory, "GET", "/protocol/approvals"), 200);
  assert.strictEqual(route(e, memory, "POST", "/protocol/tools/invoke"), 200);
  assert.strictEqual(route(e, memory, "POST", "/health"), 404);

  console.log(
    JSON.stringify({
      unit: "node-hardware-core",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "pci_address_validity",
        "gpu_class_mapping",
        "gpu_vendor_mapping",
        "gpu_capability_bits",
        "connector_status",
        "display_mode_parse",
        "npu_driver_mapping",
        "health_control_route_status",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
