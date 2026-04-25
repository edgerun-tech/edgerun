//! `join!` macro — await multiple futures concurrently.

extern crate alloc;

use core::future::Future;

#[macro_export]
macro_rules! join {
    ($fut:expr $(,)?) => {
        $fut.await
    };

    ($($fut:expr),* $(,)?) => {{
        async { ($( $fut.await ),*) }
    }};
}

pub mod join_internal {
    use core::future::Future;
    

    

    pub async fn join2<F1, F2>(f1: F1, f2: F2) -> (F1::Output, F2::Output)
    where
        F1: Future,
        F2: Future,
    {
        (f1.await, f2.await)
    }

    pub async fn join3<F1, F2, F3>(f1: F1, f2: F2, f3: F3) -> (F1::Output, F2::Output, F3::Output)
    where
        F1: Future,
        F2: Future,
        F3: Future,
    {
        (f1.await, f2.await, f3.await)
    }

    pub async fn join4<F1, F2, F3, F4>(f1: F1, f2: F2, f3: F3, f4: F4) -> (F1::Output, F2::Output, F3::Output, F4::Output)
    where
        F1: Future,
        F2: Future,
        F3: Future,
        F4: Future,
    {
        (f1.await, f2.await, f3.await, f4.await)
    }
}


pub type Select2Enum<O> = alloc::boxed::Box<core::future::Ready<O>>;