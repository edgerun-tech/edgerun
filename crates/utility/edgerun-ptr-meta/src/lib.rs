#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "std")]
extern crate std;

use core::{
    ffi::CStr,
    fmt, hash,
    marker::PhantomData,
    ptr,
};

/// Associates pointer metadata with a pointee type.
///
/// Pointers to [`Sized`] types have metadata `()`. Pointers to DSTs like
/// slices (`[T]`), `str`, and trait objects have metadata `usize` or
/// [`DynMetadata`] respectively.
///
/// # Safety
///
/// `Metadata` must be the correct pointer metadata type for `Self`.
pub unsafe trait Pointee {
    type Metadata: Copy + Send + Sync + Ord + hash::Hash + Unpin;
}

// SAFETY: Sized types have no extra metadata.
unsafe impl<T> Pointee for T {
    type Metadata = ();
}

// SAFETY: Slices store their length as usize metadata.
unsafe impl<T> Pointee for [T] {
    type Metadata = usize;
}

// SAFETY: `str` stores its byte length as usize metadata.
unsafe impl Pointee for str {
    type Metadata = usize;
}

// SAFETY: CStr stores its byte length (incl. nul) as usize metadata.
unsafe impl Pointee for CStr {
    type Metadata = usize;
}

#[cfg(feature = "std")]
unsafe impl Pointee for std::ffi::OsStr {
    type Metadata = usize;
}

/// Returns the metadata component of a pointer.
#[inline]
pub const fn metadata<T: Pointee + ?Sized>(ptr: *const T) -> <T as Pointee>::Metadata {
    unsafe { PtrRepr { const_ptr: ptr }.components.metadata }
}

/// Decomposes a raw pointer into its data address and metadata.
#[inline]
pub const fn to_raw_parts<T: Pointee + ?Sized>(ptr: *const T) -> (*const (), <T as Pointee>::Metadata) {
    (ptr as *const (), metadata(ptr))
}

/// Decomposes a mutable raw pointer into its data address and metadata.
#[inline]
pub const fn to_raw_parts_mut<T: Pointee + ?Sized>(ptr: *mut T) -> (*mut (), <T as Pointee>::Metadata) {
    (ptr as *mut (), metadata(ptr))
}

/// Reconstructs a raw pointer from its data address and metadata.
#[inline]
pub const fn from_raw_parts<T: Pointee + ?Sized>(
    data_address: *const (),
    metadata: <T as Pointee>::Metadata,
) -> *const T {
    unsafe {
        PtrRepr {
            components: PtrComponents { data_address, metadata },
        }
        .const_ptr
    }
}

/// Reconstructs a mutable raw pointer from its data address and metadata.
#[inline]
pub const fn from_raw_parts_mut<T: Pointee + ?Sized>(
    data_address: *mut (),
    metadata: <T as Pointee>::Metadata,
) -> *mut T {
    unsafe {
        PtrRepr {
            components: PtrComponents {
                data_address,
                metadata,
            },
        }
        .mut_ptr
    }
}

#[repr(C)]
union PtrRepr<T: Pointee + ?Sized> {
    const_ptr: *const T,
    mut_ptr: *mut T,
    components: PtrComponents<T>,
}

#[repr(C)]
struct PtrComponents<T: Pointee + ?Sized> {
    data_address: *const (),
    metadata: <T as Pointee>::Metadata,
}

impl<T: Pointee + ?Sized> Copy for PtrComponents<T> {}
impl<T: Pointee + ?Sized> Clone for PtrComponents<T> {
    fn clone(&self) -> Self { *self }
}

/// Metadata for trait objects, wrapping a vtable pointer.
pub struct DynMetadata<Dyn: ?Sized> {
    vtable_ptr: &'static VTable,
    phantom: PhantomData<Dyn>,
}

struct VTable;

impl<Dyn: ?Sized> DynMetadata<Dyn> {
    /// Returns the size of the concrete type behind the trait object.
    #[inline]
    pub fn size_of(self) -> usize {
        unsafe {
            (self.vtable_ptr as *const VTable as *const usize).add(1).read()
        }
    }

    /// Returns the alignment of the concrete type behind the trait object.
    #[inline]
    pub fn align_of(self) -> usize {
        unsafe {
            (self.vtable_ptr as *const VTable as *const usize).add(2).read()
        }
    }

    /// Returns the layout of the concrete type behind the trait object.
    #[inline]
    pub fn layout(self) -> core::alloc::Layout {
        unsafe {
            core::alloc::Layout::from_size_align_unchecked(self.size_of(), self.align_of())
        }
    }
}

// SAFETY: Vtable references are `Send` and `Sync`.
unsafe impl<Dyn: ?Sized> Send for DynMetadata<Dyn> {}
unsafe impl<Dyn: ?Sized> Sync for DynMetadata<Dyn> {}

impl<Dyn: ?Sized> fmt::Debug for DynMetadata<Dyn> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DynMetadata")
            .field(&(self.vtable_ptr as *const VTable))
            .finish()
    }
}

impl<Dyn: ?Sized> Unpin for DynMetadata<Dyn> {}
impl<Dyn: ?Sized> Copy for DynMetadata<Dyn> {}
impl<Dyn: ?Sized> Clone for DynMetadata<Dyn> {
    fn clone(&self) -> Self { *self }
}
impl<Dyn: ?Sized> Eq for DynMetadata<Dyn> {}
impl<Dyn: ?Sized> PartialEq for DynMetadata<Dyn> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq::<VTable>(self.vtable_ptr, other.vtable_ptr)
    }
}
impl<Dyn: ?Sized> Ord for DynMetadata<Dyn> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        (self.vtable_ptr as *const VTable).cmp(&(other.vtable_ptr as *const VTable))
    }
}
impl<Dyn: ?Sized> PartialOrd for DynMetadata<Dyn> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<Dyn: ?Sized> hash::Hash for DynMetadata<Dyn> {
    fn hash<H: hash::Hasher>(&self, hasher: &mut H) {
        ptr::hash::<VTable, _>(self.vtable_ptr, hasher)
    }
}

#[cfg(feature = "std")]
mod impls {
    use core::{any::Any, error::Error};
    use crate::{DynMetadata, Pointee};

    unsafe impl Pointee for dyn Any { type Metadata = DynMetadata<dyn Any>; }
    unsafe impl Pointee for dyn Any + Send { type Metadata = DynMetadata<dyn Any + Send>; }
    unsafe impl Pointee for dyn Any + Sync { type Metadata = DynMetadata<dyn Any + Sync>; }
    unsafe impl Pointee for dyn Any + Send + Sync { type Metadata = DynMetadata<dyn Any + Send + Sync>; }
    unsafe impl Pointee for dyn Error { type Metadata = DynMetadata<dyn Error>; }
    unsafe impl Pointee for dyn Error + Send { type Metadata = DynMetadata<dyn Error + Send>; }
    unsafe impl Pointee for dyn Error + Sync { type Metadata = DynMetadata<dyn Error + Sync>; }
    unsafe impl Pointee for dyn Error + Send + Sync { type Metadata = DynMetadata<dyn Error + Send + Sync>; }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: Pointee + ?Sized>(value: &T) {
        let ptr = value as *const T;
        let (raw, meta) = to_raw_parts(ptr);
        let re_ptr = from_raw_parts::<T>(raw, meta);
        assert_eq!(ptr, re_ptr);
    }

    #[test]
    fn sized_types() {
        roundtrip(&());
        roundtrip(&42u32);
        roundtrip(&true);
        roundtrip(&[1, 2, 3, 4]);
        struct Unit;
        roundtrip(&Unit);
    }

    #[test]
    fn unsized_types() {
        roundtrip("hello world");
        roundtrip(&[1, 2, 3, 4] as &[i32]);
    }

    #[test]
    fn metadata_roundtrip() {
        let s = "hello";
        assert_eq!(metadata(s as *const str), 5);
    }
}
