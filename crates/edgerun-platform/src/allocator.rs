//! Simple bump allocator with freelist

#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_START: usize = 0x100000;
const HEAP_END: usize = 0x200000;

static HEAP_FREE: AtomicUsize = AtomicUsize::new(HEAP_START);
static HEAP_END_ADDR: AtomicUsize = AtomicUsize::new(HEAP_END);

#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator;

pub struct Allocator;

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        
        if size == 0 {
            return core::ptr::null_mut();
        }
        
        let mut current = HEAP_FREE.load(Ordering::Acquire);
        loop {
            let aligned = (current + align - 1) & !(align - 1);
            let end = aligned + size;
            
            if end > HEAP_END_ADDR.load(Ordering::Acquire) {
                return core::ptr::null_mut();
            }
            
            let result = HEAP_FREE.compare_exchange(current, end, Ordering::Release, Ordering::Acquire);
            match result {
                Ok(_) => return aligned as *mut u8,
                Err(new) => current = new,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}