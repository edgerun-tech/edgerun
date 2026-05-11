#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::ops::Deref;

pub unsafe trait StableDeref: Deref {}

pub unsafe trait CloneStableDeref: StableDeref + Clone {}

#[cfg(feature = "alloc")]
use alloc::boxed::Box;
#[cfg(feature = "alloc")]
use alloc::rc::Rc;
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
use alloc::sync::Arc;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::borrow::Cow;

#[cfg(feature = "std")]
use std::ffi::{CStr, CString, OsStr, OsString};
#[cfg(feature = "std")]
use std::path::{Path, PathBuf};
#[cfg(feature = "std")]
use std::sync::{MutexGuard, RwLockReadGuard, RwLockWriteGuard};

use core::cell::{Ref, RefMut};

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> StableDeref for Box<T> {}
#[cfg(feature = "alloc")]
unsafe impl<T> StableDeref for Vec<T> {}
#[cfg(feature = "alloc")]
unsafe impl StableDeref for String {}
#[cfg(feature = "std")]
unsafe impl StableDeref for CString {}
#[cfg(feature = "std")]
unsafe impl StableDeref for OsString {}
#[cfg(feature = "std")]
unsafe impl StableDeref for PathBuf {}

#[cfg(feature = "alloc")]
unsafe impl<'a> StableDeref for Cow<'a, str> {}
#[cfg(feature = "alloc")]
unsafe impl<'a, T: Clone> StableDeref for Cow<'a, [T]> {}
#[cfg(feature = "std")]
unsafe impl<'a> StableDeref for Cow<'a, Path> {}
#[cfg(feature = "std")]
unsafe impl<'a> StableDeref for Cow<'a, CStr> {}
#[cfg(feature = "std")]
unsafe impl<'a> StableDeref for Cow<'a, OsStr> {}

#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> StableDeref for Rc<T> {}
#[cfg(feature = "alloc")]
unsafe impl<T: ?Sized> CloneStableDeref for Rc<T> {}
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
unsafe impl<T: ?Sized> StableDeref for Arc<T> {}
#[cfg(all(feature = "alloc", target_has_atomic = "ptr"))]
unsafe impl<T: ?Sized> CloneStableDeref for Arc<T> {}

unsafe impl<'a, T: ?Sized> StableDeref for Ref<'a, T> {}
unsafe impl<'a, T: ?Sized> StableDeref for RefMut<'a, T> {}
#[cfg(feature = "std")]
unsafe impl<'a, T: ?Sized> StableDeref for MutexGuard<'a, T> {}
#[cfg(feature = "std")]
unsafe impl<'a, T: ?Sized> StableDeref for RwLockReadGuard<'a, T> {}
#[cfg(feature = "std")]
unsafe impl<'a, T: ?Sized> StableDeref for RwLockWriteGuard<'a, T> {}

unsafe impl<'a, T: ?Sized> StableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> CloneStableDeref for &'a T {}
unsafe impl<'a, T: ?Sized> StableDeref for &'a mut T {}
