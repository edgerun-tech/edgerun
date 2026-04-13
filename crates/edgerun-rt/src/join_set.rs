//! JoinSet — manage a dynamic set of spawned tasks.
//!
//! Tasks are spawned on the set and awaited via [`JoinSet::join_next()`],
//! which returns the result of whichever task completes next.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use crate::sync::Mutex;

use crate::oneshot;
use crate::runtime::spawn;

/// A collection of spawned tasks that can be awaited as a group.
///
/// Tasks are completed in the order they finish, not the order they were spawned.
/// When the set is dropped, remaining tasks are aborted.
pub struct JoinSet<T> {
    receivers: VecDeque<oneshot::Receiver<Result<T, crate::blocking_pool::JoinError>>>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl<T> Default for JoinSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> JoinSet<T> {
    pub fn new() -> Self {
        Self {
            receivers: VecDeque::new(),
            waker: Arc::new(Mutex::new(None)),
        }
    }

    /// Spawn a future on the current runtime and add it to the set.
    ///
    /// # Panics
    /// If called outside a runtime.
    pub fn spawn<F>(&mut self, f: F)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let waker = self.waker.clone();
        spawn(async move {
            let result = f.await;
            let _ = tx.send(Ok(result));
            if let Some(w) = waker.lock().take() {
                w.wake();
            }
        });
        self.receivers.push_back(rx);
    }

    /// Number of unfinished tasks in the set.
    pub fn len(&self) -> usize {
        self.receivers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.receivers.is_empty()
    }

    /// Wait for the next task to complete, removing it from the set.
    ///
    /// Returns `None` when the set is empty.
    pub fn join_next(&mut self) -> JoinNext<'_, T> {
        JoinNext { set: self }
    }

    /// Abort all tasks in the set by dropping their receivers.
    pub fn abort_all(&mut self) {
        self.receivers.clear();
    }

    /// Wait for all tasks to complete, collecting their results.
    pub async fn join_all(mut self) -> Vec<Result<T, crate::blocking_pool::JoinError>>
    where
        T: Send + 'static,
    {
        let mut results = Vec::new();
        while let Some(result) = self.join_next().await {
            results.push(result);
        }
        results
    }
}

impl<T> Drop for JoinSet<T> {
    fn drop(&mut self) {
        self.abort_all();
    }
}

/// Future returned by [`JoinSet::join_next()`].
pub struct JoinNext<'a, T> {
    set: &'a mut JoinSet<T>,
}

impl<T: Send + 'static> Future for JoinNext<'_, T> {
    type Output = Option<Result<T, crate::blocking_pool::JoinError>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };

        // Register our waker so any completing task can wake us.
        *this.set.waker.lock() = Some(cx.waker().clone());

        // Scan receivers for a ready one.
        let len = this.set.receivers.len();
        for i in 0..len {
            // Safety: we have exclusive access through the &mut set reference.
            // We pin each receiver in place within the VecDeque.
            let rx = &mut this.set.receivers[i];
            let rx_pin = unsafe { Pin::new_unchecked(rx) };
            match rx_pin.poll(cx) {
                Poll::Ready(Ok(val)) => {
                    this.set.receivers.remove(i);
                    return Poll::Ready(Some(val));
                }
                Poll::Ready(Err(_)) => {
                    this.set.receivers.remove(i);
                    return Poll::Ready(Some(Err(crate::blocking_pool::JoinError)));
                }
                Poll::Pending => {}
            }
        }

        if this.set.receivers.is_empty() {
            return Poll::Ready(None);
        }

        // Check if any receiver completed between registration and scan.
        // Re-scan with a fresh waker registration.
        let len = this.set.receivers.len();
        for i in 0..len {
            let rx = &mut this.set.receivers[i];
            let rx_pin = unsafe { Pin::new_unchecked(rx) };
            match rx_pin.poll(cx) {
                Poll::Ready(Ok(val)) => {
                    this.set.receivers.remove(i);
                    return Poll::Ready(Some(val));
                }
                Poll::Ready(Err(_)) => {
                    this.set.receivers.remove(i);
                    return Poll::Ready(Some(Err(crate::blocking_pool::JoinError)));
                }
                Poll::Pending => {}
            }
        }

        Poll::Pending
    }
}
