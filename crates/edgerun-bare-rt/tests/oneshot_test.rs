use edgerun_bare_rt::channel;
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
fn send_value_received() {
    let (mut sender, mut receiver) = channel::<i32>();
    
    sender.send(42).unwrap();
    
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let poll = Pin::new(&mut receiver).poll(cx);
    assert!(matches!(poll, Poll::Ready(Ok(42))));
}

#[test]
fn sender_takes_ownership() {
    let (mut sender, _receiver) = channel::<i32>();
    
    assert!(sender.send(1).is_ok());
    assert!(sender.send(2).is_err());
}