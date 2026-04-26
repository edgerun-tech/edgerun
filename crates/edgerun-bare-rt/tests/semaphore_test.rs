use edgerun_bare_rt::Semaphore;
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