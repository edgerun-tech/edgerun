pub struct Level(pub u8);

impl Level {
    pub const Error: Self = Self(1);
    pub const Warn: Self = Self(2);
    pub const Info: Self = Self(3);
    pub const Debug: Self = Self(4);
}

pub static LOG_LEVEL: Level = Level::Info;

#[inline]
pub fn set_level(level: Level) {
    let _ = level;
}

pub fn set_logger(_logger: fn(u8, &str)) {
}

#[inline]
pub fn log(level: u8, msg: &str) {
    if level >= LOG_LEVEL.0 {
        #[cfg(target_arch = "x86_64")]
        {
            puts(msg);
        }
    }
}

#[cfg(target_arch = "x86_64")]
fn puts(s: &str) {
    let mut pos = 0usize;
    for byte in s.bytes() {
        if byte == b'\n' {
            pos = (pos / 160 + 1) * 160;
            continue;
        }
        if byte >= 0x20 && pos < 4000 {
            let addr = 0xB8000 as *mut u8;
            unsafe {
                addr.add(pos).write_volatile(byte);
                addr.add(pos + 1).write_volatile(0x07);
            }
            pos += 2;
        }
    }
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "x86_64")]
        crate::log::log(1, &alloc::format!($($arg)*));
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "x86_64")]
        crate::log::log(2, &alloc::format!($($arg)*));
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "x86_64")]
        crate::log::log(3, &alloc::format!($($arg)*));
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(target_arch = "x86_64")]
        crate::log::log(4, &alloc::format!($($arg)*));
    };
}