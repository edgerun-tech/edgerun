//! Simple inotify wrapper using raw syscalls.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

#[cfg(target_os = "none")]
pub mod cmp {
    pub use core::cmp::*;
}

#[cfg(target_os = "none")]
pub mod ffi {
    use alloc::vec::Vec;

    #[derive(Clone, Debug)]
    pub struct OsString;

    pub struct OsStr;

    impl OsStr {
        pub fn from_bytes(_bytes: &[u8]) -> &'static Self {
            static OS_STR: OsStr = OsStr;
            &OS_STR
        }

        #[must_use]
        pub fn as_bytes(&self) -> &[u8] {
            &[]
        }

        #[must_use]
        pub fn to_owned(&self) -> OsString {
            OsString
        }
    }

    pub struct CString(Vec<u8>);

    impl CString {
        pub unsafe fn from_vec_unchecked(bytes: Vec<u8>) -> Self {
            Self(bytes)
        }

        #[must_use]
        pub fn as_ptr(&self) -> *const i8 {
            self.0.as_ptr().cast()
        }
    }
}

#[cfg(target_os = "none")]
pub mod io {
    pub use edgerun_linux_sysfs::io::*;
}

#[cfg(target_os = "none")]
pub mod mem {
    pub use core::mem::*;
}

#[cfg(target_os = "none")]
pub mod os {
    pub mod fd {
        pub trait IntoRawFd {
            fn into_raw_fd(self) -> i32;
        }

        pub trait FromRawFd {
            unsafe fn from_raw_fd(fd: i32) -> Self;
        }
    }

    pub mod unix {
        pub mod ffi {
            pub trait OsStrExt {
                fn as_bytes(&self) -> &[u8];
            }

            impl OsStrExt for crate::ffi::OsStr {
                fn as_bytes(&self) -> &[u8] {
                    self.as_bytes()
                }
            }
        }
    }
}

#[cfg(target_os = "none")]
pub mod path {
    pub struct Path;

    impl Path {
        #[must_use]
        pub fn as_os_str(&self) -> &crate::ffi::OsStr {
            crate::ffi::OsStr::from_bytes(&[])
        }
    }
}

#[cfg(target_os = "none")]
pub mod slice {
    pub use core::slice::*;
}

#[cfg(target_os = "none")]
pub mod option {
    pub use core::option::*;
}

#[cfg(target_os = "none")]
pub mod result {
    pub use core::result::*;
}

#[cfg(target_os = "none")]
pub mod libc {
    #[allow(non_camel_case_types)]
    pub type c_char = i8;
    #[allow(non_camel_case_types)]
    pub type c_void = core::ffi::c_void;

    pub unsafe fn inotify_init1(_flags: i32) -> i32 {
        -1
    }

    pub unsafe fn read(_fd: i32, _buf: *mut c_void, _len: usize) -> isize {
        -1
    }

    pub unsafe fn close(_fd: i32) -> i32 {
        0
    }

    pub unsafe fn inotify_add_watch(_fd: i32, _path: *const i8, _mask: u32) -> i32 {
        -1
    }
}

use alloc::borrow::ToOwned;
use alloc::vec::Vec;
use core::cmp::Ord;
use core::convert::AsRef;
use core::fmt;
use core::fmt::Write;
use core::ops::Drop;
use core::option::Option::{self, None, Some};
use core::result::Result::{Err, Ok};
#[cfg(not(target_os = "none"))]
use std::io;
use std::os::fd::{FromRawFd, IntoRawFd};
use std::os::unix::ffi::OsStrExt;

#[repr(C)]
struct InotifyEvent {
    wd: i32,
    mask: u32,
    cookie: u32,
    len: u32,
    name: [libc::c_char; 0],
}

pub struct Inotify {
    fd: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WatchMask(u32);

impl WatchMask {
    pub const MODIFY: WatchMask = WatchMask(0x0000_0002);
    pub const CREATE: WatchMask = WatchMask(0x0000_0100);
    pub const DELETE: WatchMask = WatchMask(0x0000_0200);
    pub const DELETE_SELF: WatchMask = WatchMask(0x0000_0800);
    pub const MOVED_FROM: WatchMask = WatchMask(0x0000_0400);
    pub const MOVED_TO: WatchMask = WatchMask(0x0000_0800);
    pub const IGNORED: WatchMask = WatchMask(0x0000_8000);
    pub const ISDIR: WatchMask = WatchMask(0x4000_0000);
    pub const ONESHOT: WatchMask = WatchMask(8000_0000);

    pub fn new(bits: u32) -> Self {
        WatchMask(bits)
    }

    pub fn contains(&self, other: WatchMask) -> bool {
        self.0 & other.0 != 0
    }
}

impl core::ops::BitOr for WatchMask {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        WatchMask(self.0 | other.0)
    }
}

impl core::ops::BitOrAssign for WatchMask {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl fmt::Debug for Inotify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Inotify {{ fd: {} }}", self.fd)
    }
}

impl Inotify {
    pub fn init() -> io::Result<Self> {
        let fd = unsafe { libc::inotify_init1(0) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Inotify { fd })
    }

    pub fn watches(&self) -> Watches {
        Watches { fd: self.fd }
    }

    pub fn read_events(&mut self, buffer: &mut [u8]) -> io::Result<Vec<Event>> {
        let len = unsafe {
            libc::read(
                self.fd,
                buffer.as_mut_ptr() as *mut libc::c_void,
                buffer.len(),
            )
        };
        if len < 0 {
            return Err(io::Error::last_os_error());
        }
        if len == 0 {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let mut offset = 0;
        while offset < len as usize {
            let ptr = unsafe { buffer.as_ptr().add(offset) as *const InotifyEvent };
            let event_len = unsafe { (*ptr).len as usize };
            let name = if event_len > 0 {
                let name_ptr = unsafe { (*ptr).name.as_ptr() };
                Some(unsafe {
                    std::ffi::OsStr::from_bytes(std::slice::from_raw_parts(
                        name_ptr as *const u8,
                        event_len,
                    ))
                    .to_owned()
                })
            } else {
                None
            };
            events.push(Event {
                wd: unsafe { (*ptr).wd },
                mask: WatchMask(unsafe { (*ptr).mask }),
                cookie: unsafe { (*ptr).cookie },
                name,
            });
            let total_len = std::mem::size_of::<InotifyEvent>() + event_len;
            offset += std::cmp::max(total_len, std::mem::size_of::<InotifyEvent>());
        }
        Ok(events)
    }
}

impl Drop for Inotify {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

impl IntoRawFd for Inotify {
    fn into_raw_fd(self) -> i32 {
        let fd = self.fd;
        std::mem::forget(self);
        fd
    }
}

impl FromRawFd for Inotify {
    unsafe fn from_raw_fd(fd: i32) -> Self {
        Inotify { fd }
    }
}

pub struct Watches {
    fd: i32,
}

impl Watches {
    pub fn add<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        mask: WatchMask,
    ) -> io::Result<WatchDescriptor> {
        let path_bytes = path.as_ref().as_os_str().as_bytes();
        let mut path_buf = path_bytes.to_vec();
        path_buf.push(0);
        let path_c = unsafe { std::ffi::CString::from_vec_unchecked(path_buf) };
        let wd = unsafe { libc::inotify_add_watch(self.fd, path_c.as_ptr(), mask.0) };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(WatchDescriptor(wd))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WatchDescriptor(i32);

#[derive(Debug, Clone)]
pub struct Event {
    pub wd: i32,
    pub mask: WatchMask,
    pub cookie: u32,
    pub name: Option<std::ffi::OsString>,
}
