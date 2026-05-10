#![cfg_attr(not(test), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Trait for overwriting sensitive memory before it is released or reused.
///
/// The implementation uses volatile writes plus a compiler fence so LLVM cannot
/// remove the wipe as an apparently-unused store.
pub trait Zeroize {
    /// Overwrite this value with zeros or an equivalent inert representation.
    fn zeroize(&mut self);
}

#[inline]
fn zeroize_bytes(bytes: &mut [u8]) {
    for byte in bytes {
        // SAFETY: `byte` is a valid mutable reference for exactly one byte.
        unsafe { core::ptr::write_volatile(byte, 0) };
    }
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

impl Zeroize for [u8] {
    #[inline]
    fn zeroize(&mut self) {
        zeroize_bytes(self);
    }
}

impl<const N: usize> Zeroize for [u8; N] {
    #[inline]
    fn zeroize(&mut self) {
        self.as_mut_slice().zeroize();
    }
}

macro_rules! impl_zeroize_int {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Zeroize for $ty {
                #[inline]
                fn zeroize(&mut self) {
                    // SAFETY: `self` is a valid mutable reference for one value.
                    unsafe { core::ptr::write_volatile(self, 0) };
                    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
                }
            }
        )*
    };
}

impl_zeroize_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

#[cfg(feature = "alloc")]
impl Zeroize for alloc::vec::Vec<u8> {
    #[inline]
    fn zeroize(&mut self) {
        self.as_mut_slice().zeroize();
    }
}

/// Wrapper that zeroizes its inner value on drop.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct Zeroizing<T: Zeroize>(pub T);

impl<T: Zeroize> Zeroizing<T> {
    /// Wrap a value so it is wiped on drop.
    pub const fn new(value: T) -> Self {
        Self(value)
    }

    /// Consume the wrapper without running the drop wipe.
    pub fn into_inner(mut self) -> T {
        // SAFETY: `value` is read out and `self` is forgotten so Drop does not run.
        let value = unsafe { core::ptr::read(&self.0) };
        core::mem::forget(self);
        value
    }
}

impl<T: Zeroize> core::ops::Deref for Zeroizing<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Zeroize> core::ops::DerefMut for Zeroizing<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Zeroize> Drop for Zeroizing<T> {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::Zeroize;

    #[test]
    fn zeroizes_byte_arrays() {
        let mut bytes = [1u8, 2, 3, 4];
        bytes.zeroize();
        assert_eq!(bytes, [0, 0, 0, 0]);
    }

    #[test]
    fn zeroizes_vec_without_changing_length() {
        let mut bytes = vec![1u8, 2, 3, 4];
        bytes.zeroize();
        assert_eq!(bytes, vec![0, 0, 0, 0]);
    }
}
