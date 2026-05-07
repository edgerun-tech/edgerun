#![no_std]

edgerun_unit::no_alloc!();
edgerun_unit::metadata!(1);

#[edgerun_unit::export]
unsafe fn decision_byte_eq(input_ptr: i32, expected_ptr: i32, out_ptr: i32) -> i32 {
    if input_ptr < 0 || expected_ptr < 0 || out_ptr < 0 {
        return 1;
    }
    let input = (input_ptr as *const u8).read();
    let expected = (expected_ptr as *const u8).read();
    (out_ptr as *mut u8).write((input != expected) as u8);
    0
}
