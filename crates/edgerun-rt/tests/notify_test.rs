use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, Notify};

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
