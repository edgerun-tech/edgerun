//! Minimal EdgeRun inventory compatibility surface.

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
