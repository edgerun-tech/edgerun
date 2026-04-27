pub struct Level(u8);

impl Level {
    pub const Error: Self = Self(1);
    pub const Warn: Self = Self(2);
    pub const Info: Self = Self(3);
    pub const Debug: Self = Self(4);
    pub const Trace: Self = Self(5);

    pub const ERROR: Self = Self(1);
    pub const WARN: Self = Self(2);
    pub const INFO: Self = Self(3);
    pub const DEBUG: Self = Self(4);
    pub const TRACE: Self = Self(5);
}

#[inline]
pub fn log(_level: u8, _msg: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        let serial_port = 0x3F8u16;
        for byte in _msg.bytes() {
            unsafe {
                core::arch::asm!("out dx, al", in("al") byte, in("dx") serial_port);
            }
        }
        unsafe {
            core::arch::asm!("out dx, al", in("al") b'\n', in("dx") serial_port);
        }
    }
}
