// SPDX-License-Identifier: Apache-2.0
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Debug, Serialize)]
struct ExecutionReportJs {
    bundle_hash: String,
    abi_version: u8,
    runtime_id: String,
    output_hash: String,
    output: Vec<u8>,
    output_len: usize,
    input_len: usize,
    max_memory_bytes: u32,
    max_instructions: u64,
    fuel_limit: u64,
    fuel_remaining: u64,
}

#[derive(Debug, Serialize)]
struct ExecutionDigestReportJs {
    bundle_hash: String,
    abi_version: u8,
    runtime_id: String,
    output_hash: String,
    output_len: usize,
    input_len: usize,
    max_memory_bytes: u32,
    max_instructions: u64,
    fuel_limit: u64,
    fuel_remaining: u64,
}

#[derive(Debug, Serialize)]
struct RuntimeErrorJs {
    code: String,
    message: String,
    trap_code: Option<String>,
    fuel_limit: Option<u64>,
    fuel_remaining: Option<u64>,
}

#[wasm_bindgen]
pub fn validate_wasm_module(wasm: &[u8]) -> Result<(), JsValue> {
    edgerun_runtime::validate_wasm_module(wasm).map_err(js_error)?;
    Ok(())
}

#[wasm_bindgen]
pub fn execute_bundle_payload_bytes_strict(bundle_payload_bytes: &[u8]) -> Result<JsValue, JsValue> {
    let report =
        edgerun_runtime::execute_bundle_payload_bytes_strict(bundle_payload_bytes).map_err(runtime_error_to_js)?;
    to_js_value(ExecutionReportJs {
        bundle_hash: hex::encode(report.bundle_hash),
        abi_version: report.abi_version,
        runtime_id: hex::encode(report.runtime_id),
        output_hash: hex::encode(report.output_hash),
        output_len: report.output.len(),
        output: report.output,
        input_len: report.input_len,
        max_memory_bytes: report.max_memory_bytes,
        max_instructions: report.max_instructions,
        fuel_limit: report.fuel_limit,
        fuel_remaining: report.fuel_remaining,
    })
}

#[wasm_bindgen]
pub fn execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id_hex: &str,
    expected_abi_version: u8,
) -> Result<JsValue, JsValue> {
    let expected_runtime_id = parse_hex_32(expected_runtime_id_hex)?;
    let report = edgerun_runtime::execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
        bundle_payload_bytes,
        expected_runtime_id,
        expected_abi_version,
    )
    .map_err(runtime_error_to_js)?;
    to_js_value(ExecutionDigestReportJs {
        bundle_hash: hex::encode(report.bundle_hash),
        abi_version: report.abi_version,
        runtime_id: hex::encode(report.runtime_id),
        output_hash: hex::encode(report.output_hash),
        output_len: report.output_len,
        input_len: report.input_len,
        max_memory_bytes: report.max_memory_bytes,
        max_instructions: report.max_instructions,
        fuel_limit: report.fuel_limit,
        fuel_remaining: report.fuel_remaining,
    })
}

fn parse_hex_32(input: &str) -> Result<[u8; 32], JsValue> {
    let trimmed = input.trim();
    let hex_str = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    let bytes = hex::decode(hex_str).map_err(|e| js_error(format!("runtime_id must be hex: {e}")))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| js_error(format!("runtime_id must decode to 32 bytes, got {}", bytes.len())))?;
    Ok(arr)
}

fn to_js_value<T: Serialize>(value: T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&value).map_err(js_error)
}

fn runtime_error_to_js(err: edgerun_runtime::RuntimeError) -> JsValue {
    let mapped = RuntimeErrorJs {
        code: format!("{:?}", err.code),
        message: err.message,
        trap_code: err.trap_code,
        fuel_limit: err.fuel_limit,
        fuel_remaining: err.fuel_remaining,
    };
    match serde_wasm_bindgen::to_value(&mapped) {
        Ok(value) => value,
        Err(ser_err) => js_error(format!("runtime error serialization failed: {ser_err}")),
    }
}

fn js_error(message: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&message.to_string())
}
