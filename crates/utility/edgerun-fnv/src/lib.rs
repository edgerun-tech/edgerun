#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), test))]
extern crate alloc;

#[cfg(feature = "std")]
use std::default::Default;
#[cfg(feature = "std")]
use std::hash::{Hasher, BuildHasherDefault};
#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};
#[cfg(not(feature = "std"))]
use core::default::Default;
#[cfg(not(feature = "std"))]
use core::hash::{Hasher, BuildHasherDefault};

#[allow(missing_copy_implementations)]
pub struct FnvHasher(u64);

impl Default for FnvHasher {
    #[inline]
    fn default() -> FnvHasher {
        FnvHasher(0xcbf29ce484222325)
    }
}

impl FnvHasher {
    #[inline]
    pub fn with_key(key: u64) -> FnvHasher {
        FnvHasher(key)
    }
}

impl Hasher for FnvHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let FnvHasher(mut hash) = *self;

        for byte in bytes.iter() {
            hash = hash ^ (*byte as u64);
            hash = hash.wrapping_mul(0x100000001b3);
        }

        *self = FnvHasher(hash);
    }
}

pub type FnvBuildHasher = BuildHasherDefault<FnvHasher>;

#[cfg(feature = "std")]
pub type FnvHashMap<K, V> = HashMap<K, V, FnvBuildHasher>;

#[cfg(feature = "std")]
pub type FnvHashSet<T> = HashSet<T, FnvBuildHasher>;

#[cfg(test)]
mod test {
    use super::*;
    #[cfg(feature = "std")]
    use std::hash::Hasher;
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;

    fn fnv1a(bytes: &[u8]) -> u64 {
        let mut hasher = FnvHasher::default();
        hasher.write(bytes);
        hasher.finish()
    }

    fn repeat_10(bytes: &[u8]) -> Vec<u8> {
        (0..10).flat_map(|_| bytes.iter().cloned()).collect()
    }

    fn repeat_500(bytes: &[u8]) -> Vec<u8> {
        (0..500).flat_map(|_| bytes.iter().cloned()).collect()
    }

    #[test]
    fn basic_tests() {
        assert_eq!(fnv1a(b""), 0xcbf29ce484222325);
        assert_eq!(fnv1a(b"a"), 0xaf63dc4c8601ec8c);
        assert_eq!(fnv1a(b"b"), 0xaf63df4c8601f1a5);
        assert_eq!(fnv1a(b"c"), 0xaf63de4c8601eff2);
        assert_eq!(fnv1a(b"d"), 0xaf63d94c8601e773);
        assert_eq!(fnv1a(b"e"), 0xaf63d84c8601e5c0);
        assert_eq!(fnv1a(b"f"), 0xaf63db4c8601ead9);
        assert_eq!(fnv1a(b"fo"), 0x08985907b541d342);
        assert_eq!(fnv1a(b"foo"), 0xdcb27518fed9d577);
        assert_eq!(fnv1a(b"foob"), 0xdd120e790c2512af);
        assert_eq!(fnv1a(b"fooba"), 0xcac165afa2fef40a);
        assert_eq!(fnv1a(b"foobar"), 0x85944171f73967e8);
        assert_eq!(fnv1a(b"\0"), 0xaf63bd4c8601b7df);
        assert_eq!(fnv1a(b"a\0"), 0x089be207b544f1e4);
        assert_eq!(fnv1a(b"b\0"), 0x08a61407b54d9b5f);
        assert_eq!(fnv1a(b"c\0"), 0x08a2ae07b54ab836);
        assert_eq!(fnv1a(b"d\0"), 0x0891b007b53c4869);
        assert_eq!(fnv1a(b"e\0"), 0x088e4a07b5396540);
        assert_eq!(fnv1a(b"f\0"), 0x08987c07b5420ebb);
        assert_eq!(fnv1a(b"fo\0"), 0xdcb28a18fed9f926);
        assert_eq!(fnv1a(b"foo\0"), 0xdd1270790c25b935);
    }
}
