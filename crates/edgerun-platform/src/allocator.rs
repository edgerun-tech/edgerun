//! Simple bump allocator with freelist

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_arch = "xtensa")]
const HEAP_MIN_START: usize = 0;
#[cfg(not(target_arch = "xtensa"))]
const HEAP_MIN_START: usize = 0x200000;

#[cfg(target_arch = "xtensa")]
const HEAP_END: usize = 0x3fcd_b700;
#[cfg(not(target_arch = "xtensa"))]
const HEAP_END: usize = 0x1000000;

static HEAP_FREE: AtomicUsize = AtomicUsize::new(0);
static HEAP_END_ADDR: AtomicUsize = AtomicUsize::new(HEAP_END);
#[cfg(target_arch = "xtensa")]
static mut XTENSA_HEAP_FREE: usize = 0;

unsafe extern "C" {
    static _end: u8;
}

#[cfg(target_os = "none")]
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

        #[cfg(target_arch = "xtensa")]
        {
            let mut current = XTENSA_HEAP_FREE;
            if current == 0 {
                current = initial_heap_start();
            }
            let aligned = (current + align - 1) & !(align - 1);
            let end = aligned + size;
            if end > HEAP_END {
                return core::ptr::null_mut();
            }
            XTENSA_HEAP_FREE = end;
            return aligned as *mut u8;
        }

        #[cfg(not(target_arch = "xtensa"))]
        {
            let mut current = HEAP_FREE.load(Ordering::Acquire);
            if current == 0 {
                let start = initial_heap_start();
                match HEAP_FREE.compare_exchange(0, start, Ordering::Release, Ordering::Acquire) {
                    Ok(_) => current = start,
                    Err(value) => current = value,
                }
            }
            loop {
                let aligned = (current + align - 1) & !(align - 1);
                let end = aligned + size;

                if end > HEAP_END_ADDR.load(Ordering::Acquire) {
                    return core::ptr::null_mut();
                }

                let result =
                    HEAP_FREE.compare_exchange(current, end, Ordering::Release, Ordering::Acquire);
                match result {
                    Ok(_) => return aligned as *mut u8,
                    Err(new) => current = new,
                }
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

fn initial_heap_start() -> usize {
    let linker_end = core::ptr::addr_of!(_end) as usize;
    align_up(core::cmp::max(linker_end, HEAP_MIN_START), 4096)
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
