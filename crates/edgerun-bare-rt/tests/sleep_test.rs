use edgerun_bare_rt::{Sleep, Instant};
use core::time::Duration;
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
fn sleep_completed_if_past_deadline() {
    let deadline = Instant::now() - Duration::from_millis(1);
    let mut sleep = Sleep::new(deadline);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    
    let result = Pin::new(&mut sleep).poll(cx);
    assert!(matches!(result, Poll::Ready(())));
}

#[test]
fn sleep_pending_if_future_deadline() {
    let deadline = Instant::now() + Duration::from_secs(86400);
    let mut sleep = Sleep::new(deadline);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    
    let result = Pin::new(&mut sleep).poll(cx);
    assert!(matches!(result, Poll::Pending));
}

#[test]
fn sleep_deadline_method() {
    let deadline = Instant::now() + Duration::from_millis(100);
    let sleep = Sleep::new(deadline);
    assert_eq!(sleep.deadline().deadline_tsc(), deadline.deadline_tsc());
}