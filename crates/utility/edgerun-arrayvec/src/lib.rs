#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]

use core::ops::{Deref, DerefMut, Index, IndexMut};
use core::ptr;
use core::slice;

#[cfg(feature = "alloc")]
extern crate alloc;

/// A fixed-capacity vector backed by an inline array.
pub struct ArrayVec<T, const CAP: usize> {
    len: usize,
    xs: [core::mem::MaybeUninit<T>; CAP],
}

impl<T, const CAP: usize> ArrayVec<T, CAP> {
    #[inline]
    pub fn new() -> Self {
        Self {
            len: 0,
            xs: unsafe { core::mem::MaybeUninit::uninit().assume_init() },
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.len == CAP
    }

    #[inline]
    pub const fn remaining_capacity(&self) -> usize {
        CAP - self.len
    }

    #[track_caller]
    pub fn push(&mut self, element: T) {
        assert!(self.len < CAP, "ArrayVec::push: capacity exceeded");
        unsafe {
            (self.xs.as_mut_ptr() as *mut T).add(self.len).write(element);
        }
        self.len += 1;
    }

    pub fn try_push(&mut self, element: T) -> Result<(), T> {
        if self.len >= CAP {
            return Err(element);
        }
        unsafe {
            (self.xs.as_mut_ptr() as *mut T).add(self.len).write(element);
        }
        self.len += 1;
        Ok(())
    }

    #[inline]
    pub unsafe fn push_unchecked(&mut self, element: T) {
        debug_assert!(self.len < CAP);
        unsafe {
            (self.xs.as_mut_ptr() as *mut T).add(self.len).write(element);
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { (self.xs.as_ptr() as *const T).add(self.len).read() })
    }

    pub fn clear(&mut self) {
        let len = self.len;
        self.len = 0;
        let ptr = self.xs.as_mut_ptr() as *mut T;
        for i in 0..len {
            unsafe {
                ptr::drop_in_place(ptr.add(i));
            }
        }
    }

    pub fn truncate(&mut self, new_len: usize) {
        let old_len = self.len;
        if new_len >= old_len {
            return;
        }
        self.len = new_len;
        let ptr = self.xs.as_mut_ptr() as *mut T;
        for i in new_len..old_len {
            unsafe {
                ptr::drop_in_place(ptr.add(i));
            }
        }
    }

    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.xs.as_ptr() as *const T, self.len) }
    }

    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.xs.as_mut_ptr() as *mut T, self.len) }
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.xs.as_ptr() as *const T
    }

    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.xs.as_mut_ptr() as *mut T
    }

    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, const CAP: usize> Default for ArrayVec<T, CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const CAP: usize> Deref for ArrayVec<T, CAP> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const CAP: usize> DerefMut for ArrayVec<T, CAP> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const CAP: usize> Index<usize> for ArrayVec<T, CAP> {
    type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &T {
        &self.as_slice()[index]
    }
}

impl<T, const CAP: usize> IndexMut<usize> for ArrayVec<T, CAP> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut T {
        &mut self.as_mut_slice()[index]
    }
}

impl<T, const CAP: usize> Drop for ArrayVec<T, CAP> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: Clone, const CAP: usize> Clone for ArrayVec<T, CAP> {
    fn clone(&self) -> Self {
        let mut new = Self::new();
        for item in self.as_slice() {
            new.push(item.clone());
        }
        new
    }
}

impl<T, const CAP: usize> From<[T; CAP]> for ArrayVec<T, CAP> {
    fn from(arr: [T; CAP]) -> Self {
        let mut new = Self::new();
        for item in arr {
            unsafe { new.push_unchecked(item) };
        }
        new
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a ArrayVec<T, CAP> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}

impl<'a, T, const CAP: usize> IntoIterator for &'a mut ArrayVec<T, CAP> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_mut_slice().iter_mut()
    }
}

impl<T, const CAP: usize> IntoIterator for ArrayVec<T, CAP> {
    type Item = T;
    type IntoIter = ArrayVecIntoIter<T, CAP>;

    fn into_iter(self) -> Self::IntoIter {
        ArrayVecIntoIter {
            vec: self,
            index: 0,
        }
    }
}

pub struct ArrayVecIntoIter<T, const CAP: usize> {
    vec: ArrayVec<T, CAP>,
    index: usize,
}

impl<T, const CAP: usize> Iterator for ArrayVecIntoIter<T, CAP> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.index >= self.vec.len {
            return None;
        }
        let item = unsafe { (self.vec.xs.as_ptr() as *const T).add(self.index).read() };
        self.index += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.vec.len - self.index;
        (remaining, Some(remaining))
    }
}

impl<T, const CAP: usize> ExactSizeIterator for ArrayVecIntoIter<T, CAP> {}

impl<T, const CAP: usize> Drop for ArrayVecIntoIter<T, CAP> {
    fn drop(&mut self) {
        let len = self.vec.len;
        let ptr = self.vec.xs.as_mut_ptr() as *mut T;
        for i in self.index..len {
            unsafe {
                ptr::drop_in_place(ptr.add(i));
            }
        }
    }
}

impl<T, const CAP: usize> FromIterator<T> for ArrayVec<T, CAP> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut new = Self::new();
        for item in iter {
            new.push(item);
        }
        new
    }
}

impl<T: core::fmt::Debug, const CAP: usize> core::fmt::Debug for ArrayVec<T, CAP> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_push_pop() {
        let mut v = ArrayVec::<u32, 4>::new();
        assert!(v.is_empty());
        assert_eq!(v.len(), 0);
        assert_eq!(v.capacity(), 4);
        v.push(10);
        v.push(20);
        v.push(30);
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 10);
        assert_eq!(v[1], 20);
        assert_eq!(v[2], 30);
        assert_eq!(v.pop(), Some(30));
        assert_eq!(v.pop(), Some(20));
        assert_eq!(v.pop(), Some(10));
        assert_eq!(v.pop(), None);
    }

    #[test]
    #[should_panic]
    fn push_too_many() {
        let mut v = ArrayVec::<u32, 2>::new();
        v.push(1);
        v.push(2);
        v.push(3);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn clear_drops() {
        use alloc::string::String;
        let mut v = ArrayVec::<String, 4>::new();
        v.push("hello".into());
        v.push("world".into());
        assert_eq!(v.len(), 2);
        v.clear();
        assert_eq!(v.len(), 0);
    }

    #[test]
    fn from_array() {
        let v = ArrayVec::from([1u32, 2, 3, 4]);
        assert_eq!(v.len(), 4);
        assert_eq!(v.as_slice(), &[1, 2, 3, 4]);
    }

    #[test]
    fn deref_to_slice() {
        let mut v = ArrayVec::<u32, 8>::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.as_slice(), &[1, 2, 3]);
        v.as_mut_slice()[1] = 42;
        assert_eq!(v[1], 42);
    }

    #[test]
    fn iteration() {
        let mut v = ArrayVec::<u32, 8>::new();
        v.push(1);
        v.push(2);
        v.push(3);
        let mut sum = 0;
        for x in &v {
            sum += x;
        }
        assert_eq!(sum, 6);
    }

    #[test]
    fn into_iter() {
        let mut v = ArrayVec::<u32, 4>::new();
        v.push(1);
        v.push(2);
        let mut iter = v.into_iter();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn truncate() {
        let mut v = ArrayVec::<u32, 8>::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        v.truncate(2);
        assert_eq!(v.len(), 2);
        assert_eq!(v.as_slice(), &[1, 2]);
    }

    #[test]
    fn try_push_overflow() {
        let mut v = ArrayVec::<u32, 2>::new();
        assert!(v.try_push(1).is_ok());
        assert!(v.try_push(2).is_ok());
        let r = v.try_push(3);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), 3);
    }

    #[test]
    fn is_full() {
        let mut v = ArrayVec::<u32, 2>::new();
        assert!(!v.is_full());
        v.push(1);
        assert!(!v.is_full());
        v.push(2);
        assert!(v.is_full());
    }
}
