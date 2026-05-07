#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(2104);

#[edgerun_unit::export]
unsafe fn hmac_sha256(
    key_ptr: i32,
    key_len: i32,
    data_ptr: i32,
    data_len: i32,
    out_ptr: i32,
) -> i32 {
    if key_ptr < 0 || key_len < 0 || data_ptr < 0 || data_len < 0 || out_ptr < 0 {
        return 1;
    }
    let key = core::slice::from_raw_parts(key_ptr as *const u8, key_len as usize);
    let data = core::slice::from_raw_parts(data_ptr as *const u8, data_len as usize);
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        let digest = edgerun_crypto::sha256(key);
        normalized[..digest.len()].copy_from_slice(&digest);
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    let mut index = 0;
    while index < 64 {
        ipad[index] ^= normalized[index];
        opad[index] ^= normalized[index];
        index += 1;
    }
    let mut inner_input = [0u8; 64 + 4096];
    if data.len() > 4096 {
        return 2;
    }
    inner_input[..64].copy_from_slice(&ipad);
    inner_input[64..64 + data.len()].copy_from_slice(data);
    let inner_digest = edgerun_crypto::sha256(&inner_input[..64 + data.len()]);
    let mut outer_input = [0u8; 96];
    outer_input[..64].copy_from_slice(&opad);
    outer_input[64..].copy_from_slice(&inner_digest);
    let digest = edgerun_crypto::sha256(&outer_input);
    core::ptr::copy_nonoverlapping(digest.as_ptr(), out_ptr as *mut u8, 32);
    0
}
