#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/device-primitives/device-types-core.wat");
const wasm = path.join(os.tmpdir(), `device-types-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.device_biometric_assurance(1, 1, 1), 3);
  assert.strictEqual(e.device_biometric_assurance(1, 1, 0), 2);
  assert.strictEqual(e.device_biometric_assurance(1, 0, 0), 1);
  assert.strictEqual(e.device_biometric_satisfies(2, 3), 0);
  assert.strictEqual(e.device_display_update_result(1, 0, 1, 100), 1);
  assert.strictEqual(e.device_display_update_result(1, 100, 1, 0), 2);
  assert.strictEqual(e.device_audio_capture_result(1, 48000, 2), 0);
  assert.strictEqual(e.device_audio_capture_result(0, 48000, 2), 1);
  assert.strictEqual(e.device_audio_playback_result(1, 48000, 2, 8, 1, 401, 0, 0), 5);
  assert.strictEqual(e.device_audio_playback_result(1, 48000, 2, 8, 1, 400, 1, 101), 6);
  assert.strictEqual(e.device_wifi_ap_config_result(0, 0, 0), 1);
  assert.strictEqual(e.device_wifi_ap_config_result(4, 33, 0), 2);
  assert.strictEqual(e.device_wifi_ap_config_result(4, 4, 1), 3);
  assert.strictEqual(e.device_input_read_result(0), 0);
  assert.strictEqual(e.device_npu_workload_result(4), 1);

  assert.strictEqual(e.device_gpu_vendor(0x1002), 1);
  assert.strictEqual(e.device_gpu_vendor(0x8086), 2);
  assert.strictEqual(e.device_gpu_vendor(0x10de), 3);
  assert.strictEqual(e.device_gpu_vendor(0x1af4), 9);
  assert.strictEqual(e.device_nfc_technology_code(5), 5);
  assert.strictEqual(e.device_nfc_technology_code(7), 0);
  assert.strictEqual(e.device_camera_enroll_result(0, 1), 1);
  assert.strictEqual(e.device_camera_enroll_result(4, 0), 2);
  assert.strictEqual(e.device_liveness_challenge_result(0, 1, 0), 1);
  assert.strictEqual(e.device_liveness_challenge_result(10, 0, 0), 2);
  assert.strictEqual(e.device_default_liveness_field(1), 1500);
  assert.strictEqual(e.device_default_liveness_field(2), 1);
  assert.strictEqual(e.device_fingerprint_enroll_result(4, 1), 0);

  assert.strictEqual(e.device_quectel_model_code(3), 3);
  assert.strictEqual(e.device_quectel_model_code(5), 0);
  assert.strictEqual(e.device_quectel_default_config(1), 1);
  assert.strictEqual(e.device_quectel_default_config(3), 4);
  assert.strictEqual(e.device_quectel_pin_valid(0, 0, 0), 1);
  assert.strictEqual(e.device_quectel_pin_valid(1, 4, 1), 1);
  assert.strictEqual(e.device_quectel_pin_valid(1, 5, 1), 0);
  assert.strictEqual(e.device_quectel_runtime_api_result(), 1);
  assert.strictEqual(e.device_quectel_at_command_kind(8), 8);
  assert.strictEqual(e.device_quectel_at_command_kind(9), 0);
  assert.strictEqual(e.device_quectel_state_after_at(1), 2);
  assert.strictEqual(e.device_quectel_state_after_at(2), 5);
  assert.strictEqual(e.device_quectel_state_after_at(8), 1);
  assert.strictEqual(e.device_quectel_power_spec_mv(1, 1), 3700);
  assert.strictEqual(e.device_quectel_power_spec_mv(2, 3), 1800);
  assert.strictEqual(e.device_quectel_power_spec_mv(3, 0), 1710);
  assert.strictEqual(e.device_quectel_uart_default(1), 115200);
  assert.strictEqual(e.device_quectel_adc_divider_mv(1800), 900);
  assert.strictEqual(e.device_quectel_pin_function(2), 1);
  assert.strictEqual(e.device_quectel_pin_function(10), 3);
  assert.strictEqual(e.device_quectel_pin_function(15), 4);
  assert.strictEqual(e.device_quectel_pin_function(24), 5);

  console.log("device types core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
