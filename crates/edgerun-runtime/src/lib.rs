// SPDX-License-Identifier: Apache-2.0
mod bundle;
mod validate;

use anyhow::{anyhow, Result};
use thiserror::Error;
use wasmi::{
    Caller, Config, Engine, Linker, Memory, Module, Store, StoreLimitsBuilder, TrapCode, TypedFunc,
};

pub use bundle::decode_bundle_from_canonical_bytes;
pub use validate::validate_wasm_module;

#[derive(Debug, Clone)]
pub struct ExecutionReport {
    pub bundle_hash: [u8; 32],
    pub abi_version: u8,
    pub runtime_id: [u8; 32],
    pub output_hash: [u8; 32],
    pub output: Vec<u8>,
    pub input_len: usize,
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionDigestReport {
    pub bundle_hash: [u8; 32],
    pub abi_version: u8,
    pub runtime_id: [u8; 32],
    pub output_hash: [u8; 32],
    pub output_len: usize,
    pub input_len: usize,
    pub max_memory_bytes: u32,
    pub max_instructions: u64,
    pub fuel_limit: u64,
    pub fuel_remaining: u64,
}

#[derive(Debug, Clone)]
struct ExecutionOutcome {
    output: Option<Vec<u8>>,
    output_hash: [u8; 32],
    output_len: usize,
    fuel_remaining: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Buffered,
    HashOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeErrorCode {
    BundleDecode,
    AbiVersionMismatch,
    RuntimeIdMismatch,
    ValidationFailed,
    EngineCompile,
    InstantiationFailed,
    MissingMemoryExport,
    StartFunctionMissing,
    FuelConfiguration,
    InstructionLimitExceeded,
    MemoryLimitExceeded,
    OutputContractViolation,
    HostcallFailed,
    Trap,
}

#[derive(Debug, Error)]
#[error("[{code:?}] {message}")]
pub struct RuntimeError {
    pub code: RuntimeErrorCode,
    pub message: String,
    pub trap_code: Option<String>,
    pub fuel_limit: Option<u64>,
    pub fuel_remaining: Option<u64>,
}

impl RuntimeError {
    fn new(code: RuntimeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            trap_code: None,
            fuel_limit: None,
            fuel_remaining: None,
        }
    }

    fn with_trap_code(mut self, trap_code: impl Into<String>) -> Self {
        self.trap_code = Some(trap_code.into());
        self
    }

    fn with_fuel(mut self, fuel_limit: u64, fuel_remaining: Option<u64>) -> Self {
        self.fuel_limit = Some(fuel_limit);
        self.fuel_remaining = fuel_remaining;
        self
    }
}

#[derive(Debug, Clone)]
struct RuntimeHostState {
    input: Vec<u8>,
    output: Vec<u8>,
    output_len: usize,
    output_hasher: blake3::Hasher,
    output_mode: OutputMode,
    output_write_calls: u32,
    max_output_bytes: usize,
    memory: Option<Memory>,
}

#[derive(Debug)]
struct RuntimeStoreData {
    host: RuntimeHostState,
    limits: wasmi::StoreLimits,
}

pub fn execute_bundle_payload_bytes(bundle_payload_bytes: &[u8]) -> Result<ExecutionReport> {
    execute_bundle_payload_bytes_strict(bundle_payload_bytes)
        .map_err(|e| anyhow!("[{:?}] {}", e.code, e.message))
}

pub fn execute_bundle_payload_bytes_for_runtime(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: [u8; 32],
) -> Result<ExecutionReport> {
    execute_bundle_payload_bytes_with_policy_strict(
        bundle_payload_bytes,
        Some(expected_runtime_id),
        None,
    )
    .map_err(|e| anyhow!("[{:?}] {}", e.code, e.message))
}

pub fn execute_bundle_payload_bytes_for_runtime_and_abi(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: [u8; 32],
    expected_abi_version: u8,
) -> Result<ExecutionReport> {
    execute_bundle_payload_bytes_with_policy_strict(
        bundle_payload_bytes,
        Some(expected_runtime_id),
        Some(expected_abi_version),
    )
    .map_err(|e| anyhow!("[{:?}] {}", e.code, e.message))
}

pub fn execute_bundle_payload_bytes_strict(
    bundle_payload_bytes: &[u8],
) -> core::result::Result<ExecutionReport, RuntimeError> {
    execute_bundle_payload_bytes_with_policy_strict(bundle_payload_bytes, None, None)
}

pub fn execute_bundle_payload_bytes_for_runtime_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: [u8; 32],
) -> core::result::Result<ExecutionReport, RuntimeError> {
    execute_bundle_payload_bytes_with_policy_strict(
        bundle_payload_bytes,
        Some(expected_runtime_id),
        None,
    )
}

pub fn execute_bundle_payload_bytes_for_runtime_and_abi_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: [u8; 32],
    expected_abi_version: u8,
) -> core::result::Result<ExecutionReport, RuntimeError> {
    execute_bundle_payload_bytes_with_policy_strict(
        bundle_payload_bytes,
        Some(expected_runtime_id),
        Some(expected_abi_version),
    )
}

pub fn execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: [u8; 32],
    expected_abi_version: u8,
) -> core::result::Result<ExecutionDigestReport, RuntimeError> {
    execute_bundle_payload_bytes_with_policy_digest_strict(
        bundle_payload_bytes,
        Some(expected_runtime_id),
        Some(expected_abi_version),
    )
}

fn execute_bundle_payload_bytes_with_policy_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: Option<[u8; 32]>,
    expected_abi_version: Option<u8>,
) -> core::result::Result<ExecutionReport, RuntimeError> {
    let (bundle_hash, bundle) = decode_validate_with_policy(
        bundle_payload_bytes,
        expected_runtime_id,
        expected_abi_version,
    )?;
    let outcome = execute_wasm_with_hostcalls(
        &bundle.wasm,
        &bundle.input,
        &bundle.limits,
        OutputMode::Buffered,
    )?;
    let output = outcome.output.unwrap_or_default();

    Ok(ExecutionReport {
        bundle_hash,
        abi_version: bundle.v,
        runtime_id: bundle.runtime_id,
        output_hash: outcome.output_hash,
        output,
        input_len: bundle.input.len(),
        max_memory_bytes: bundle.limits.max_memory_bytes,
        max_instructions: bundle.limits.max_instructions,
        fuel_limit: bundle.limits.max_instructions,
        fuel_remaining: outcome.fuel_remaining,
    })
}

fn execute_bundle_payload_bytes_with_policy_digest_strict(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: Option<[u8; 32]>,
    expected_abi_version: Option<u8>,
) -> core::result::Result<ExecutionDigestReport, RuntimeError> {
    let (bundle_hash, bundle) = decode_validate_with_policy(
        bundle_payload_bytes,
        expected_runtime_id,
        expected_abi_version,
    )?;
    let outcome = execute_wasm_with_hostcalls(
        &bundle.wasm,
        &bundle.input,
        &bundle.limits,
        OutputMode::HashOnly,
    )?;

    Ok(ExecutionDigestReport {
        bundle_hash,
        abi_version: bundle.v,
        runtime_id: bundle.runtime_id,
        output_hash: outcome.output_hash,
        output_len: outcome.output_len,
        input_len: bundle.input.len(),
        max_memory_bytes: bundle.limits.max_memory_bytes,
        max_instructions: bundle.limits.max_instructions,
        fuel_limit: bundle.limits.max_instructions,
        fuel_remaining: outcome.fuel_remaining,
    })
}

fn decode_validate_with_policy(
    bundle_payload_bytes: &[u8],
    expected_runtime_id: Option<[u8; 32]>,
    expected_abi_version: Option<u8>,
) -> core::result::Result<([u8; 32], edgerun_types::BundlePayload), RuntimeError> {
    // Invariant: always hash raw canonical bytes before any decode/re-encode.
    let bundle_hash = edgerun_crypto::compute_bundle_hash(bundle_payload_bytes);
    let bundle = decode_bundle_from_canonical_bytes(bundle_payload_bytes)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::BundleDecode, e.to_string()))?;

    if let Some(expected_abi_version) = expected_abi_version {
        if bundle.v != expected_abi_version {
            return Err(RuntimeError::new(
                RuntimeErrorCode::AbiVersionMismatch,
                format!(
                    "bundle ABI version {} does not match expected ABI version {}",
                    bundle.v, expected_abi_version
                ),
            ));
        }
    }

    if let Some(expected_runtime_id) = expected_runtime_id {
        if bundle.runtime_id != expected_runtime_id {
            return Err(RuntimeError::new(
                RuntimeErrorCode::RuntimeIdMismatch,
                format!(
                    "bundle runtime_id {} does not match expected runtime_id {}",
                    hex::encode(bundle.runtime_id),
                    hex::encode(expected_runtime_id)
                ),
            ));
        }
    }

    validate_wasm_module(&bundle.wasm)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::ValidationFailed, e.to_string()))?;

    if bundle.limits.max_instructions == 0 {
        return Err(RuntimeError::new(
            RuntimeErrorCode::ValidationFailed,
            "max_instructions must be greater than zero",
        ));
    }

    Ok((bundle_hash, bundle))
}

fn execute_wasm_with_hostcalls(
    wasm: &[u8],
    input: &[u8],
    limits: &edgerun_types::Limits,
    output_mode: OutputMode,
) -> core::result::Result<ExecutionOutcome, RuntimeError> {
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);

    let module = Module::new(&engine, wasm)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::EngineCompile, e.to_string()))?;

    let store_limits = StoreLimitsBuilder::new()
        .memory_size(limits.max_memory_bytes as usize)
        .tables(0)
        .instances(1)
        .memories(1)
        .trap_on_grow_failure(true)
        .build();

    let store_data = RuntimeStoreData {
        host: RuntimeHostState {
            input: input.to_vec(),
            output: Vec::new(),
            output_len: 0,
            output_hasher: blake3::Hasher::new(),
            output_mode,
            output_write_calls: 0,
            max_output_bytes: limits.max_memory_bytes as usize,
            memory: None,
        },
        limits: store_limits,
    };

    let mut store = Store::new(&engine, store_data);
    store.limiter(|state| &mut state.limits);
    store
        .set_fuel(limits.max_instructions)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::FuelConfiguration, e.to_string()))?;

    let mut linker = Linker::<RuntimeStoreData>::new(&engine);
    linker
        .func_wrap("edgerun", "input_len", host_input_len)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::HostcallFailed, e.to_string()))?;
    linker
        .func_wrap("edgerun", "read_input", host_read_input)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::HostcallFailed, e.to_string()))?;
    linker
        .func_wrap("edgerun", "write_output", host_write_output)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::HostcallFailed, e.to_string()))?;

    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::InstantiationFailed, e.to_string()))?;

    let memory = instance.get_memory(&store, "memory").ok_or_else(|| {
        RuntimeError::new(
            RuntimeErrorCode::MissingMemoryExport,
            "module must export memory as \"memory\"",
        )
    })?;

    {
        let state = store.data_mut();
        state.host.memory = Some(memory);
    }

    if memory.data_size(&store) > limits.max_memory_bytes as usize {
        return Err(RuntimeError::new(
            RuntimeErrorCode::MemoryLimitExceeded,
            format!(
                "module memory size {} exceeds max_memory_bytes {}",
                memory.data_size(&store),
                limits.max_memory_bytes
            ),
        ));
    }

    let start: TypedFunc<(), ()> = instance
        .get_typed_func(&store, "_start")
        .map_err(|e| RuntimeError::new(RuntimeErrorCode::StartFunctionMissing, e.to_string()))?;

    if let Err(err) = start.call(&mut store, ()) {
        let fuel_remaining = store.get_fuel().ok();
        return Err(map_execution_error(
            err,
            limits.max_instructions,
            fuel_remaining,
        ));
    }

    let host = &store.data().host;
    let output = if host.output_mode == OutputMode::Buffered {
        Some(host.output.clone())
    } else {
        None
    };
    let output_len = host.output_len;
    let output_hash: [u8; 32] = host.output_hasher.clone().finalize().into();
    let fuel_after_exec = store.get_fuel().ok().unwrap_or(0);
    if output_len > limits.max_memory_bytes as usize {
        return Err(RuntimeError::new(
            RuntimeErrorCode::MemoryLimitExceeded,
            format!(
                "output size {} exceeds max_memory_bytes {}",
                output_len, limits.max_memory_bytes
            ),
        ));
    }
    if host.output_write_calls != 1 {
        return Err(RuntimeError::new(
            RuntimeErrorCode::OutputContractViolation,
            format!(
                "module must call write_output exactly once, observed {} calls",
                host.output_write_calls
            ),
        ));
    }

    Ok(ExecutionOutcome {
        output,
        output_hash,
        output_len,
        fuel_remaining: fuel_after_exec,
    })
}

fn host_input_len(caller: Caller<'_, RuntimeStoreData>) -> i32 {
    let len = caller.data().host.input.len();
    i32::try_from(len).unwrap_or(i32::MAX)
}

fn host_read_input(
    mut caller: Caller<'_, RuntimeStoreData>,
    dst_ptr: i32,
    input_offset: i32,
    len: i32,
) -> i32 {
    if dst_ptr < 0 || input_offset < 0 || len < 0 {
        return -1;
    }

    let Some(memory) = caller.data().host.memory else {
        return -1;
    };

    let input = caller.data().host.input.clone();
    let start = input_offset as usize;
    if start >= input.len() {
        return 0;
    }
    let req = len as usize;
    let copied = req.min(input.len() - start);
    let slice = &input[start..start + copied];

    if memory.write(&mut caller, dst_ptr as usize, slice).is_err() {
        return -1;
    }
    copied as i32
}

fn host_write_output(mut caller: Caller<'_, RuntimeStoreData>, src_ptr: i32, len: i32) -> i32 {
    if src_ptr < 0 || len < 0 {
        return -1;
    }

    let Some(memory) = caller.data().host.memory else {
        return -1;
    };

    let mut bytes = vec![0_u8; len as usize];
    if memory.read(&caller, src_ptr as usize, &mut bytes).is_err() {
        return -1;
    }

    let state = caller.data_mut();
    state.host.output_write_calls = state.host.output_write_calls.saturating_add(1);
    let next_len = state.host.output_len.saturating_add(bytes.len());
    if next_len > state.host.max_output_bytes {
        return -1;
    }
    state.host.output_len = next_len;
    state.host.output_hasher.update(&bytes);
    if state.host.output_mode == OutputMode::Buffered {
        state.host.output.extend_from_slice(&bytes);
    }
    bytes.len() as i32
}

fn map_execution_error(
    err: wasmi::Error,
    max_instructions: u64,
    fuel_remaining: Option<u64>,
) -> RuntimeError {
    if let Some(code) = err.as_trap_code() {
        return match code {
            TrapCode::OutOfFuel => RuntimeError::new(
                RuntimeErrorCode::InstructionLimitExceeded,
                format!("execution exceeded max_instructions {max_instructions}"),
            )
            .with_trap_code("OutOfFuel")
            .with_fuel(max_instructions, fuel_remaining),
            TrapCode::GrowthOperationLimited => RuntimeError::new(
                RuntimeErrorCode::MemoryLimitExceeded,
                "memory/table growth exceeded runtime limits",
            )
            .with_trap_code("GrowthOperationLimited")
            .with_fuel(max_instructions, fuel_remaining),
            _ => RuntimeError::new(RuntimeErrorCode::Trap, format!("wasm trap: {code:?}"))
                .with_trap_code(format!("{code:?}"))
                .with_fuel(max_instructions, fuel_remaining),
        };
    }
    RuntimeError::new(
        RuntimeErrorCode::Trap,
        format!("wasm execution failed: {err}"),
    )
    .with_fuel(max_instructions, fuel_remaining)
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;

    fn sample_payload_with_runtime_and_input(
        wasm: Vec<u8>,
        abi_version: u8,
        runtime_id: [u8; 32],
        input: Vec<u8>,
        max_memory_bytes: u32,
        max_instructions: u64,
    ) -> Vec<u8> {
        let payload = edgerun_types::BundlePayload {
            v: abi_version,
            runtime_id,
            wasm,
            input,
            limits: edgerun_types::Limits {
                max_memory_bytes,
                max_instructions,
            },
            meta: None,
        };
        edgerun_types::encode_bundle_payload_canonical(&payload).expect("canonical encode")
    }

    fn sample_payload(wasm: Vec<u8>, max_memory_bytes: u32, max_instructions: u64) -> Vec<u8> {
        sample_payload_with_runtime_and_input(
            wasm,
            edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
            [9_u8; 32],
            b"hello-edgerun".to_vec(),
            max_memory_bytes,
            max_instructions,
        )
    }

    #[test]
    fn deterministic_execution_is_stable() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (func $input_len (result i32)))
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $read_input (i32.const 0) (i32.const 0) (call $input_len)))
                    (drop (call $write_output (i32.const 0) (call $input_len)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let a = execute_bundle_payload_bytes(&bytes).expect("exec a");
        let b = execute_bundle_payload_bytes(&bytes).expect("exec b");
        assert_eq!(a.bundle_hash, b.bundle_hash);
        assert_eq!(a.output_hash, b.output_hash);
        assert_eq!(a.output, b.output);
        assert_eq!(a.output, b"hello-edgerun".to_vec());
        assert_eq!(a.abi_version, 1);
        assert_eq!(a.runtime_id, [9_u8; 32]);
        assert_eq!(a.fuel_limit, 50_000);
        assert!(a.fuel_remaining <= a.fuel_limit);
    }

    #[test]
    fn enforces_instruction_limit() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (loop
                        br 0
                    )
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 1_000);
        let err = execute_bundle_payload_bytes(&bytes).expect_err("must fail with out-of-fuel");
        assert!(err
            .to_string()
            .contains("execution exceeded max_instructions"));

        let strict = execute_bundle_payload_bytes_strict(&bytes).expect_err("strict must fail");
        assert_eq!(strict.code, RuntimeErrorCode::InstructionLimitExceeded);
        assert_eq!(strict.trap_code.as_deref(), Some("OutOfFuel"));
        assert_eq!(strict.fuel_limit, Some(1_000));
        assert!(strict.fuel_remaining.is_some());
    }

    #[test]
    fn enforces_output_memory_limit() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (data (i32.const 0) "abcdef")
                (func (export "_start")
                    (drop (call $write_output (i32.const 0) (i32.const 6)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 4, 50_000);
        let err = execute_bundle_payload_bytes(&bytes).expect_err("must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("memory")
                || msg.contains("output")
                || msg.contains("max_memory_bytes")
                || msg.contains("limit")
                || msg.contains("instantiate"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn strict_api_maps_validation_errors() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start") (result i32)
                    i32.const 1
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let err = execute_bundle_payload_bytes_strict(&bytes).expect_err("must fail");
        assert_eq!(err.code, RuntimeErrorCode::ValidationFailed);
    }

    #[test]
    fn strict_api_rejects_runtime_id_mismatch() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload_with_runtime_and_input(
            wasm,
            edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
            [9_u8; 32],
            b"hello-edgerun".to_vec(),
            1024 * 1024,
            50_000,
        );
        let err = execute_bundle_payload_bytes_for_runtime_strict(&bytes, [7_u8; 32])
            .expect_err("must fail runtime_id mismatch");
        assert_eq!(err.code, RuntimeErrorCode::RuntimeIdMismatch);
    }

    #[test]
    fn strict_api_rejects_abi_version_mismatch() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload_with_runtime_and_input(
            wasm,
            edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
            [9_u8; 32],
            b"hello-edgerun".to_vec(),
            1024 * 1024,
            50_000,
        );
        let err = execute_bundle_payload_bytes_for_runtime_and_abi_strict(&bytes, [9_u8; 32], 2)
            .expect_err("must fail ABI version mismatch");
        assert_eq!(err.code, RuntimeErrorCode::AbiVersionMismatch);
    }

    #[test]
    fn strict_api_accepts_current_and_n_minus_one_abi_versions() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $write_output (i32.const 0) (i32.const 0)))
                )
            )"#,
        )
        .expect("wat parse");

        let n_minus_one = edgerun_types::BUNDLE_ABI_MIN_SUPPORTED;
        let current = edgerun_types::BUNDLE_ABI_CURRENT;
        let runtime_id = [9_u8; 32];

        let bytes_n_minus_one = sample_payload_with_runtime_and_input(
            wasm.clone(),
            n_minus_one,
            runtime_id,
            b"hello-edgerun".to_vec(),
            1024 * 1024,
            50_000,
        );
        let report_n_minus_one = execute_bundle_payload_bytes_for_runtime_and_abi_strict(
            &bytes_n_minus_one,
            runtime_id,
            n_minus_one,
        )
        .expect("n-1 ABI should be accepted");
        assert_eq!(report_n_minus_one.abi_version, n_minus_one);

        let bytes_current = sample_payload_with_runtime_and_input(
            wasm,
            current,
            runtime_id,
            b"hello-edgerun".to_vec(),
            1024 * 1024,
            50_000,
        );
        let report_current = execute_bundle_payload_bytes_for_runtime_and_abi_strict(
            &bytes_current,
            runtime_id,
            current,
        )
        .expect("current ABI should be accepted");
        assert_eq!(report_current.abi_version, current);
    }

    #[test]
    fn strict_api_rejects_unsupported_abi_versions() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $write_output (i32.const 0) (i32.const 0)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes_unsupported = sample_payload_with_runtime_and_input(
            wasm,
            edgerun_types::BUNDLE_ABI_CURRENT.saturating_add(1),
            [9_u8; 32],
            b"hello-edgerun".to_vec(),
            1024 * 1024,
            50_000,
        );

        let err = execute_bundle_payload_bytes_strict(&bytes_unsupported)
            .expect_err("unsupported ABI version must fail decode");
        assert_eq!(err.code, RuntimeErrorCode::BundleDecode);
    }

    #[test]
    fn digest_mode_matches_buffered_output_hash_and_length() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (func $input_len (result i32)))
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $read_input (i32.const 0) (i32.const 0) (call $input_len)))
                    (drop (call $write_output (i32.const 0) (call $input_len)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let buffered = execute_bundle_payload_bytes_for_runtime_and_abi_strict(
            &bytes,
            [9_u8; 32],
            edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
        )
        .expect("buffered exec");
        let digest = execute_bundle_payload_bytes_for_runtime_and_abi_digest_strict(
            &bytes,
            [9_u8; 32],
            edgerun_types::BUNDLE_ABI_MIN_SUPPORTED,
        )
        .expect("digest exec");

        assert_eq!(digest.output_hash, buffered.output_hash);
        assert_eq!(digest.output_len, buffered.output.len());
    }

    #[test]
    fn enforces_single_write_output_contract() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $write_output (i32.const 0) (i32.const 0)))
                    (drop (call $write_output (i32.const 0) (i32.const 0)))
                )
            )"#,
        )
        .expect("wat parse");
        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let err = execute_bundle_payload_bytes_strict(&bytes).expect_err("must fail");
        assert_eq!(err.code, RuntimeErrorCode::OutputContractViolation);
    }

    #[test]
    fn rejects_missing_write_output_contract() {
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1 1)
                (func (export "_start"))
            )"#,
        )
        .expect("wat parse");
        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let err = execute_bundle_payload_bytes_strict(&bytes).expect_err("must fail");
        assert_eq!(err.code, RuntimeErrorCode::OutputContractViolation);
    }

    #[test]
    fn hostcalls_allow_zero_length_read_and_write() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (drop (call $read_input (i32.const 0) (i32.const 0) (i32.const 0)))
                    (drop (call $write_output (i32.const 0) (i32.const 0)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let report = execute_bundle_payload_bytes(&bytes).expect("exec");
        assert!(report.output.is_empty());
    }

    #[test]
    fn hostcalls_partial_read_near_end_of_input() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (func $input_len (result i32)))
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (local $n i32)
                    (local $copied i32)
                    (local.set $n (call $input_len))
                    (local.set $copied
                        (call $read_input
                            (i32.const 0)
                            (i32.sub (local.get $n) (i32.const 3))
                            (i32.const 10)))
                    (drop (call $write_output (i32.const 0) (local.get $copied)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let report = execute_bundle_payload_bytes(&bytes).expect("exec");
        assert_eq!(report.output, b"run");
    }

    #[test]
    fn hostcalls_allow_maximum_legal_write_at_memory_boundary() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "input_len" (func $input_len (result i32)))
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (local $n i32)
                    (local $copied i32)
                    (local.set $n (call $input_len))
                    (local.set $copied
                        (call $read_input
                            (i32.const 65535)
                            (i32.sub (local.get $n) (i32.const 1))
                            (i32.const 1)))
                    (drop (call $write_output (i32.const 65535) (local.get $copied)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let report = execute_bundle_payload_bytes(&bytes).expect("exec");
        assert_eq!(report.output, b"n");
    }

    #[test]
    fn hostcalls_reject_negative_and_overflow_pointers() {
        let wasm = wat::parse_str(
            r#"(module
                (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
                (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
                (memory (export "memory") 1 1)
                (func (export "_start")
                    (local $read_rc i32)
                    (local $write_rc i32)
                    (local.set $read_rc (call $read_input (i32.const -1) (i32.const 0) (i32.const 1)))
                    (local.set $write_rc (call $write_output (i32.const 2147483647) (i32.const 1)))
                    (i32.store8 (i32.const 0) (local.get $read_rc))
                    (i32.store8 (i32.const 1) (local.get $write_rc))
                    (drop (call $write_output (i32.const 0) (i32.const 2)))
                )
            )"#,
        )
        .expect("wat parse");

        let bytes = sample_payload(wasm, 1024 * 1024, 50_000);
        let report = execute_bundle_payload_bytes(&bytes).expect("exec");
        assert_eq!(report.output, vec![0xff, 0xff]);
    }
}

#[cfg(all(test, miri))]
mod miri_tests {
    use super::*;

    #[test]
    fn rejects_non_bundle_bytes_under_miri() {
        let err = execute_bundle_payload_bytes_strict(b"not-a-canonical-bundle")
            .expect_err("invalid bundle bytes must fail");
        assert_eq!(err.code, RuntimeErrorCode::BundleDecode);
    }
}
