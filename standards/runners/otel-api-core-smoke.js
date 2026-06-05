const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");
const { execFileSync } = require("child_process");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/codec-primitives/otel-api-core.wat");
const wasm = path.join(os.tmpdir(), `otel-api-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm]);
const bytes = fs.readFileSync(wasm);

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes);
  const e = instance.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const view = new DataView(e.memory.buffer);

  function put(s, off = 64) {
    mem.set(Buffer.from(s), off);
    return [off, Buffer.byteLength(s)];
  }
  function callString(fn, s) {
    return e[fn](...put(s));
  }
  function putF64(values, off = 512) {
    values.forEach((v, i) => view.setFloat64(off + i * 8, v, true));
    return off;
  }

  assert.strictEqual(e.proto_abi_version(), 2);
  assert.strictEqual(e.proto_standard_id(), 300135);

  assert.strictEqual(callString("otel_baggage_key_valid", "tenant_id"), 1);
  assert.strictEqual(callString("otel_baggage_key_valid", ""), 0);
  assert.strictEqual(callString("otel_baggage_key_valid", "bad,key"), 0);
  assert.strictEqual(callString("otel_baggage_key_valid", "bad=value"), 0);
  assert.strictEqual(e.otel_baggage_insert_result(0, 0, 3, 5, 0, 0, 0, 1), 0);
  assert.strictEqual(e.otel_baggage_insert_result(64, 10, 3, 5, 0, 0, 0, 1), 3);
  assert.strictEqual(e.otel_baggage_insert_result(1, 8190, 3, 5, 0, 0, 0, 1), 4);
  assert.strictEqual(e.otel_baggage_insert_result(1, 20, 3, 7, 0, 1, 8, 1), 1);
  assert.strictEqual(e.otel_baggage_insert_result(1, 20, 3, 7, 0, 1, 8, 0), 2);

  assert.strictEqual(callString("otel_tracestate_key_valid", "foo"), 1);
  assert.strictEqual(callString("otel_tracestate_key_valid", "123"), 1);
  assert.strictEqual(callString("otel_tracestate_key_valid", "foo@bar"), 1);
  assert.strictEqual(callString("otel_tracestate_key_valid", "foo@012345678"), 1);
  assert.strictEqual(callString("otel_tracestate_key_valid", "foo@0123456789abcdef"), 0);
  assert.strictEqual(callString("otel_tracestate_key_valid", "FOO"), 0);
  assert.strictEqual(callString("otel_tracestate_value_valid", "bar"), 1);
  assert.strictEqual(callString("otel_tracestate_value_valid", "bad,value"), 0);
  assert.strictEqual(callString("otel_tracestate_value_valid", "bad=value"), 0);

  assert.strictEqual(e.otel_trace_flags_is_sampled(0), 0);
  assert.strictEqual(e.otel_trace_flags_is_sampled(3), 1);
  assert.strictEqual(e.otel_trace_flags_with_sampled(2, 1), 3);
  assert.strictEqual(e.otel_trace_flags_with_sampled(3, 0), 2);
  assert.strictEqual(e.otel_span_context_valid(1, 1), 1);
  assert.strictEqual(e.otel_span_context_valid(1, 0), 0);

  assert.strictEqual(
    callString("otel_traceparent_valid", "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
    1,
  );
  assert.strictEqual(
    callString("otel_traceparent_valid", "00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
    0,
  );
  assert.strictEqual(
    callString("otel_traceparent_valid", "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01"),
    0,
  );

  assert.strictEqual(e.otel_status_apply(0, 1), 1);
  assert.strictEqual(e.otel_status_apply(1, 2), 2);
  assert.strictEqual(e.otel_status_apply(2, 1), 2);

  assert.strictEqual(e.otel_span_kind_relation(1), 9);
  assert.strictEqual(e.otel_span_kind_relation(2), 5);
  assert.strictEqual(e.otel_span_kind_relation(3), 10);
  assert.strictEqual(e.otel_span_kind_relation(4), 6);
  assert.strictEqual(e.otel_span_kind_relation(5), 0);

  assert.strictEqual(callString("otel_metric_unit_valid", "ms"), 1);
  assert.strictEqual(callString("otel_metric_unit_valid", "x".repeat(64)), 0);
  assert.strictEqual(e.otel_histogram_boundaries_valid(putF64([0, 5, 10]), 3), 1);
  assert.strictEqual(e.otel_histogram_boundaries_valid(putF64([0, 5, 5]), 3), 0);
  assert.strictEqual(e.otel_histogram_boundaries_valid(putF64([0, Number.NaN]), 2), 0);

  assert.strictEqual(e.otel_log_severity_band(1), 1);
  assert.strictEqual(e.otel_log_severity_band(9), 3);
  assert.strictEqual(e.otel_log_severity_band(17), 5);
  assert.strictEqual(e.otel_log_severity_band(24), 6);
  assert.strictEqual(e.otel_log_severity_band(25), 0);
  assert.strictEqual(e.otel_composite_steps(3), 3);

  fs.unlinkSync(wasm);
  console.log("otel-api-core smoke ok");
})().catch((err) => {
  try { fs.unlinkSync(wasm); } catch (_) {}
  throw err;
});
