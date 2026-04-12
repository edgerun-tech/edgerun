//! `select!` macro — race multiple futures, take the first to complete.
//!
//! # Note
//! This macro uses `.await` internally and can only be called from within
//! an async context (e.g. inside `Runtime::block_on(async { ... })`).

/// Race two futures concurrently and return the result of the first
/// one to complete.
///
/// Both futures must have the same output type. The losing future's
/// result is discarded.
///
/// # Panics
/// If called outside a runtime.
#[macro_export]
macro_rules! select {
    ($fut1:expr, $fut2:expr $(,)?) => {{
        let (tx, rx) = $crate::oneshot::channel();
        let tx = std::sync::Arc::new(std::sync::Mutex::new(Some(tx)));

        // Spawn first future — sends its result via oneshot if first.
        let tx1 = tx.clone();
        let _h1 = $crate::spawn(async move {
            let result = $fut1.await;
            if let Ok(mut guard) = tx1.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(result);
                }
            }
        });

        // Spawn second future — sends its result via oneshot if first.
        let tx2 = tx.clone();
        let _h2 = $crate::spawn(async move {
            let result = $fut2.await;
            if let Ok(mut guard) = tx2.lock() {
                if let Some(tx) = guard.take() {
                    let _ = tx.send(result);
                }
            }
        });

        // Block until the first result arrives.
        // This is safe because spawned tasks run on worker threads.
        rx.blocking_recv()
            .unwrap_or_else(|_| panic!("select! all futures failed"))
    }};
}
