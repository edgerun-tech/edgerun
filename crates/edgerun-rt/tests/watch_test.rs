use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, watch_channel};
#[cfg(not(target_os = "none"))]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
#[cfg(not(target_os = "none"))]
use std::task::{Wake, Waker};

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

#[test]
#[cfg(not(target_os = "none"))]
fn watch_replaces_polling_waker() {
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

    let (sender, mut receiver) = watch_channel(5);
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

    sender.send(6);

    assert!(matches!(
        Pin::new(&mut receiver).poll(&mut cx2),
        Poll::Ready(6)
    ));
    assert_eq!(first.calls.load(Ordering::Relaxed), 0);
    assert_eq!(second.calls.load(Ordering::Relaxed), 1);
}

#[test]
#[cfg(not(target_os = "none"))]
fn watch_drop_removes_waiter() {
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

    let (sender, receiver) = watch_channel(10);
    let counter = Arc::new(Counter::default());
    {
        let mut dropped = receiver;
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            Pin::new(&mut dropped).poll(&mut cx),
            Poll::Pending
        ));
    }

    sender.close();

    assert_eq!(counter.calls.load(Ordering::Relaxed), 0);
}
