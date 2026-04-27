use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, AsyncMutex};

#[test]
fn mutex_new_creates_mutex() {
    let mutex = AsyncMutex::new(42);
    let _ = mutex;
}

#[test]
fn mutex_lock_becomes_ready_if_unlocked() {
    let mutex = AsyncMutex::new(42);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let mut lock = mutex.lock();
    let poll = Pin::new(&mut lock).poll(cx);
    assert!(matches!(poll, Poll::Ready(_)));
}
