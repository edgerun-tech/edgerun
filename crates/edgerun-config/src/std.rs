//! Minimal std-shaped compatibility surface for no_std config parsing.

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

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

pub mod net {
    pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
}

pub mod time {
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug)]
    pub struct SystemTime;
    pub struct UnixEpoch;
    pub const UNIX_EPOCH: UnixEpoch = UnixEpoch;

    impl SystemTime {
        pub fn now() -> Self {
            Self
        }

        pub fn duration_since(&self, _epoch: UnixEpoch) -> Result<Duration, ()> {
            Ok(Duration::from_secs(0))
        }
    }
}

pub use alloc::format;
pub use core::{cmp, convert, error, fmt, mem, option, result, str};
