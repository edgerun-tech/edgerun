pub struct Level(u8);

impl Level {
    pub const ERROR: Self = Self(1);
    pub const WARN: Self = Self(2);
    pub const INFO: Self = Self(3);
    pub const DEBUG: Self = Self(4);
}

#[inline]
pub fn log(_level: u8, _msg: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        let mut pos = 0usize;
        for byte in _msg.bytes() {
            if byte >= 0x20 {
                let addr = 0xB8000 as *mut u8;
                unsafe {
                    addr.add(pos).write_volatile(byte);
                    addr.add(pos + 1).write_volatile(0x07);
                }
                pos += 2;
            }
        }
    }
}