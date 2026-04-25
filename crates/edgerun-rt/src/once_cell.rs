//! `OnceCell` — cell that can be initialized exactly once.

use crate::sync::Mutex;
use std::task::{Context, Poll, Waker};

enum State<T> {
    Uninit,
    Initializing,
    Ready(T),
}

/// A cell that can be initialized exactly once.
pub struct OnceCell<T> {
    state: Mutex<State<T>>,
    waker: Mutex<Option<Waker>>,
}

impl<T> Default for OnceCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> OnceCell<T> {
    /// Creates a new, uninitialized `OnceCell`.
    pub const fn new() -> Self {
        Self {
            state: Mutex::new(State::Uninit),
            waker: Mutex::new(None),
        }
    }

    /// Returns `true` if initialized.
    pub fn initialized(&self) -> bool {
        matches!(*self.state.lock(), State::Ready(_))
    }

    /// Gets a reference, or `None` if not initialized.
    ///
    /// # Safety
    /// Returns a reference that lives as long as `self`.
    pub fn get(&self) -> Option<&T> {
        let guard = self.state.lock();
        if let State::Ready(_) = &*guard {
            // Safety: The value lives as long as the OnceCell.
            // We read through the locked guard to ensure visibility,
            // but return a reference without the guard. This is safe
            // because OnceCell never uninitializes a Ready state.
            unsafe {
                let ptr: *const T = match &*guard {
                    State::Ready(v) => v,
                    _ => unreachable!(),
                };
                Some(&*ptr)
            }
        } else {
            None
        }
    }

    /// Gets a mutable reference, or `None` if not initialized.
    pub fn get_mut(&mut self) -> Option<&mut T> {
        let state = self.state.get_mut();
        if let State::Ready(v) = state {
            Some(v)
        } else {
            None
        }
    }

    /// Takes the value out, if initialized.
    pub fn into_inner(self) -> Option<T> {
        match self.state.into_inner() {
            State::Ready(v) => Some(v),
            _ => None,
        }
    }

    /// Gets or initializes the value synchronously.
    ///
    /// The closure is called exactly once. If called concurrently,
    /// this panics.
    pub fn get_or_init(&self, f: impl FnOnce() -> T) -> &T {
        {
            let s = self.state.lock();
            if let State::Ready(_) = &*s {
                unsafe {
                    let ptr: *const T = match &*s {
                        State::Ready(v) => v,
                        _ => unreachable!(),
                    };
                    return &*ptr;
                }
            }
        }

        let mut s = self.state.lock();
        match &*s {
            State::Ready(_) => unsafe {
                let ptr: *const T = match &*s {
                    State::Ready(v) => v,
                    _ => unreachable!(),
                };
                return &*ptr;
            },
            State::Initializing => panic!("OnceCell: concurrent init"),
            State::Uninit => {}
        }
        *s = State::Initializing;
        drop(s);

        let value = f();

        let mut s = self.state.lock();
        *s = State::Ready(value);
        let ptr: *const T = match &*s {
            State::Ready(v) => v,
            _ => unreachable!(),
        };

        // Wake any async waiter.
        if let Some(w) = self.waker.lock().take() {
            w.wake();
        }

        unsafe { &*ptr }
    }

    /// Waits for the cell to be initialized.
    pub fn wait(&self) -> WaitUntilReady<'_, T> {
        WaitUntilReady { cell: self }
    }
}

/// Future that resolves when the [`OnceCell`] is initialized.
pub struct WaitUntilReady<'a, T> {
    cell: &'a OnceCell<T>,
}

impl<T> std::future::Future for WaitUntilReady<'_, T> {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.cell.initialized() {
            return Poll::Ready(());
        }
        self.cell.waker.lock().replace(cx.waker().clone());
        // Re-check after registering.
        if self.cell.initialized() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
