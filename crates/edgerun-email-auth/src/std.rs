//! Minimal std-shaped compatibility surface backed by core and alloc.

pub use core::{fmt, future, net, result, str};

pub mod collections {
    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
}

pub mod io {
    use alloc::format;
    use alloc::string::String;
    use core::fmt;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        InvalidData,
        NotFound,
        Other,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Error {
        kind: ErrorKind,
        message: String,
    }

    impl Error {
        pub fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
            Self {
                kind,
                message: format!("{message}"),
            }
        }

        pub fn other(message: impl fmt::Display) -> Self {
            Self::new(ErrorKind::Other, message)
        }

        pub fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.message)
        }
    }

    impl core::error::Error for Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}

pub mod sync {
    pub use alloc::sync::Arc;
}
