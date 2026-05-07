#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
fn capability_operation_valid(operation: i32) -> i32 {
    (0..=63).contains(&operation) as i32
}

#[edgerun_unit::export]
fn capability_access_class_valid(access_class: i32) -> i32 {
    (0..=3).contains(&access_class) as i32
}

#[edgerun_unit::export]
unsafe fn capability_authorize_invocation(
    granted_ops_ptr: i32,
    granted_ops_count: i32,
    operation: i32,
    requested_access_class: i32,
    is_local: i32,
    grant_revoked: i32,
) -> i32 {
    if granted_ops_ptr < 0 || granted_ops_count < 0 {
        return 1;
    }
    if grant_revoked != 0 {
        return 2;
    }
    if capability_operation_valid(operation) == 0 || capability_access_class_valid(requested_access_class) == 0 {
        return 3;
    }
    if requested_access_class == 3 && is_local == 0 {
        return 4;
    }
    let granted = core::slice::from_raw_parts(granted_ops_ptr as *const u32, granted_ops_count as usize);
    let mut index = 0;
    while index < granted.len() {
        if granted[index] == operation as u32 {
            return 0;
        }
        index += 1;
    }
    5
}
