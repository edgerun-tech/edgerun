//! join! macro - stub implementations.

#![no_std]

extern crate alloc;

pub type Select2Enum<O> = alloc::boxed::Box<core::future::Ready<O>>;

pub fn select_2<F1, F2, O>(_: F1, _: F2) -> impl core::future::Future<Output = O>
where
    F1: core::future::Future<Output = O>,
    F2: core::future::Future<Output = O>,
{
    core::future::ready(unsafe { core::mem::zeroed() })
}

pub fn select_3<F1, F2, F3, O>(_: F1, _: F2, _: F3) -> impl core::future::Future<Output = O>
where
    F1: core::future::Future<Output = O>,
    F2: core::future::Future<Output = O>,
    F3: core::future::Future<Output = O>,
{
    core::future::ready(unsafe { core::mem::zeroed() })
}

pub fn select_4<F1, F2, F3, F4, O>(_: F1, _: F2, _: F3, _: F4) -> impl core::future::Future<Output = O>
where
    F1: core::future::Future<Output = O>,
    F2: core::future::Future<Output = O>,
    F3: core::future::Future<Output = O>,
    F4: core::future::Future<Output = O>,
{
    core::future::ready(unsafe { core::mem::zeroed() })
}

pub use select_2 as select_internal;