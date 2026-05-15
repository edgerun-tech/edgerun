#![no_std]

#[macro_export]
macro_rules! assert_eq {
    ($($tt:tt)*) => {
        ::core::assert_eq!($($tt)*)
    };
}
