use core::future::Future;
use core::task::Poll;
use core::pin::Pin;
use edgerun_bare_rt::YieldNow;

#[test]
fn yield_first_pending() {
    use core::task::Poll;
    use core::pin::Pin;
    use core::task::Waker;
    use core::task::RawWaker;
    use core::task::RawWakerVTable;
    use core::task::Context;
    
    unsafe fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    unsafe fn noop_wake(_: *const ()) {}
    unsafe fn noop_drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);
    
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let cx = &mut Context::from_waker(&waker);
    let mut fut = YieldNow::new();
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(fut.yielded);
}

#[test]
fn yield_second_ready() {
    use core::task::Poll;
    use core::pin::Pin;
    use core::task::Waker;
    use core::task::RawWaker;
    use core::task::RawWakerVTable;
    use core::task::Context;
    
    unsafe fn noop_clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    unsafe fn noop_wake(_: *const ()) {}
    unsafe fn noop_drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop_wake, noop_wake, noop_drop);
    
    let waker = unsafe { Waker::from_raw(RawWaker::new(core::ptr::null(), &VTABLE)) };
    let cx = &mut Context::from_waker(&waker);
    let mut fut = YieldNow::new();
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Ready(())));
}