#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
fn capability_invocation_validate(
    version: i32,
    invocation_id_len: i32,
    grant_id_len: i32,
    operation: i32,
    requested_access_class: i32,
) -> i32 {
    if version != 1 {
        return 1;
    }
    if invocation_id_len <= 0 || invocation_id_len > 64 || grant_id_len <= 0 || grant_id_len > 64 {
        return 2;
    }
    if !(0..=63).contains(&operation) {
        return 3;
    }
    if !(0..=3).contains(&requested_access_class) {
        return 4;
    }
    0
}

#[edgerun_unit::export]
fn capability_result_validate(
    version: i32,
    invocation_id_len: i32,
    grant_id_len: i32,
    result_access_class: i32,
) -> i32 {
    if version != 1 {
        return 1;
    }
    if invocation_id_len <= 0 || invocation_id_len > 64 || grant_id_len <= 0 || grant_id_len > 64 {
        return 2;
    }
    if !(0..=3).contains(&result_access_class) {
        return 3;
    }
    0
}

#[edgerun_unit::export]
fn capability_result_frame_validate(result_present: i32, inline_payload_len: i32) -> i32 {
    if result_present != 0 && result_present != 1 {
        return 1;
    }
    if inline_payload_len < 0 || inline_payload_len > 4096 {
        return 2;
    }
    if result_present == 0 && inline_payload_len != 0 {
        return 3;
    }
    0
}
