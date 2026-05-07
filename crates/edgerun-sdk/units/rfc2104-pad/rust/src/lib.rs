#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(2104);

#[edgerun_unit::export]
unsafe fn rfc2104_key_pad(
    key_ptr: i32,
    key_len: i32,
    block_size: i32,
    ipad_out: i32,
    opad_out: i32,
) -> i32 {
    if key_ptr < 0 || key_len < 0 || ipad_out < 0 || opad_out < 0 || block_size <= 0 {
        return 1;
    }
    if block_size as usize > 128 {
        return 2;
    }
    let key = core::slice::from_raw_parts(key_ptr as *const u8, key_len as usize);
    let block = block_size as usize;
    let mut normalized = [0u8; 128];
    if key.len() > block {
        if block != 64 {
            return 3;
        }
        let digest = edgerun_crypto::sha256(key);
        normalized[..digest.len()].copy_from_slice(&digest);
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let ipad = core::slice::from_raw_parts_mut(ipad_out as *mut u8, block);
    let opad = core::slice::from_raw_parts_mut(opad_out as *mut u8, block);
    let mut index = 0;
    while index < block {
        ipad[index] = normalized[index] ^ 0x36;
        opad[index] = normalized[index] ^ 0x5c;
        index += 1;
    }
    0
}
