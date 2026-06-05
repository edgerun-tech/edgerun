const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/telemetry-value-core.wat");
const wasm = path.join(os.tmpdir(), `telemetry-value-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300140);

  assert.strictEqual(e.telemetry_log_level_enabled(3, 3), 1);
  assert.strictEqual(e.telemetry_log_level_enabled(4, 3), 0);
  assert.strictEqual(e.telemetry_log_level_enabled(1, 0), 0);
  assert.strictEqual(e.telemetry_log_set_logger_result(0), 0);
  assert.strictEqual(e.telemetry_log_set_logger_result(1), 1);
  assert.strictEqual(e.telemetry_log_macro_evaluates(3, 3, 3, 1), 1);
  assert.strictEqual(e.telemetry_log_macro_evaluates(4, 3, 5, 1), 0);
  assert.strictEqual(e.telemetry_log_macro_evaluates(3, 3, 3, 0), 0);
  assert.strictEqual(e.telemetry_log_dispatch_action(1, 1), 1);
  assert.strictEqual(e.telemetry_log_dispatch_action(0, 1), 0);

  assert.strictEqual(e.telemetry_kv_visit_method(0), 0);
  assert.strictEqual(e.telemetry_kv_visit_method(1), 1);
  assert.strictEqual(e.telemetry_kv_visit_method(12), 12);
  assert.strictEqual(e.telemetry_kv_visit_method(99), -1);
  assert.strictEqual(e.telemetry_kv_option_kind(0, 7), 0);
  assert.strictEqual(e.telemetry_kv_option_kind(1, 7), 7);
  assert.strictEqual(e.telemetry_kv_source_get_steps(5, 2), 3);
  assert.strictEqual(e.telemetry_kv_source_get_steps(5, 9), 5);

  assert.strictEqual(e.telemetry_valuable_visit_route(1), 1);
  assert.strictEqual(e.telemetry_valuable_visit_route(16), 1);
  assert.strictEqual(e.telemetry_valuable_visit_route(20), 2);
  assert.strictEqual(e.telemetry_valuable_visit_route(21), 3);
  assert.strictEqual(e.telemetry_valuable_visit_route(22), 4);
  assert.strictEqual(e.telemetry_valuable_visit_route(23), 5);
  assert.strictEqual(e.telemetry_valuable_visit_route(24), 6);
  assert.strictEqual(e.telemetry_valuable_visit_route(0), 7);
  assert.strictEqual(e.telemetry_valuable_named_iter_count(4), 4);
  assert.strictEqual(e.telemetry_valuable_map_visit_calls(3), 3);

  assert.strictEqual(e.telemetry_otel_span_kind_valid(1), 1);
  assert.strictEqual(e.telemetry_otel_span_kind_valid(5), 1);
  assert.strictEqual(e.telemetry_otel_span_kind_valid(6), 0);
  assert.strictEqual(e.telemetry_otel_status_code(1), 2);
  assert.strictEqual(e.telemetry_otel_status_code(2), 1);
  assert.strictEqual(e.telemetry_otel_status_code(0), 0);
  assert.strictEqual(e.telemetry_otel_event_field_action(1, 0, 0), 1);
  assert.strictEqual(e.telemetry_otel_event_field_action(0, 1, 0), 2);
  assert.strictEqual(e.telemetry_otel_event_field_action(0, 0, 1), 3);
  assert.strictEqual(e.telemetry_otel_event_field_action(0, 0, 0), 4);

  assert.strictEqual(e.telemetry_metric_instrument_kind(1, 1, 1), 1);
  assert.strictEqual(e.telemetry_metric_instrument_kind(1, 3, 1), 2);
  assert.strictEqual(e.telemetry_metric_instrument_kind(2, 1, 1), 3);
  assert.strictEqual(e.telemetry_metric_instrument_kind(2, 1, 0), -1);
  assert.strictEqual(e.telemetry_metric_instrument_kind(2, 3, 1), 4);
  assert.strictEqual(e.telemetry_metric_instrument_kind(3, 1, 1), 5);
  assert.strictEqual(e.telemetry_metric_instrument_kind(3, 3, 1), 6);
  assert.strictEqual(e.telemetry_metric_instrument_kind(4, 1, 1), 7);
  assert.strictEqual(e.telemetry_metric_instrument_kind(4, 2, 1), 8);
  assert.strictEqual(e.telemetry_metric_instrument_kind(4, 3, 1), 9);
  assert.strictEqual(e.telemetry_metric_instrument_kind(0, 2, 1), 0);
  assert.strictEqual(e.telemetry_metric_update_path(1), 1);
  assert.strictEqual(e.telemetry_metric_update_path(0), 2);
  assert.strictEqual(e.telemetry_otel_activation_result(1), 2);
  assert.strictEqual(e.telemetry_otel_activation_result(2), 2);
  assert.strictEqual(e.telemetry_otel_activation_result(0), 0);

  fs.unlinkSync(wasm);
  console.log("telemetry-value-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
