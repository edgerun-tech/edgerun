#![no_std]

edgerun_unit::no_alloc!();

const FRAME_LEN: usize = 1024;

static mut BUFFER: [u8; FRAME_LEN] = [0; FRAME_LEN];
static mut COUNT: u16 = 0;
static mut FULL: u8 = 0;

#[unsafe(no_mangle)]
pub extern "C" fn proto_abi_version() -> i32 {
    edgerun_unit::ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn proto_standard_id() -> i32 {
    41024
}

#[unsafe(no_mangle)]
pub extern "C" fn frame_len() -> i32 {
    FRAME_LEN as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn count() -> i32 {
    COUNT as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_full() -> i32 {
    FULL as i32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn reset() -> i32 {
    COUNT = 0;
    FULL = 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn append(input: i32) -> i32 {
    if FULL != 0 {
        return 1;
    }
    *core::ptr::addr_of_mut!(BUFFER).cast::<u8>().add(COUNT as usize) = (input & 0xff) as u8;
    COUNT += 1;
    if COUNT as usize == FRAME_LEN {
        FULL = 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flush(out_ptr: i32, out_len: i32) -> i32 {
    if out_ptr < 0 || out_len as usize != FRAME_LEN || FULL == 0 {
        return 1;
    }
    core::ptr::copy_nonoverlapping(
        core::ptr::addr_of!(BUFFER).cast::<u8>(),
        out_ptr as *mut u8,
        FRAME_LEN,
    );
    COUNT = 0;
    FULL = 0;
    0
}
