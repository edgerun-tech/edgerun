//! `join!` macro — await multiple futures concurrently.

/// Await multiple futures concurrently and return a tuple of their results.
///
/// All futures are spawned as tasks on the current runtime, so they run
/// in parallel. The macro returns a tuple with each future's output.
///
/// # Panics
/// If called outside a runtime, or if any spawned task panics.
#[macro_export]
macro_rules! join {
    ($fut1:expr $(,)?) => {{
        let h1 = $crate::spawn(async move { $fut1.await });
        h1.blocking_recv().unwrap_or_else(|_| panic!("join task panicked"))
    }};

    ($fut1:expr, $fut2:expr $(,)?) => {{
        let h1 = $crate::spawn(async move { $fut1.await });
        let h2 = $crate::spawn(async move { $fut2.await });
        (
            h1.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h2.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
        )
    }};

    ($fut1:expr, $fut2:expr, $fut3:expr $(,)?) => {{
        let h1 = $crate::spawn(async move { $fut1.await });
        let h2 = $crate::spawn(async move { $fut2.await });
        let h3 = $crate::spawn(async move { $fut3.await });
        (
            h1.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h2.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h3.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
        )
    }};

    ($fut1:expr, $fut2:expr, $fut3:expr, $fut4:expr $(,)?) => {{
        let h1 = $crate::spawn(async move { $fut1.await });
        let h2 = $crate::spawn(async move { $fut2.await });
        let h3 = $crate::spawn(async move { $fut3.await });
        let h4 = $crate::spawn(async move { $fut4.await });
        (
            h1.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h2.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h3.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
            h4.blocking_recv().unwrap_or_else(|_| panic!("join task panicked")),
        )
    }};
}
