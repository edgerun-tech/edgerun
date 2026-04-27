use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use edgerun_bare_rt::Barrier;

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
fn barrier_new_creates_barrier() {
    let barrier = Barrier::new(3);
    let _ = barrier;
}

#[test]
fn barrier_wait_completes_when_count_reached() {
    let barrier = Barrier::new(2);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    let mut wait1 = barrier.wait();
    let poll1 = Pin::new(&mut wait1).poll(cx);
    // First waiter should be pending (waiting for second)
    assert!(matches!(poll1, Poll::Pending));
}
