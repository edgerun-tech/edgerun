//! Minimal std-shaped compatibility surface for bare edgerun-core builds.

pub mod prelude {
    pub mod v1 {
        pub use alloc::boxed::Box;
        pub use alloc::borrow::ToOwned;
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
    pub use alloc::collections::{BTreeMap, BTreeSet};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

pub mod io {
    use alloc::format;
    use alloc::string::String;
    use core::fmt;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        UnexpectedEof,
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

    pub trait Read {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
    }

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }
}

pub mod path {
    use alloc::string::String;

    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PathBuf(String);

    pub type Path = PathBuf;

    impl PathBuf {
        pub fn from(path: impl Into<String>) -> Self {
            Self(path.into())
        }

        pub fn join(&self, _path: impl AsRef<str>) -> Self {
            self.clone()
        }

        pub fn file_name(&self) -> Option<&str> {
            None
        }

        pub fn is_file(&self) -> bool {
            false
        }

        pub fn exists(&self) -> bool {
            false
        }
    }

    impl From<&str> for PathBuf {
        fn from(value: &str) -> Self {
            Self(String::from(value))
        }
    }

    impl From<String> for PathBuf {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
}

pub mod fs {
    use super::io;
    use super::path::PathBuf;
    use alloc::string::String;
    use alloc::vec::Vec;

    pub struct DirEntry;
    pub struct ReadDir;

    impl Iterator for ReadDir {
        type Item = io::Result<DirEntry>;

        fn next(&mut self) -> Option<Self::Item> {
            None
        }
    }

    impl DirEntry {
        pub fn path(&self) -> PathBuf {
            PathBuf::default()
        }

        pub fn file_type(&self) -> io::Result<FileType> {
            Ok(FileType)
        }
    }

    pub struct FileType;

    impl FileType {
        pub fn is_dir(&self) -> bool {
            false
        }
    }

    pub fn read_to_string(_path: impl Sized) -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound, "filesystem unavailable"))
    }

    pub fn read(_path: impl Sized) -> io::Result<Vec<u8>> {
        Err(io::Error::new(io::ErrorKind::NotFound, "filesystem unavailable"))
    }

    pub fn read_dir(_path: impl Sized) -> io::Result<ReadDir> {
        Ok(ReadDir)
    }

    pub fn create_dir_all(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }

    pub fn write(_path: impl Sized, _data: impl AsRef<[u8]>) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Other, "filesystem unavailable"))
    }

    pub fn remove_file(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }

    pub fn remove_dir_all(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }

    pub struct OpenOptions;

    impl OpenOptions {
        pub fn new() -> Self {
            Self
        }

        pub fn create(&mut self, _create: bool) -> &mut Self {
            self
        }

        pub fn append(&mut self, _append: bool) -> &mut Self {
            self
        }

        pub fn open(&self, _path: impl Sized) -> io::Result<File> {
            Err(io::Error::new(io::ErrorKind::Other, "filesystem unavailable"))
        }
    }

    pub struct File;

    impl File {
        pub fn sync_all(&self) -> io::Result<()> {
            Ok(())
        }
    }

    impl io::Write for File {
        fn write_all(&mut self, _buf: &[u8]) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::Other, "filesystem unavailable"))
        }
    }
}

pub mod env {
    use alloc::string::String;

    pub fn var(_key: &str) -> core::result::Result<String, ()> {
        Err(())
    }

    pub fn temp_dir() -> crate::path::PathBuf {
        crate::path::PathBuf::default()
    }
}

pub mod process {
    pub fn id() -> u32 {
        0
    }
}

pub mod hint {
    pub use core::hint::black_box;
}

pub mod time {
    use core::ops::{Add, Sub};

    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Instant(Duration);

    impl Instant {
        pub fn now() -> Self {
            Self(Duration::from_secs(0))
        }

        pub fn elapsed(self) -> Duration {
            Duration::ZERO
        }
    }

    impl Add<Duration> for Instant {
        type Output = Instant;

        fn add(self, rhs: Duration) -> Self::Output {
            Instant(self.0 + rhs)
        }
    }

    impl Sub<Instant> for Instant {
        type Output = Duration;

        fn sub(self, rhs: Instant) -> Self::Output {
            self.0.checked_sub(rhs.0).unwrap_or(Duration::ZERO)
        }
    }

    #[derive(Clone, Copy, Debug)]
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
}

pub mod sync {
    pub use alloc::sync::Arc;
}

pub use alloc::format;
pub use core::{cmp, convert, error, fmt, marker, mem, option, result, str};
