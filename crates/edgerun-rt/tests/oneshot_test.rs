use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use edgerun_rt::channel;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::Wake;

#[test]
fn oneshot_await_receiver() {
    let rt = edgerun_rt::Builder::new_multi_thread().build().unwrap();

    async fn demo() -> i32 {
        let (mut sender, receiver) = channel();
        sender.send(42).unwrap();
        receiver.await.unwrap()
    }

    let result = rt.block_on(Box::pin(demo()));
    assert_eq!(result, 42);
}

#[test]
fn oneshot_notifies_waiter_and_replaces_waker() {
    #[derive(Default)]
    struct Counter {
        calls: AtomicUsize,
    }

    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    let (mut sender, mut receiver) = channel();
    let first = Arc::new(Counter::default());
    let second = Arc::new(Counter::default());
    let first_waker = Waker::from(first.clone());
    let second_waker = Waker::from(second.clone());
    let mut cx1 = Context::from_waker(&first_waker);
    let mut cx2 = Context::from_waker(&second_waker);

    assert!(matches!(
        Pin::new(&mut receiver).poll(&mut cx1),
        Poll::Pending
    ));
    assert!(matches!(
        Pin::new(&mut receiver).poll(&mut cx2),
        Poll::Pending
    ));

    sender.send(7).unwrap();
    assert!(matches!(
        Pin::new(&mut receiver).poll(&mut cx2),
        Poll::Ready(Ok(7))
    ));

    assert_eq!(first.calls.load(Ordering::Relaxed), 0);
    assert_eq!(second.calls.load(Ordering::Relaxed), 1);

    drop(sender);
}

#[test]
fn oneshot_dropped_waiter_is_removed() {
    #[derive(Default)]
    struct Counter {
        calls: AtomicUsize,
    }

    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.calls.fetch_add(1, Ordering::Relaxed);
        }
    }

    let (sender, receiver) = channel::<i32>();
    let counter = Arc::new(Counter::default());
    {
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);
        let mut dropped = receiver;
        assert!(matches!(
            Pin::new(&mut dropped).poll(&mut cx),
            Poll::Pending
        ));
    }

    drop(sender);
    assert_eq!(counter.calls.load(Ordering::Relaxed), 0);
}
