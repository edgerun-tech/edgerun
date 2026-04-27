use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{mpsc_channel, noop_waker};

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
