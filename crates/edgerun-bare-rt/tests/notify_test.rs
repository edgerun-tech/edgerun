use edgerun_bare_rt::Notify;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker, RawWaker, RawWakerVTable};

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
fn notify_new_creates_notify() {
    let notify = Notify::new();
    let _ = notify;
}

#[test]
fn notify_notify_wakes() {
    let notify = Notify::new();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    
    let mut notified = notify.notified();
    let poll = Pin::new(&mut notified).poll(cx);
    // First poll is pending since notify not called
    assert!(matches!(poll, Poll::Pending));
    
    notify.notify_one();
    
    let mut notified = notify.notified();
    let poll = Pin::new(&mut notified).poll(cx);
    // After notify, should be ready
    assert!(matches!(poll, Poll::Ready(())));
}