#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = edgerun_runtime::decode_bundle_from_canonical_bytes(data);
});
