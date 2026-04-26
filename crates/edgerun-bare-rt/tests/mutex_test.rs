use edgerun_bare_rt::AsyncMutex;
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