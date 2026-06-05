#!/usr/bin/env node

const assert = require("assert");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const watPath =
  process.argv[2] ||
  path.join(__dirname, "../build/wasm/codec-primitives/tracing-otel-core.wat");

function compileWat(file) {
  fs.accessSync(file, fs.constants.R_OK);
  const wasm = execFileSync("wat2wasm", [file, "-o", "-"]);
  execFileSync("wasm-validate", ["-"], { input: wasm });
  return wasm;
}

function put(memory, value, ptr = 4096) {
  const bytes = Buffer.from(value, "ascii");
  memory.fill(0, ptr, ptr + Math.max(128, bytes.length + 1));
  memory.set(bytes, ptr);
  return [ptr, bytes.length];
}

function code(e, memory, fn, value) {
  return e[fn](...put(memory, value));
}

(async () => {
  const wasm = compileWat(watPath);
  await WebAssembly.compile(wasm);
  const { instance } = await WebAssembly.instantiate(wasm, {});
  const e = instance.exports;
  const memory = new Uint8Array(e.memory.buffer);

  assert.equal(e.proto_abi_version(), 2);
  assert.equal(e.proto_standard_id(), 300134);

  assert.equal(code(e, memory, "tracing_level_code", "trace"), 5);
  assert.equal(code(e, memory, "tracing_level_code", "error"), 1);
  assert.equal(e.tracing_level_allows(3, 2), 1);
  assert.equal(e.tracing_level_allows(2, 4), 0);
  assert.equal(e.tracing_directive_specificity(12, 2), 3074);
  assert.equal(e.tracing_directive_allows(4, 3, 1, 2, 2), 1);
  assert.equal(e.tracing_directive_allows(4, 3, 1, 2, 1), 0);
  assert.equal(e.tracing_interest_join(2, 2), 2);
  assert.equal(e.tracing_interest_join(2, 1), 1);
  assert.equal(e.tracing_interest_join(0, 2), 0);
  assert.equal(e.tracing_layer_event_order(1), 10);
  assert.equal(e.tracing_layer_event_order(5), 50);

  assert.equal(code(e, memory, "otel_span_kind_code", "server"), 1);
  assert.equal(code(e, memory, "otel_span_kind_code", "consumer"), 4);
  assert.equal(code(e, memory, "otel_status_code", "ok"), 1);
  assert.equal(code(e, memory, "otel_status_code", "error"), 2);
  assert.equal(e.otel_sampler_decision(1, 0, 0, 0, 0, 0n), 2);
  assert.equal(e.otel_sampler_decision(2, 0, 0, 0, 1000, 0n), 0);
  assert.equal(e.otel_sampler_decision(4, 1, 1, 0, 0, 0n), 2);
  assert.equal(e.otel_sampler_decision(4, 1, 0, 2, 0, 0n), 0);
  assert.equal(e.otel_sampler_decision(4, 0, 0, 2, 0, 0n), 2);
  assert.equal(e.otel_sampler_decision(3, 0, 0, 0, 500, 1n), 2);
  assert.equal(e.otel_sampler_decision(3, 0, 0, 0, 500, 9223372036854775806n), 0);

  assert.equal(e.otel_batch_action(1, 512, 0, 0, 0, 0, 0), 0);
  assert.equal(e.otel_batch_action(512, 512, 0, 0, 0, 0, 0), 1);
  assert.equal(e.otel_batch_action(1, 512, 0, 0, 1, 0, 0), 2);
  assert.equal(e.otel_batch_action(1, 512, 0, 0, 0, 1, 0), 3);
  assert.equal(e.otel_batch_action(1, 512, 1, 0, 0, 0, 0), 4);
  assert.equal(e.otel_batch_action(1, 512, 0, 1, 0, 0, 0), 5);

  assert.equal(code(e, memory, "otel_metric_instrument_code", "monotonic_counter.requests"), 1);
  assert.equal(code(e, memory, "otel_metric_instrument_code", "counter.bytes"), 2);
  assert.equal(code(e, memory, "otel_metric_instrument_code", "histogram.latency"), 3);
  assert.equal(code(e, memory, "otel_metric_instrument_code", "gauge.load"), 4);
  assert.equal(e.otel_aggregation_valid(1, 0, 0), 1);
  assert.equal(e.otel_aggregation_valid(5, 1, 0), 1);
  assert.equal(e.otel_aggregation_valid(5, 0, 0), 0);
  assert.equal(e.otel_aggregation_valid(6, 0, -10), 1);
  assert.equal(e.otel_aggregation_valid(6, 0, 21), 0);
  assert.equal(code(e, memory, "otel_export_result_code", "channel_full"), 3);
  assert.equal(code(e, memory, "otel_export_result_code", "exporter_error"), 6);
  assert.equal(code(e, memory, "tracing_format_mode_code", "json"), 1);
  assert.equal(code(e, memory, "tracing_format_mode_code", "pretty"), 3);

  console.log(JSON.stringify({ unit: "tracing-otel-core", standard_id: e.proto_standard_id(), ok: true }));
})().catch((error) => {
  console.error(error.stack || String(error));
  process.exit(1);
});
