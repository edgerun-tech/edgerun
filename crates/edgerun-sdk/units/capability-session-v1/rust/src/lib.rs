#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
fn capability_session_mode_valid(mode: i32) -> i32 {
    (0..=2).contains(&mode) as i32
}

#[edgerun_unit::export]
fn capability_session_open_validate(
    version: i32,
    session_id_len: i32,
    mode: i32,
    requested_ops_count: i32,
    requested_access_class: i32,
) -> i32 {
    if version != 1 {
        return 1;
    }
    if session_id_len <= 0 || session_id_len > 64 {
        return 2;
    }
    if capability_session_mode_valid(mode) == 0 {
        return 3;
    }
    if requested_ops_count <= 0 || requested_ops_count > 64 {
        return 4;
    }
    if !(0..=3).contains(&requested_access_class) {
        return 5;
    }
    0
}

#[edgerun_unit::export]
fn capability_session_accept_unchecked_status(open_status: i32) -> i32 {
    if open_status == 0 {
        0
    } else {
        1
    }
}

#[edgerun_unit::export]
fn capability_session_reject_status() -> i32 {
    1
}
