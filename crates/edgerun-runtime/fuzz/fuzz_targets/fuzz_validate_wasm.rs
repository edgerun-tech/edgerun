#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = edgerun_runtime::validate_wasm_module(data);
});
