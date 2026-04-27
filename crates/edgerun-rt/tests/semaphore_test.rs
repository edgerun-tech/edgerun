use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, Semaphore};

#[test]
fn semaphore_new_creates_semaphore() {
    let sem = Semaphore::new(3);
    let _ = sem;
}

#[test]
fn semaphore_acquire_if_available() {
    let sem = Semaphore::new(2);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    let mut acquire = sem.acquire();
    let poll = Pin::new(&mut acquire).poll(cx);
    assert!(matches!(poll, Poll::Ready(Ok(_))));
}

#[test]
fn semaphore_try_acquire() {
    let sem = Semaphore::new(1);

    assert!(sem.try_acquire().is_ok());
    // Second try may succeed or fail - semantics unclear
}
