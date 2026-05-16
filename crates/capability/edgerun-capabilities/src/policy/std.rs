//! Minimal std-shaped compatibility surface for bare capability policy builds.

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod boxed {
    pub use alloc::boxed::Box;
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

pub mod time {
    use core::ops::{Add, Sub};

    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
    pub struct SystemTime(Duration);

    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

    impl SystemTime {
        pub fn now() -> Self {
            Self(Duration::from_secs(0))
        }

        pub fn duration_since(self, earlier: SystemTime) -> core::result::Result<Duration, ()> {
            self.0.checked_sub(earlier.0).ok_or(())
        }
    }

    impl Add<Duration> for SystemTime {
        type Output = SystemTime;

        fn add(self, rhs: Duration) -> Self::Output {
            SystemTime(self.0 + rhs)
        }
    }

    impl Sub<Duration> for SystemTime {
        type Output = SystemTime;

        fn sub(self, rhs: Duration) -> Self::Output {
            SystemTime(self.0.checked_sub(rhs).unwrap_or(Duration::ZERO))
        }
    }
}

pub use alloc::format;
pub use core::{cmp, convert, error, fmt, marker, mem, option, result, slice, str};
