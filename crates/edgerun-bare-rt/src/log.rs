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

static LOG_BUF: &mut dyn Fn(u8, &str) = &|_level, _msg| {};

pub fn set_logger(logger: fn(u8, &str)) {
    let _ = logger;
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
    let mut col = 0u16;
    let mut row = 0u16;
    unsafe {
        core::arch::asm!(
            "mov dx, 0x3D4",
            "mov al, 0x0E",
            "out dx, al",
            "mov dx, 0x3D5",
            "in al, dx",
            "mov bh, al",
            "mov dx, 0x3D4",
            "mov al, 0x0F",
            "out dx, al", 
            "mov dx, 0x3D5",
            "in al, dx",
            "mov bl, al",
            out("bx") _,
        );
    }
    let pos = (row as usize * 80 + col as usize) * 2;
    for byte in s.bytes() {
        if byte == b'\n' {
            col = 0;
            row = row.saturating_add(1);
            if row >= 25 {
                row = 0;
            }
            continue;
        }
        if byte >= 0x20 {
            let addr = 0xB8000 as *mut u8;
            unsafe {
                addr.offset((pos as isize) as *mut u8).write_volatile(byte);
                addr.offset((pos as isize + 1) as *mut u8).write_volatile(0x07);
            }
            col += 1;
            if col >= 80 {
                col = 0;
                row = row.saturating_add(1);
                if row >= 25 {
                    row = 0;
                }
            }
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