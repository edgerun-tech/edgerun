#![no_std]

use core::panic::PanicInfo;

#[link(wasm_import_module = "edgerun_host")]
extern "C" {
    fn log(ptr: *const u8, len: usize);
    fn capability_request(ptr: *const u8, len: usize) -> i32;
}

const STARTED: &[u8] = b"edgerun dash wasm app started";
const CAPABILITY_REQUEST: &[u8] =
    br#"{"kind":"capability_request","version":"v0","source":"edgerun-dash-apps"}"#;

#[no_mangle]
pub extern "C" fn edgerun_app_start() {
    unsafe {
        log(STARTED.as_ptr(), STARTED.len());
        let _ = capability_request(CAPABILITY_REQUEST.as_ptr(), CAPABILITY_REQUEST.len());
    }
}

#[no_mangle]
pub extern "C" fn edgerun_app_abi_version() -> u32 {
    1
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
