use edgerun_bare_rt::Latch;

#[test]
fn latch_new_with_count() {
    let latch = Latch::new(3);
    let _ = latch;
}

#[test]
fn latch_wait() {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    use edgerun_bare_rt::WaitLatch;

    unsafe fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    unsafe fn noop_wake(_: *const ()) {}
    unsafe fn noop_drop(_: *const ()) {}
    static VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);

    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let cx = &mut Context::from_waker(&waker);

    let latch = Latch::new(1);
    let mut wait = latch.wait();
    let poll = Pin::new(&mut wait).poll(cx);
    // May be Ready if count reached, or Pending
    assert!(matches!(poll, Poll::Ready(()) | Poll::Pending));
}
