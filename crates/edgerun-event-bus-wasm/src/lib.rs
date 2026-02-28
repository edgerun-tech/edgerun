// SPDX-License-Identifier: Apache-2.0

static mut EVENT_SEQ: u32 = 1;

#[no_mangle]
pub extern "C" fn bus_reset() {
    // SAFETY: this module is intended to run in a single worker thread.
    unsafe {
        EVENT_SEQ = 1;
    }
}

#[no_mangle]
pub extern "C" fn bus_next_seq() -> u32 {
    // SAFETY: this module is intended to run in a single worker thread.
    unsafe {
        let current = EVENT_SEQ;
        EVENT_SEQ = EVENT_SEQ.wrapping_add(1).max(1);
        current
    }
}

#[no_mangle]
pub extern "C" fn bus_event_code(topic_hash: u32, ts_unix_ms_low: u32) -> u32 {
    let seq = bus_next_seq();
    topic_hash.rotate_left(5) ^ ts_unix_ms_low.rotate_right(3) ^ seq
}
