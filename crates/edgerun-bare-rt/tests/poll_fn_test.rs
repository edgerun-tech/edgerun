use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use edgerun_bare_rt::poll_fn;

#[test]
fn poll_fn_closure() {
    let mut done = false;
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    
    let mut future = poll_fn(|_| {
        if done { return Poll::Ready(42) }
        done = true;
        Poll::Pending
    });
    
    let result = Pin::new(&mut future).poll(&mut cx);
    assert!(matches!(result, Poll::Pending));
}