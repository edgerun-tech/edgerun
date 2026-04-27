use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, YieldNow};

#[test]
fn yield_first_pending() {
    let mut fut = YieldNow::new();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(fut.yielded);
}

#[test]
fn yield_second_ready() {
    let mut fut = YieldNow::new();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Ready(())));
}

#[test]
fn yield_immediately_ready_if_already_yielded() {
    let mut fut = YieldNow::new();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    // First poll - yields
    let _ = Pin::new(&mut fut).poll(cx);

    // After yielded, next poll immediately returns Ready
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Ready(())));
}
