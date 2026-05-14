#![no_std]

use edgerun_crypto::P256SigningKey;

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

const PRIVATE_KEY_LEN: usize = 32;
const PUBLIC_KEY_LEN: usize = 64;

#[edgerun_unit::export]
fn edgerun_p256_private_key_len() -> i32 {
    PRIVATE_KEY_LEN as i32
}

#[edgerun_unit::export]
fn edgerun_p256_public_key_len() -> i32 {
    PUBLIC_KEY_LEN as i32
}

#[edgerun_unit::export]
unsafe fn edgerun_p256_private_key_valid(private_ptr: i32, private_len: i32) -> i32 {
    if private_ptr < 0 || private_len < 0 {
        return 0;
    }
    if private_len as usize != PRIVATE_KEY_LEN {
        return 0;
    }
    let private = core::slice::from_raw_parts(private_ptr as *const u8, PRIVATE_KEY_LEN);
    let Ok(private) = <[u8; PRIVATE_KEY_LEN]>::try_from(private) else {
        return 0;
    };
    P256SigningKey::from_bytes(&private).is_ok() as i32
}

#[edgerun_unit::export]
unsafe fn edgerun_p256_public_key_from_private(
    private_ptr: i32,
    private_len: i32,
    out_ptr: i32,
) -> i32 {
    if private_ptr < 0 || private_len < 0 || out_ptr < 0 {
        return 1;
    }
    if private_len as usize != PRIVATE_KEY_LEN {
        return 2;
    }
    let private = core::slice::from_raw_parts(private_ptr as *const u8, PRIVATE_KEY_LEN);
    let Ok(private) = <[u8; PRIVATE_KEY_LEN]>::try_from(private) else {
        return 3;
    };
    let Ok(signing_key) = P256SigningKey::from_bytes(&private) else {
        return 3;
    };
    let public_key = signing_key.public_key_sec1();
    core::ptr::copy_nonoverlapping(
        public_key.as_ptr().add(1),
        out_ptr as *mut u8,
        PUBLIC_KEY_LEN,
    );
    0
}
