pub struct Level(pub u8);

impl Level {
    pub const Error: Self = Self(1);
    pub const Warn: Self = Self(2);
    pub const Info: Self = Self(3);
    pub const Debug: Self = Self(4);
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {};
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {};
}