#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

const PUBLIC_KEY_LEN: usize = 64;
const SIGNATURE_LEN: usize = 64;

#[edgerun_unit::export]
fn edgerun_p256_public_key_len() -> i32 {
    PUBLIC_KEY_LEN as i32
}

#[edgerun_unit::export]
fn edgerun_p256_signature_len() -> i32 {
    SIGNATURE_LEN as i32
}

#[edgerun_unit::export]
unsafe fn edgerun_p256_raw64_public_key_valid(public_key_ptr: i32, public_key_len: i32) -> i32 {
    if public_key_ptr < 0 || public_key_len < 0 {
        return 0;
    }
    if public_key_len as usize != PUBLIC_KEY_LEN {
        return 0;
    }
    let public_key = core::slice::from_raw_parts(public_key_ptr as *const u8, PUBLIC_KEY_LEN);
    sec1_from_raw64(public_key)
        .and_then(|sec1| edgerun_crypto::P256VerifyingKey::from_sec1_bytes(&sec1).ok())
        .is_some() as i32
}

#[edgerun_unit::export]
unsafe fn edgerun_p256_verify_prehash_input(
    public_key_ptr: i32,
    input_ptr: i32,
    input_len: i32,
    signature_ptr: i32,
) -> i32 {
    if public_key_ptr < 0 || input_ptr < 0 || input_len < 0 || signature_ptr < 0 {
        return 0;
    }
    let public_key = core::slice::from_raw_parts(public_key_ptr as *const u8, PUBLIC_KEY_LEN);
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let signature = core::slice::from_raw_parts(signature_ptr as *const u8, SIGNATURE_LEN);
    let Some(public_key_sec1) = sec1_from_raw64(public_key) else {
        return 0;
    };
    edgerun_crypto::verification::p256_verify_prehash_fixed(&public_key_sec1, input, signature)
        .is_ok() as i32
}

fn sec1_from_raw64(public_key: &[u8]) -> Option<[u8; 65]> {
    if public_key.len() != PUBLIC_KEY_LEN {
        return None;
    }
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(public_key);
    Some(sec1)
}
