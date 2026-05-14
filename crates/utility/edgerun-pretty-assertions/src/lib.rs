#[macro_export]
macro_rules! assert_eq {
    ($($tt:tt)*) => {
        ::std::assert_eq!($($tt)*)
    };
}
