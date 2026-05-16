#[cfg(feature = "default-hasher")]
use core::hash::{BuildHasher, Hasher};

/// Default hash builder for the `S` type parameter of
/// [`HashMap`](crate::HashMap) and [`HashSet`](crate::HashSet).
///
/// This only implements `BuildHasher` when the "default-hasher" crate feature
/// is enabled; otherwise it just serves as a placeholder, and a custom `S` type
/// must be used to have a fully functional `HashMap` or `HashSet`.
#[derive(Clone, Debug, Default)]
pub struct DefaultHashBuilder {
    #[cfg(feature = "default-hasher")]
    seed: u64,
}

#[cfg(feature = "default-hasher")]
impl BuildHasher for DefaultHashBuilder {
    type Hasher = DefaultHasher;

    #[inline(always)]
    fn build_hasher(&self) -> Self::Hasher {
        DefaultHasher {
            hash: self.seed ^ 0xcbf2_9ce4_8422_2325,
        }
    }
}

/// Default hasher for [`HashMap`](crate::HashMap) and [`HashSet`](crate::HashSet).
#[cfg(feature = "default-hasher")]
#[derive(Clone)]
pub struct DefaultHasher {
    hash: u64,
}

#[cfg(feature = "default-hasher")]
impl Hasher for DefaultHasher {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    #[inline(always)]
    fn write_u8(&mut self, arg: u8) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_u16(&mut self, arg: u16) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_u32(&mut self, arg: u32) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_u64(&mut self, arg: u64) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_u128(&mut self, arg: u128) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_usize(&mut self, arg: usize) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_i8(&mut self, arg: i8) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_i16(&mut self, arg: i16) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_i32(&mut self, arg: i32) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_i64(&mut self, arg: i64) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_i128(&mut self, arg: i128) {
        self.write(&arg.to_ne_bytes());
    }

    #[inline(always)]
    fn write_isize(&mut self, arg: isize) {
        self.write(&arg.to_ne_bytes());
    }
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.hash
    }
}
