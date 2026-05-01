#![no_std]
#![no_main]
#![allow(static_mut_refs)]

extern crate alloc;

use core::alloc::{GlobalAlloc, Layout};

const HEAP_SIZE: usize = 256 * 1024;
static mut HEAP: [u8; HEAP_SIZE] = [0u8; HEAP_SIZE];
static mut HEAP_POS: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        use core::sync::atomic::Ordering;
        let align = layout.align();
        let size = layout.size();
        loop {
            let pos = HEAP_POS.load(Ordering::Relaxed);
            let aligned = (pos + align - 1) & !(align - 1);
            let end = aligned + size;
            if end > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            if HEAP_POS
                .compare_exchange(pos, end, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return HEAP.as_mut_ptr().add(aligned);
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use edgerun_sdk::{EventType, Request, Response};
use edgerun_sdk::host::{poll_event_safe, write_output};

const HTML: &str = r#"<!DOCTYPE html>
<html>
<head><title>EdgeRun</title></head>
<body>
<h1>Hello from EdgeRun</h1>
<p>This is a static website served from WASM.</p>
</body>
</html>"#;

fn handle_events() {
    let mut buf = [0u8; 1024];
    let mut event_count = 0u32;
    loop {
        match poll_event_safe(&mut buf) {
            Some(event) => {
                event_count += 1;
                match event.event_type {
                    EventType::Network => {
                        if let Some(sock_id) = event.network_sock_id() {
                            match event.network_subtype() {
                                Some(edgerun_sdk::NetworkSubtype::Received) => {
                                    if let Some(payload) = event.network_payload() {
                                        let resp = Response::ok("network recv");
                                        let bytes = resp.to_bytes();
                                        unsafe {
                                            write_output(bytes.as_ptr() as i32, bytes.len() as i32);
                                        }
                                        let _ = (sock_id, payload);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    EventType::Disk => {
                        if let Some(op_id) = event.disk_op_id() {
                            match event.disk_subtype() {
                                Some(edgerun_sdk::DiskSubtype::ReadDone) => {
                                    if let Some(data) = event.disk_data() {
                                        let resp = Response::ok("disk read done");
                                        let bytes = resp.to_bytes();
                                        unsafe {
                                            write_output(bytes.as_ptr() as i32, bytes.len() as i32);
                                        }
                                        let _ = (op_id, data);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    EventType::Timer => {
                        if let Some(timer_id) = event.timer_id() {
                            let resp = Response::ok("timer fired");
                            let bytes = resp.to_bytes();
                            unsafe {
                                write_output(bytes.as_ptr() as i32, bytes.len() as i32);
                            }
                            let _ = timer_id;
                        }
                    }
                }
            }
            None => break,
        }
    }
    if event_count == 0 {
        let resp = Response::ok("no events");
        let bytes = resp.to_bytes();
        unsafe {
            write_output(bytes.as_ptr() as i32, bytes.len() as i32);
        }
    }
}

#[edgerun_sdk::main]
fn main(_req: Request) -> Response {
    handle_events();
    Response::html(HTML)
}
