//! Simple bump allocator

use core::alloc::{GlobalAlloc, Layout};

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator;

pub struct Allocator;

const HEAP_START: usize = 0x100000;
const HEAP_END: usize = 0x140000;

static mut HEAP_PTR: usize = HEAP_START;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = HEAP_PTR;
        let aligned = (ptr + layout.align() - 1) & !(layout.align() - 1);
        let end = aligned + layout.size();
        
        if end > HEAP_END {
            core::ptr::null_mut()
        } else {
            HEAP_PTR = end;
            aligned as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}