#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/codex-app-protocol-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function writeAscii(memory, value, ptr = 2048) {
  memory.fill(0, ptr, ptr + Math.max(256, value.length + 1));
  memory.set(Buffer.from(value, "ascii"), ptr);
  return ptr;
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  const code = (fn, value) => {
    const ptr = writeAscii(memory, value);
    return e[fn](ptr, value.length);
  };

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300112);

  assert.equal(e.codex_jsonrpc_shape(1, 1, 0, 0), 1);
  assert.equal(e.codex_jsonrpc_shape(0, 1, 0, 0), 2);
  assert.equal(e.codex_jsonrpc_shape(1, 0, 1, 0), 3);
  assert.equal(e.codex_jsonrpc_shape(1, 0, 0, 1), 4);
  assert.equal(e.codex_jsonrpc_shape(0, 0, 0, 0), 0);

  assert.equal(code("codex_turn_status_code", "completed"), 1);
  assert.equal(code("codex_turn_status_code", "interrupted"), 2);
  assert.equal(code("codex_turn_status_code", "inProgress"), 4);

  assert.equal(code("codex_item_kind_code", "userMessage"), 1);
  assert.equal(code("codex_item_kind_code", "mcpToolCall"), 10);
  assert.equal(code("codex_item_kind_code", "contextCompaction"), 11);
  assert.equal(code("codex_item_kind_code", "function_call_output"), 13);

  assert.equal(code("codex_permission_code", "managed"), 1);
  assert.equal(code("codex_permission_code", "external"), 3);
  assert.equal(code("codex_permission_code", "write"), 5);
  assert.equal(code("codex_permission_code", "enabled"), 7);

  assert.equal(code("codex_approval_decision_code", "accept"), 1);
  assert.equal(code("codex_approval_decision_code", "acceptForSession"), 2);
  assert.equal(code("codex_approval_decision_code", "applyNetworkPolicyAmendment"), 4);
  assert.equal(code("codex_approval_decision_code", "cancel"), 6);

  assert.equal(code("codex_exec_status_code", "completed"), 1);
  assert.equal(code("codex_exec_status_code", "declined"), 3);
  assert.equal(code("codex_mcp_status_code", "inProgress"), 1);
  assert.equal(code("codex_mcp_status_code", "starting"), 4);

  assert.equal(code("codex_network_protocol_code", "http"), 1);
  assert.equal(code("codex_network_protocol_code", "https_connect"), 2);
  assert.equal(code("codex_network_protocol_code", "http-connect"), 2);
  assert.equal(code("codex_network_protocol_code", "socks5_udp"), 4);

  assert.equal(code("codex_method_scope_code", "thread/resume"), 1);
  assert.equal(code("codex_method_scope_code", "turn/start"), 1);
  assert.equal(code("codex_method_scope_code", "command/exec/write"), 2);
  assert.equal(code("codex_method_scope_code", "process/spawn"), 3);
  assert.equal(code("codex_method_scope_code", "fuzzyFileSearch/sessionUpdate"), 4);
  assert.equal(code("codex_method_scope_code", "fs/watch"), 5);
  assert.equal(code("codex_method_scope_code", "config/read"), 7);
  assert.equal(code("codex_method_scope_code", "plugin/install"), 6);
  assert.equal(code("codex_method_scope_code", "config/mcpServer/reload"), 9);
  assert.equal(code("codex_method_scope_code", "mcpServer/oauth/login"), 8);
  assert.equal(code("codex_method_scope_code", "account/login/start"), 10);
  assert.equal(code("codex_method_scope_code", "memory/reset"), 11);
  assert.equal(code("codex_method_scope_code", "thread/list"), 0);

  assert.equal(code("codex_method_family_code", "thread/realtime/start"), 1);
  assert.equal(code("codex_method_family_code", "mcpServer/tool/call"), 0);
  assert.equal(code("codex_method_family_code", "app/list"), 5);

  console.log(
    JSON.stringify({
      unit: "codex-app-protocol-core",
      abi_version: e.proto_abi_version(),
      standard_id: e.proto_standard_id(),
      ok: true,
      cases: [
        "jsonrpc_shape",
        "turn_status",
        "item_kind",
        "permission",
        "approval_decision",
        "exec_mcp_status",
        "network_protocol_aliases",
        "method_scope",
      ],
    }),
  );
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
