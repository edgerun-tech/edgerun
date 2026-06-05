#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/agent-primitives/codex-agent-core.wat");
const wasm = path.join(os.tmpdir(), `codex-agent-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const put = (s, ptr = 2048) => {
    mem.fill(0, ptr, ptr + 64);
    mem.set(Buffer.from(s, "utf8"), ptr);
    return [ptr, Buffer.byteLength(s)];
  };
  const schema = (s) => e.codex_schema_type_code(...put(s));

  assert.strictEqual(schema("string"), 1);
  assert.strictEqual(schema("number"), 2);
  assert.strictEqual(schema("boolean"), 3);
  assert.strictEqual(schema("integer"), 4);
  assert.strictEqual(schema("object"), 5);
  assert.strictEqual(schema("array"), 6);
  assert.strictEqual(schema("null"), 7);
  assert.strictEqual(schema("bad"), 0);
  assert.strictEqual(e.codex_schema_infer_type(0, 0, 1, 0, 0, 0), 5);
  assert.strictEqual(e.codex_schema_infer_type(0, 0, 0, 1, 0, 0), 6);
  assert.strictEqual(e.codex_schema_infer_type(0, 0, 0, 0, 1, 0), 1);
  assert.strictEqual(e.codex_schema_infer_type(0, 0, 0, 0, 0, 1), 2);
  assert.strictEqual(e.codex_schema_parse_result(7), 1);

  assert.strictEqual(e.codex_tool_environment_mode(0), 0);
  assert.strictEqual(e.codex_tool_environment_mode(1), 1);
  assert.strictEqual(e.codex_tool_environment_mode(2), 2);
  assert.strictEqual(e.codex_tool_shell_type(0, 0, 1, 2, 1), 0);
  assert.strictEqual(e.codex_tool_shell_type(1, 1, 1, 2, 1), 1);
  assert.strictEqual(e.codex_tool_shell_type(1, 0, 1, 1, 1), 2);
  assert.strictEqual(e.codex_tool_shell_type(1, 0, 0, 2, 1), 1);
  assert.strictEqual(e.codex_unified_exec_mode(1, 1, 1, 1, 1, 1), 1);
  assert.strictEqual(e.codex_unified_exec_mode(1, 1, 0, 1, 1, 1), 0);
  assert.strictEqual(e.codex_tool_gates(1, 1, 1, 1, 1, 1, 1, 1), 7);
  assert.strictEqual(e.codex_tool_gates(1, 0, 1, 1, 1, 1, 1, 1), 6);
  assert.strictEqual(e.codex_apply_patch_tool_type(0, 1), 2);
  assert.strictEqual(e.codex_apply_patch_tool_type(1, 1), 1);
  assert.strictEqual(e.codex_request_user_input_mode_available(0, 1, 1), 1);

  assert.strictEqual(e.codex_request_body_result(0, 0, 0, 0), 0);
  assert.strictEqual(e.codex_request_body_result(2, 1, 0, 0), 1);
  assert.strictEqual(e.codex_request_body_result(1, 1, 0, 1), 2);
  assert.strictEqual(e.codex_request_body_result(1, 1, 0, 0), 3);
  assert.strictEqual(e.codex_request_body_result(1, 0, 0, 0), 4);
  assert.strictEqual(e.codex_retry_should_retry(1, 1, 1, 0, 4, 1, 429), 1);
  assert.strictEqual(e.codex_retry_should_retry(0, 1, 1, 0, 4, 1, 503), 1);
  assert.strictEqual(e.codex_retry_should_retry(0, 1, 1, 4, 4, 1, 503), 0);
  assert.strictEqual(e.codex_retry_should_retry(0, 0, 1, 0, 4, 2, 0), 1);
  assert.strictEqual(e.codex_retry_backoff_nominal_ms(200, 0), 200);
  assert.strictEqual(e.codex_retry_backoff_nominal_ms(200, 3), 800);
  assert.strictEqual(e.codex_sse_event_result(1), 0);
  assert.strictEqual(e.codex_sse_event_result(4), 3);

  assert.strictEqual(e.codex_provider_validate_result(1, 1, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.codex_provider_validate_result(1, 0, 1, 0, 0, 0, 0), 2);
  assert.strictEqual(e.codex_provider_validate_result(0, 0, 0, 0, 1, 0, 1), 3);
  assert.strictEqual(e.codex_provider_validate_result(0, 0, 1, 0, 1, 0, 0), 4);
  assert.strictEqual(e.codex_provider_retry_cap(-1, 0), 4);
  assert.strictEqual(e.codex_provider_retry_cap(-1, 1), 5);
  assert.strictEqual(e.codex_provider_retry_cap(101, 0), 100);
  assert.strictEqual(e.codex_provider_timeout_ms(-1, 0), 300000);
  assert.strictEqual(e.codex_provider_timeout_ms(-1, 1), 15000);
  assert.strictEqual(e.codex_model_cache_ttl_seconds(), 300);
  assert.strictEqual(e.codex_refresh_strategy_allows_network(1, 1), 1);
  assert.strictEqual(e.codex_refresh_strategy_allows_network(2, 0), 0);
  assert.strictEqual(e.codex_refresh_strategy_allows_network(3, 0), 1);
  assert.strictEqual(e.codex_refresh_strategy_allows_network(3, 1), 0);

  assert.strictEqual(e.codex_pipeline_stage_count(), 7);
  assert.strictEqual(e.codex_pipeline_stage_kind(0), 1);
  assert.strictEqual(e.codex_pipeline_stage_kind(6), 7);
  assert.strictEqual(e.codex_pipeline_route_mask(1), 0x43);
  assert.strictEqual(e.codex_pipeline_route_mask(2), 0x77);
  assert.strictEqual(e.codex_pipeline_route_mask(3), 0x7f);
  assert.strictEqual(e.codex_pipeline_route_mask(4), 0x67);
  assert.strictEqual(e.codex_pipeline_limit(1), 2400);
  assert.strictEqual(e.codex_pipeline_limit(8), 4);

  assert.strictEqual(e.codex_code_mode_parse_result(1, 0, 0, 0, 0, 0), 1);
  assert.strictEqual(e.codex_code_mode_parse_result(0, 1, 1, 1, 0, 1), 2);
  assert.strictEqual(e.codex_code_mode_parse_result(0, 1, 0, 0, 0, 1), 3);
  assert.strictEqual(e.codex_code_mode_parse_result(0, 1, 0, 1, 1, 1), 4);
  assert.strictEqual(e.codex_code_mode_parse_result(0, 1, 0, 1, 0, 0), 5);
  assert.strictEqual(e.codex_code_mode_identifier_char("_".charCodeAt(0), 0), 1);
  assert.strictEqual(e.codex_code_mode_identifier_char("9".charCodeAt(0), 0), 0);
  assert.strictEqual(e.codex_code_mode_identifier_char("9".charCodeAt(0), 1), 1);
  assert.strictEqual(e.codex_ui_patch_kind(50), 50);
  assert.strictEqual(e.codex_ui_patch_kind(51), -1);
  assert.strictEqual(e.codex_ui_agent_component_id(8), 8);
  assert.strictEqual(e.codex_ui_agent_component_id(9), 0);
  assert.strictEqual(e.codex_ui_string_payload_len(255), 256);
  assert.strictEqual(e.codex_ui_string_payload_len(256), -1);

  console.log("codex agent core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
