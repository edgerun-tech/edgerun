use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, watch_channel};

#[test]
fn watch_notifies_on_send() {
    let (sender, mut receiver) = watch_channel(10);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    assert!(matches!(Pin::new(&mut receiver).poll(cx), Poll::Pending));

    sender.send(20);
    assert!(matches!(Pin::new(&mut receiver).poll(cx), Poll::Ready(20)));
}

#[test]
fn watch_close_allows_poll_completion() {
    let (sender, mut receiver) = watch_channel(30);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    sender.close();
    assert!(matches!(Pin::new(&mut receiver).poll(cx), Poll::Ready(30)));
}
