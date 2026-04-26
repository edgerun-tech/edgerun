use edgerun_bare_rt::mpsc_channel;
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
fn mpsc_send_and_recv() {
    let (sender, receiver) = mpsc_channel::<i32>(10);
    
    sender.try_send(42).unwrap();
    
    let mut recv = receiver.recv();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let poll = Pin::new(&mut recv).poll(cx);
    assert!(matches!(poll, Poll::Ready(Some(42))));
}

#[test]
fn mpsc_try_send_blocks() {
    let (sender, _receiver) = mpsc_channel::<i32>(1);
    
    assert!(sender.try_send(1).is_ok());
    assert!(sender.try_send(2).is_err());
}

#[test]
fn mpsc_closed_returns_none() {
    let (sender, receiver) = mpsc_channel::<i32>(10);
    
    drop(sender);
    
    let mut recv = receiver.recv();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let poll = Pin::new(&mut recv).poll(cx);
    assert!(matches!(poll, Poll::Ready(None)));
}