//! Minimal std-shaped compatibility surface for bare storage builds.

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
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

pub mod io {
    use alloc::format;
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::fmt;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        InvalidInput,
        InvalidData,
        UnexpectedEof,
        WriteZero,
        BrokenPipe,
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

        pub fn last_os_error() -> Self {
            Self::other("os error unavailable")
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

        fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
            let _ = buf;
            Err(Error::new(ErrorKind::UnexpectedEof, "read unavailable"))
        }
    }

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }

    #[derive(Clone, Copy, Debug)]
    pub enum SeekFrom {
        Start(u64),
        End(i64),
        Current(i64),
    }

    pub trait Seek {
        fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

        fn stream_position(&mut self) -> Result<u64> {
            self.seek(SeekFrom::Current(0))
        }
    }

    pub struct Cursor<T> {
        inner: T,
        pos: u64,
    }

    impl<T> Cursor<T> {
        pub fn new(inner: T) -> Self {
            Self { inner, pos: 0 }
        }

        pub fn position(&self) -> u64 {
            self.pos
        }
    }

    impl<T: AsRef<[u8]>> Read for Cursor<T> {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            let src = self.inner.as_ref();
            let start = self.pos as usize;
            let end = start.saturating_add(buf.len());
            if end > src.len() {
                return Err(Error::new(ErrorKind::UnexpectedEof, "cursor eof"));
            }
            buf.copy_from_slice(&src[start..end]);
            self.pos = end as u64;
            Ok(())
        }
    }

    impl Write for Vec<u8> {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            self.extend_from_slice(buf);
            Ok(())
        }
    }
}

pub mod path {
    use alloc::string::{String, ToString};

    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PathBuf(String);

    pub type Path = PathBuf;

    #[derive(Clone, Debug)]
    pub struct PathPart(String);

    impl PathBuf {
        pub fn from(path: impl Into<String>) -> Self {
            Self(path.into())
        }

        pub fn join(&self, path: impl core::fmt::Display) -> Self {
            let mut out = self.0.clone();
            if !out.is_empty() && !out.ends_with('/') {
                out.push('/');
            }
            out.push_str(&path.to_string());
            Self(out)
        }

        pub fn parent(&self) -> Option<&Path> {
            None
        }

        pub fn exists(&self) -> bool {
            false
        }

        pub fn extension(&self) -> Option<PathPart> {
            self.0
                .rsplit_once('.')
                .map(|(_, ext)| PathPart(ext.to_string()))
        }

        pub fn file_stem(&self) -> Option<PathPart> {
            let name = self.0.rsplit('/').next().unwrap_or(&self.0);
            Some(PathPart(
                name.rsplit_once('.')
                    .map(|(stem, _)| stem)
                    .unwrap_or(name)
                    .to_string(),
            ))
        }

        pub fn as_os_str(&self) -> &Self {
            self
        }

        pub fn to_path_buf(&self) -> Self {
            self.clone()
        }
    }

    impl core::fmt::Display for PathBuf {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            f.write_str(&self.0)
        }
    }

    impl PathPart {
        pub fn to_str(&self) -> Option<&str> {
            Some(&self.0)
        }

        pub fn into_string(self) -> String {
            self.0
        }
    }

    impl From<&str> for PathBuf {
        fn from(value: &str) -> Self {
            Self(value.to_string())
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
    use alloc::vec::Vec;

    pub struct File;
    pub struct Metadata;
    pub struct OpenOptions;
    pub struct DirEntry;
    pub struct ReadDir;

    impl File {
        pub fn open(_path: impl Sized) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "filesystem unavailable",
            ))
        }

        pub fn create(_path: impl Sized) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "filesystem unavailable",
            ))
        }

        pub fn metadata(&self) -> io::Result<Metadata> {
            Ok(Metadata)
        }

        pub fn sync_all(&self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Metadata {
        pub fn len(&self) -> u64 {
            0
        }
    }

    impl io::Read for File {
        fn read_exact(&mut self, _buf: &mut [u8]) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "filesystem unavailable",
            ))
        }

        fn read_to_end(&mut self, _buf: &mut Vec<u8>) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "filesystem unavailable",
            ))
        }
    }

    impl io::Write for File {
        fn write_all(&mut self, _buf: &[u8]) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "filesystem unavailable",
            ))
        }
    }

    impl edgerun_core::io::Read for File {
        fn read_exact(&mut self, buf: &mut [u8]) -> edgerun_core::io::Result<()> {
            io::Read::read_exact(self, buf).map_err(to_core_io_error)
        }
    }

    impl edgerun_core::io::Write for File {
        fn write_all(&mut self, buf: &[u8]) -> edgerun_core::io::Result<()> {
            io::Write::write_all(self, buf).map_err(to_core_io_error)
        }
    }

    fn to_core_io_error(err: io::Error) -> edgerun_core::io::Error {
        let kind = match err.kind() {
            io::ErrorKind::UnexpectedEof => edgerun_core::io::ErrorKind::UnexpectedEof,
            io::ErrorKind::InvalidData => edgerun_core::io::ErrorKind::InvalidData,
            io::ErrorKind::NotFound => edgerun_core::io::ErrorKind::NotFound,
            _ => edgerun_core::io::ErrorKind::Other,
        };
        edgerun_core::io::Error::new(kind, err)
    }

    impl io::Seek for File {
        fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "filesystem unavailable",
            ))
        }
    }

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

        pub fn read(&mut self, _read: bool) -> &mut Self {
            self
        }

        pub fn write(&mut self, _write: bool) -> &mut Self {
            self
        }

        pub fn open(&self, _path: impl Sized) -> io::Result<File> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "filesystem unavailable",
            ))
        }
    }

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
    }

    pub fn create_dir_all(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }

    pub fn remove_dir_all(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }

    pub fn read_dir(_path: impl Sized) -> io::Result<ReadDir> {
        Ok(ReadDir)
    }

    pub fn read(_path: impl Sized) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "filesystem unavailable",
        ))
    }

    pub fn read_to_string(_path: impl Sized) -> io::Result<alloc::string::String> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "filesystem unavailable",
        ))
    }

    pub fn write(_path: impl Sized, _contents: impl AsRef<[u8]>) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "filesystem unavailable",
        ))
    }

    pub fn remove_file(_path: impl Sized) -> io::Result<()> {
        Ok(())
    }
}

pub mod sync {
    use super::io;

    pub use alloc::sync::Arc;
    pub mod atomic {
        pub use core::sync::atomic::*;
    }

    pub struct Mutex<T>(edgerun_bare_rt::Mutex<T>);
    pub struct RwLock<T>(edgerun_bare_rt::RwLock<T>);

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self(edgerun_bare_rt::Mutex::new(value))
        }

        pub fn lock(&self) -> Result<edgerun_bare_rt::MutexGuard<'_, T>, io::Error> {
            Ok(self.0.lock())
        }
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T> RwLock<T> {
        pub fn new(value: T) -> Self {
            Self(edgerun_bare_rt::RwLock::new(value))
        }

        pub fn read(&self) -> RwLockReadResult<'_, T> {
            RwLockReadResult(Some(self.0.read()))
        }

        pub fn write(&self) -> RwLockWriteResult<'_, T> {
            RwLockWriteResult(Some(self.0.write()))
        }
    }

    pub struct RwLockReadResult<'a, T>(Option<edgerun_bare_rt::RwLockReadGuard<'a, T>>);
    pub struct RwLockWriteResult<'a, T>(Option<edgerun_bare_rt::RwLockWriteGuard<'a, T>>);

    impl<'a, T> RwLockReadResult<'a, T> {
        pub fn unwrap(mut self) -> edgerun_bare_rt::RwLockReadGuard<'a, T> {
            self.0.take().expect("rwlock read result consumed")
        }
    }

    impl<'a, T> RwLockWriteResult<'a, T> {
        pub fn unwrap(mut self) -> edgerun_bare_rt::RwLockWriteGuard<'a, T> {
            self.0.take().expect("rwlock write result consumed")
        }
    }

    pub mod mpsc {
        use crate::std::io;
        use core::fmt;

        pub struct SyncSender<T>(core::marker::PhantomData<T>);
        pub struct Receiver<T>(core::marker::PhantomData<T>);
        pub struct SendError<T>(pub T);

        impl<T> fmt::Display for SendError<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("send failed")
            }
        }

        impl<T> Clone for SyncSender<T> {
            fn clone(&self) -> Self {
                Self(core::marker::PhantomData)
            }
        }

        impl<T> SyncSender<T> {
            pub fn send(&self, value: T) -> Result<(), SendError<T>> {
                Err(SendError(value))
            }
        }

        impl<T> Receiver<T> {
            pub fn recv(&self) -> Result<T, io::Error> {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "channel unavailable",
                ))
            }
        }

        pub fn sync_channel<T>(_bound: usize) -> (SyncSender<T>, Receiver<T>) {
            (
                SyncSender(core::marker::PhantomData),
                Receiver(core::marker::PhantomData),
            )
        }
    }
}

pub mod thread {
    use crate::std::io;

    pub struct JoinHandle<T>(core::marker::PhantomData<T>);
    pub struct Builder;

    impl Builder {
        pub fn new() -> Self {
            Self
        }

        pub fn name(self, _name: alloc::string::String) -> Self {
            self
        }

        pub fn spawn<F, T>(self, _f: F) -> io::Result<JoinHandle<T>>
        where
            F: FnOnce() -> T + Send + 'static,
            T: Send + 'static,
        {
            Err(io::Error::new(io::ErrorKind::Other, "threads unavailable"))
        }
    }
}

pub mod env {
    pub fn temp_dir() -> crate::std::path::PathBuf {
        crate::std::path::PathBuf::default()
    }
}

pub mod process {
    pub fn id() -> u32 {
        0
    }
}

pub mod time {
    pub use core::time::Duration;

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

pub mod ffi {
    use alloc::vec::Vec;

    pub struct CString(Vec<u8>);

    impl CString {
        pub fn new(bytes: &[u8]) -> Result<Self, ()> {
            let mut out = bytes.to_vec();
            out.push(0);
            Ok(Self(out))
        }

        pub fn as_ptr(&self) -> *const i8 {
            self.0.as_ptr() as *const i8
        }
    }
}

pub mod os {
    pub mod unix {
        pub mod ffi {
            pub trait OsStrExt {
                fn as_bytes(&self) -> &[u8];
            }

            impl OsStrExt for crate::std::path::PathBuf {
                fn as_bytes(&self) -> &[u8] {
                    &[]
                }
            }
        }
    }
}

pub use alloc::format;
pub use core::{cell, cmp, convert, error, fmt, mem, option, result, slice, str};
