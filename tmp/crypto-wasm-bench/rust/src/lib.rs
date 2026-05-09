#[no_mangle]
pub extern "C" fn edgerun_sha256(input_ptr: *const u8, input_len: usize, out_ptr: *mut u8) -> i32 {
    if input_ptr.is_null() || out_ptr.is_null() {
        return 1;
    }

    let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };
    let digest = edgerun_crypto::sha256(input);
    unsafe {
        core::ptr::copy_nonoverlapping(digest.as_ptr(), out_ptr, digest.len());
    }
    0
}

#[no_mangle]
pub extern "C" fn edgerun_hmac_sha256(
    key_ptr: *const u8,
    key_len: usize,
    input_ptr: *const u8,
    input_len: usize,
    out_ptr: *mut u8,
) -> i32 {
    if key_ptr.is_null() || input_ptr.is_null() || out_ptr.is_null() {
        return 1;
    }

    let key = unsafe { core::slice::from_raw_parts(key_ptr, key_len) };
    let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };
    let mac = edgerun_crypto::hmac_sha256(key, input);
    if mac.len() != 32 {
        return 2;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(mac.as_ptr(), out_ptr, mac.len());
    }
    0
}

#[no_mangle]
pub extern "C" fn edgerun_p256_sign_prehash(
    key_ptr: *const u8,
    digest_ptr: *const u8,
    out_ptr: *mut u8,
) -> i32 {
    if key_ptr.is_null() || digest_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }

    let key_bytes = unsafe { core::slice::from_raw_parts(key_ptr, 32) };
    let digest = unsafe { core::slice::from_raw_parts(digest_ptr, 32) };
    let Ok(signing_key) = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(key_bytes.into())
    else {
        return -2;
    };

    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    let Ok(signature): Result<edgerun_crypto::p256::ecdsa::Signature, _> =
        signing_key.sign_prehash(digest)
    else {
        return -3;
    };

    let der = signature.to_der();
    let bytes = der.as_bytes();
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    }
    bytes.len() as i32
}

#[no_mangle]
pub extern "C" fn edgerun_ed25519_sign(
    key_ptr: *const u8,
    input_ptr: *const u8,
    input_len: usize,
    out_ptr: *mut u8,
) -> i32 {
    if key_ptr.is_null() || input_ptr.is_null() || out_ptr.is_null() {
        return -1;
    }

    let key = unsafe { &*(key_ptr as *const [u8; 32]) };
    let input = unsafe { core::slice::from_raw_parts(input_ptr, input_len) };
    let signing_key = edgerun_crypto::ed25519_dalek::SigningKey::from_bytes(key);
    use edgerun_crypto::Signer;
    let signature = signing_key.sign(input);
    let bytes = signature.to_bytes();
    unsafe {
        core::ptr::copy_nonoverlapping(bytes.as_ptr(), out_ptr, bytes.len());
    }
    bytes.len() as i32
}
