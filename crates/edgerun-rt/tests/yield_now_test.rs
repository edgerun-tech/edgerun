use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use edgerun_rt::YieldNow;

fn noop_waker() -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) }
}

unsafe fn noop_clone(_: *const ()) -> RawWaker {
    RawWaker::new(core::ptr::null(), &VTABLE)
}
unsafe fn noop_wake(_: *const ()) {}
unsafe fn noop_drop(_: *const ()) {}
static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

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
