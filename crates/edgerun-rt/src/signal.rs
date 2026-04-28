//! Signal/interrupt handling for bare-metal

use crate::sync::Mutex;
use core::pin::Pin;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{Context, Poll, Waker};

pub struct Signal {
    raised: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

impl Signal {
    pub fn new() -> Self {
        Self {
            raised: AtomicBool::new(false),
            waker: Mutex::new(None),
        }
    }

    pub fn raised(&self) -> bool {
        self.raised.load(Ordering::Acquire)
    }

    pub fn raise(&self) {
        self.raised.store(true, Ordering::Release);
        let waker = {
            let mut guard = self.waker.lock();
            guard.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    pub fn clear(&self) {
        self.raised.store(false, Ordering::Release);
    }
}

impl core::future::Future for Signal {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.raised() {
            Poll::Ready(())
        } else {
            let mut guard = self.waker.lock();
            if self.raised() {
                return Poll::Ready(());
            }
            *guard = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct SignalHandler<F> {
    signal: Signal,
    handler: F,
}

impl<F> SignalHandler<F> {
    pub fn new(signal: Signal, handler: F) -> Self {
        Self { signal, handler }
    }

    pub fn poll(&mut self)
    where
        F: FnMut(),
    {
        if self.signal.raised() {
            (self.handler)();
            self.signal.clear();
        }
    }
}

pub fn ctrl_c() -> Signal {
    Signal::new()
}
pub fn alarm() -> Signal {
    Signal::new()
}
pub fn usr1() -> Signal {
    Signal::new()
}
pub fn usr2() -> Signal {
    Signal::new()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalKind {
    Interrupt,
    Termination,
    Child,
}

pub struct CtrlC(Signal);

impl CtrlC {
    pub fn new() -> Self {
        Self(Signal::new())
    }

    pub fn raised(&self) -> bool {
        self.0.raised()
    }
}

impl Default for CtrlC {
    fn default() -> Self {
        Self::new()
    }
}
