#![no_main]

use arbitrary::Arbitrary;
use edgerun_types::{BundlePayload, Limits};
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
struct HostcallCase {
    dst_ptr: i32,
    input_offset: i32,
    read_len: i32,
    src_ptr: i32,
    write_len: i32,
    input: Vec<u8>,
    max_memory_pages: u8,
    max_instructions: u32,
}

fn module_for_case(case: &HostcallCase) -> String {
    format!(
        r#"(module
            (import "edgerun" "read_input" (func $read_input (param i32 i32 i32) (result i32)))
            (import "edgerun" "write_output" (func $write_output (param i32 i32) (result i32)))
            (memory (export "memory") 1 1)
            (func (export "_start")
                (local $r i32)
                (local $w i32)
                (local.set $r (call $read_input (i32.const {dst_ptr}) (i32.const {input_offset}) (i32.const {read_len})))
                (local.set $w (call $write_output (i32.const {src_ptr}) (i32.const {write_len})))
                (i32.store8 (i32.const 0) (local.get $r))
                (i32.store8 (i32.const 1) (local.get $w))
                (drop (call $write_output (i32.const 0) (i32.const 2)))
            )
        )"#,
        dst_ptr = case.dst_ptr,
        input_offset = case.input_offset,
        read_len = case.read_len,
        src_ptr = case.src_ptr,
        write_len = case.write_len,
    )
}

fuzz_target!(|case: HostcallCase| {
    let wasm = match wat::parse_str(module_for_case(&case)) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    let input = case.input.into_iter().take(512).collect::<Vec<_>>();
    let max_pages = u32::from((case.max_memory_pages % 16).max(1));
    let max_instructions = u64::from(case.max_instructions).max(1000);

    let payload = BundlePayload {
        v: 1,
        runtime_id: [9_u8; 32],
        wasm,
        input,
        limits: Limits {
            max_memory_bytes: max_pages * 65_536,
            max_instructions,
        },
        meta: None,
    };

    let bytes = match edgerun_types::encode_bundle_payload_canonical(&payload) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    let _ = edgerun_runtime::execute_bundle_payload_bytes_strict(&bytes);
});
