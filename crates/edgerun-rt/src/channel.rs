//! Async channels for bare-metal

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

use crate::sync::Mutex;

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        data: UnsafeCell::new(None),
        sent: AtomicBool::new(false),
        wakers: Mutex::new(Vec::new()),
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner, waker: None },
    )
}

struct Inner<T> {
    data: UnsafeCell<Option<T>>,
    sent: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
}

unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.sent.load(Ordering::Acquire) {
            return Err(SendError(value));
        }
        unsafe { *self.inner.data.get() = Some(value) };
        self.inner.sent.store(true, Ordering::Release);
        let wakers = {
            let mut wakers = self.inner.wakers.lock();
            core::mem::take(&mut *wakers)
        };
        for waker in wakers {
            waker.wake();
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.inner.sent.load(Ordering::Acquire)
    }
}

pub struct SendError<T>(pub T);

impl<T> core::fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SendError(..)")
    }
}

impl<T> core::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "channel closed")
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
    waker: Option<Waker>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct RecvError;

impl<T> Future for Receiver<T> {
    type Output = Result<T, RecvError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.inner.sent.load(Ordering::Acquire) {
            let ptr = this.inner.data.get();
            unsafe {
                if (*ptr).is_some() {
                    if let Some(waker) = this.waker.take() {
                        let mut wakers = this.inner.wakers.lock();
                        remove_waker(&mut wakers, &waker);
                    }
                    return Poll::Ready(Ok((*ptr).take().unwrap()));
                }
            }
            if let Some(waker) = this.waker.take() {
                let mut wakers = this.inner.wakers.lock();
                remove_waker(&mut wakers, &waker);
            }
            return Poll::Ready(Err(RecvError));
        }
        let should_register = match this.waker.as_ref() {
            Some(registered) => !registered.will_wake(cx.waker()),
            None => true,
        };
        if should_register {
            let mut wakers = this.inner.wakers.lock();
            if let Some(previous) = this.waker.replace(cx.waker().clone()) {
                remove_waker(&mut wakers, &previous);
            }
            register_waker(&mut wakers, this.waker.as_ref().expect("registered wakeup"));
        }
        Poll::Pending
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        if let Some(waker) = self.waker.take() {
            let mut wakers = self.inner.wakers.lock();
            remove_waker(&mut wakers, &waker);
        }
    }
}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Option<T> {
        if self.inner.sent.load(Ordering::Acquire) {
            unsafe { self.inner.data.get().as_mut()?.take() }
        } else {
            None
        }
    }

    pub fn close(&self) {
        self.inner.sent.store(true, Ordering::Release);
        let wakers = {
            let mut wakers = self.inner.wakers.lock();
            core::mem::take(&mut *wakers)
        };
        for waker in wakers {
            waker.wake();
        }
    }
}

fn register_waker(wakers: &mut Vec<Waker>, waker: &Waker) {
    if !wakers.iter().any(|registered| registered.will_wake(waker)) {
        wakers.push(waker.clone());
    }
}

fn remove_waker(wakers: &mut Vec<Waker>, waker: &Waker) {
    if let Some(pos) = wakers
        .iter()
        .position(|registered| registered.will_wake(waker))
    {
        wakers.remove(pos);
    }
}
