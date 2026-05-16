use std::sync::Arc;
use std::sync::Mutex;

use edgerun_tokio::sync::mpsc;
use edgerun_tokio::task;

pub use mpsc::SendError;
pub use mpsc::TryRecvError;
pub use mpsc::TrySendError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecvError;

impl std::fmt::Display for RecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("receiving from a closed channel")
    }
}

impl std::error::Error for RecvError {}

pub struct Sender<T> {
    inner: mpsc::Sender<T>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct Receiver<T> {
    inner: Arc<Mutex<mpsc::Receiver<T>>>,
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub fn bounded<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel(cap);
    (
        Sender { inner: tx },
        Receiver {
            inner: Arc::new(Mutex::new(rx)),
        },
    )
}

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>) {
    let (tx, rx) = mpsc::channel(0);
    (
        Sender { inner: tx },
        Receiver {
            inner: Arc::new(Mutex::new(rx)),
        },
    )
}

impl<T> Sender<T> {
    pub async fn send(&self, value: T) -> Result<(), SendError<T>>
    where
        T: Unpin,
    {
        self.inner.send(value).await
    }

    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        self.inner.try_send(value)
    }

    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

impl<T> Receiver<T> {
    pub async fn recv(&self) -> Result<T, RecvError> {
        loop {
            match self.try_recv() {
                Ok(value) => return Ok(value),
                Err(TryRecvError::Disconnected) => return Err(RecvError),
                Err(TryRecvError::Empty) => task::yield_now().await,
            }
        }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.inner
            .lock()
            .expect("channel receiver poisoned")
            .try_recv()
    }

    pub fn close(&self) {
        self.inner
            .lock()
            .expect("channel receiver poisoned")
            .close();
    }
}
