#![no_std]

use core::ptr::{addr_of, addr_of_mut, copy_nonoverlapping};

#[path = "../src/blake3.rs"]
#[allow(dead_code)]
mod blake3;

const INPUT_LEN: usize = 1024;
const OUTPUT_LEN: usize = 32;

static mut INPUT: [u8; INPUT_LEN] = [0; INPUT_LEN];
static mut OUTPUT: [u8; OUTPUT_LEN] = [0; OUTPUT_LEN];

#[unsafe(no_mangle)]
pub extern "C" fn input_ptr() -> *mut u8 {
    addr_of_mut!(INPUT).cast::<u8>()
}

#[unsafe(no_mangle)]
pub extern "C" fn input_len() -> usize {
    INPUT_LEN
}

#[unsafe(no_mangle)]
pub extern "C" fn output_ptr() -> *const u8 {
    addr_of!(OUTPUT).cast::<u8>()
}

#[unsafe(no_mangle)]
pub extern "C" fn output_len() -> usize {
    OUTPUT_LEN
}

#[unsafe(no_mangle)]
pub extern "C" fn hash_1024() -> usize {
    let input = unsafe { core::slice::from_raw_parts(addr_of!(INPUT).cast::<u8>(), INPUT_LEN) };
    let hash = blake3::hash(input);
    unsafe {
        copy_nonoverlapping(
            hash.as_bytes().as_ptr(),
            addr_of_mut!(OUTPUT).cast::<u8>(),
            OUTPUT_LEN,
        );
    }
    OUTPUT_LEN
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
