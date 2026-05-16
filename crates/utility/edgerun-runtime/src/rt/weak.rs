//! Weak reference for Arc

use alloc::sync::{Arc, Weak as StdWeak};

pub struct Weak<T> {
    inner: StdWeak<T>,
}

impl<T> Weak<T> {
    pub fn new() -> Self {
        Self {
            inner: StdWeak::new(),
        }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        self.inner.upgrade()
    }

    pub fn from_arc(arc: &Arc<T>) -> Self {
        Self {
            inner: Arc::downgrade(arc),
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Default for Weak<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for Weak<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Weak").field("inner", &self.inner).finish()
    }
}
