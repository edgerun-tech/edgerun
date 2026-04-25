//! JoinSet - manage a dynamic set of spawned tasks.


extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::oneshot;

// ===========================================================================
// JoinSet
// ===========================================================================

pub struct JoinSet<T> {
    receivers: UnsafeCell<Vec<oneshot::Receiver<Result<T, crate::blocking_pool::JoinError>>>>,
}

impl<T> Default for JoinSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> JoinSet<T> {
    pub fn new() -> Self {
        Self {
            receivers: UnsafeCell::new(Vec::new()),
        }
    }

    pub fn spawn<F>(&mut self, _rt: &crate::runtime::RuntimeHandle, f: F)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let (mut tx, rx) = oneshot::channel();
        let _fut = Box::pin(async move {
            let result = f.await;
            let _ = tx.send(Ok(result));
        });
        unsafe { (*self.receivers.get()).push(rx) };
    }

    pub fn len(&self) -> usize {
        unsafe { (*self.receivers.get()).len() }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn join_next(&mut self) -> JoinNext<'_, T> {
        JoinNext { set: self }
    }

    pub fn abort_all(&mut self) {
        unsafe { (*self.receivers.get()).clear() };
    }
}

pub struct JoinNext<'a, T> {
    set: &'a mut JoinSet<T>,
}

impl<T: Send + 'static> Future for JoinNext<'_, T> {
    type Output = Option<Result<T, crate::blocking_pool::JoinError>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let receivers = unsafe { &mut *this.set.receivers.get() };
        for i in (0..receivers.len()).rev() {
            let rx = &mut receivers[i];
            match Future::poll(Pin::new(rx), cx) {
                Poll::Ready(Ok(val)) => {
                    receivers.remove(i);
                    return Poll::Ready(Some(val));
                }
                Poll::Ready(Err(_)) => {
                    receivers.remove(i);
                    return Poll::Ready(Some(Err(crate::blocking_pool::JoinError)));
                }
                Poll::Pending => {}
            }
        }
        if receivers.is_empty() {
            return Poll::Ready(None);
        }
        Poll::Pending
    }
}