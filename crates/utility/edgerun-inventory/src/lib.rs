//! Minimal EdgeRun inventory compatibility surface.

#![no_std]

#[macro_export]
macro_rules! collect {
    ($ty:ty $(,)?) => {};
}

#[macro_export]
macro_rules! submit {
    ($($item:tt)*) => {
        const _: () = ();
    };
}
