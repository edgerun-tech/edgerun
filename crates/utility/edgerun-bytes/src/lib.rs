//! Edgerun byte-casting helpers.

/// Marker for types that can be safely initialized with all-zero bytes.
///
/// # Safety
///
/// Implementors must ensure the all-zero bit pattern is a valid value for the
/// type.
pub unsafe trait Zeroable: Copy {}

/// Marker for plain-old-data types that can be viewed as raw bytes.
///
/// # Safety
///
/// Implementors must have no padding with undefined contents, no invalid bit
/// patterns, and no drop behavior.
pub unsafe trait Pod: Zeroable {}

macro_rules! impl_pod_primitive {
    ($($ty:ty),* $(,)?) => {
        $(
            unsafe impl Zeroable for $ty {}
            unsafe impl Pod for $ty {}
        )*
    };
}

impl_pod_primitive!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

/// View a POD slice as bytes.
pub fn cast_slice<T: Pod>(slice: &[T]) -> &[u8] {
    let byte_len = std::mem::size_of_val(slice);
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), byte_len) }
}
