use edgerun_bare_rt::{mpsc_channel, select_2};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct NoopWake;

impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

#[test]
fn select_mpsc_with_runtime() {
    let (tx, rx) = mpsc_channel(1);
    let mut recv_future = rx.recv();
    let mut other = Box::pin(async { Some(42) });

    tx.try_send(10).unwrap();

    let mut selected = select_2(&mut recv_future, other.as_mut());
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let result = Pin::new(&mut selected).poll(&mut cx);

    assert!(matches!(result, Poll::Ready(Some(10))));
}
