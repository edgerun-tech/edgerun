//! Pin utilities for bare-metal async

#[macro_export]
macro_rules! pin_mut {
    ($expr:expr) => {
        core::pin::Pin::new(&mut $expr)
    };
}