#![no_std]

//! Minimal source annotations for deterministic Edgerun wasm units.
//!
//! Unit exports use the wasm scalar ABI only: `i32`, `i64`, `f32`, and `f64`.
//! Pointer-like values are represented as integer offsets into the unit's own
//! exported memory. Units should not import host memory or ambient host state.

pub const ABI_VERSION: i32 = 2;

pub use edgerun_unit_macros::export;

#[macro_export]
macro_rules! metadata {
    ($standard_id:literal) => {
        #[no_mangle]
        pub extern "C" fn proto_abi_version() -> i32 {
            $crate::ABI_VERSION
        }

        #[no_mangle]
        pub extern "C" fn proto_standard_id() -> i32 {
            $standard_id
        }
    };
}

#[macro_export]
macro_rules! no_alloc {
    () => {
        struct EdgerunUnitNoAlloc;

        unsafe impl core::alloc::GlobalAlloc for EdgerunUnitNoAlloc {
            unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
                core::ptr::null_mut()
            }

            unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {}
        }

        #[global_allocator]
        static EDGERUN_UNIT_ALLOCATOR: EdgerunUnitNoAlloc = EdgerunUnitNoAlloc;

        #[panic_handler]
        fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
            loop {}
        }
    };
}
