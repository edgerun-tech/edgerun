#![no_std]

use edgerun_crypto::P256SigningKey;

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

const PRIVATE_KEY_LEN: usize = 32;
const SIGNATURE_LEN: usize = 64;

#[edgerun_unit::export]
fn edgerun_signature_algorithm_p256_sha256() -> i32 {
    1
}

#[edgerun_unit::export]
fn edgerun_signature_len_p256() -> i32 {
    SIGNATURE_LEN as i32
}

#[edgerun_unit::export]
fn edgerun_signature_input_len(domain_len: i32, record_hash_len: i32) -> i32 {
    if domain_len < 0 || record_hash_len < 0 {
        return -1;
    }
    domain_len.saturating_add(1).saturating_add(record_hash_len)
}

#[edgerun_unit::export]
unsafe fn edgerun_signature_input(
    domain_ptr: i32,
    domain_len: i32,
    record_hash_ptr: i32,
    record_hash_len: i32,
    out_ptr: i32,
) -> i32 {
    if domain_ptr < 0 || domain_len < 0 || record_hash_ptr < 0 || record_hash_len < 0 || out_ptr < 0
    {
        return 1;
    }
    let domain = core::slice::from_raw_parts(domain_ptr as *const u8, domain_len as usize);
    let record_hash =
        core::slice::from_raw_parts(record_hash_ptr as *const u8, record_hash_len as usize);
    let out = out_ptr as *mut u8;
    core::ptr::copy_nonoverlapping(domain.as_ptr(), out, domain.len());
    out.add(domain.len()).write(0);
    core::ptr::copy_nonoverlapping(
        record_hash.as_ptr(),
        out.add(domain.len() + 1),
        record_hash.len(),
    );
    0
}

#[edgerun_unit::export]
unsafe fn edgerun_p256_sign_prehash_input(
    private_ptr: i32,
    private_len: i32,
    input_ptr: i32,
    input_len: i32,
    out_ptr: i32,
) -> i32 {
    if private_ptr < 0 || private_len < 0 || input_ptr < 0 || input_len < 0 || out_ptr < 0 {
        return 1;
    }
    if private_len as usize != PRIVATE_KEY_LEN {
        return 2;
    }
    let private = core::slice::from_raw_parts(private_ptr as *const u8, PRIVATE_KEY_LEN);
    let input = core::slice::from_raw_parts(input_ptr as *const u8, input_len as usize);
    let Ok(private) = <[u8; PRIVATE_KEY_LEN]>::try_from(private) else {
        return 3;
    };
    let Ok(signing_key) = P256SigningKey::from_bytes(&private) else {
        return 3;
    };
    let Ok(signature) = signing_key.sign_prehash_fixed(input) else {
        return 4;
    };
    core::ptr::copy_nonoverlapping(signature.as_ptr(), out_ptr as *mut u8, SIGNATURE_LEN);
    0
}
