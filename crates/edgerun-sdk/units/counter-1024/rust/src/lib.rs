#![no_std]

edgerun_unit::no_alloc!();

const COUNTER_LIMIT: u16 = 1024;

static mut COUNT: u16 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn proto_abi_version() -> i32 {
    edgerun_unit::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn proto_standard_id() -> i32 {
    21024
}

#[unsafe(no_mangle)]
pub extern "C" fn counter_limit() -> i32 {
    COUNTER_LIMIT as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset() -> i32 {
    COUNT = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn count() -> i32 {
    COUNT as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pass(input: i32) -> i32 {
    COUNT = (COUNT + 1) % COUNTER_LIMIT;
    input & 0xff
}
