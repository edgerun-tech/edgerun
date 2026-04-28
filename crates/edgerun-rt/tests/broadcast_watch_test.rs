use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{broadcast, noop_waker};
#[cfg(not(target_os = "none"))]
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
#[cfg(not(target_os = "none"))]
use std::task::{Wake, Waker};

#[test]
fn broadcast_with_multiple_subscribers_and_overflow() {
    let (publisher, subscriber) = broadcast(2usize);
    let mut first = subscriber;
    let mut second = first.clone();

    publisher.send(1);
    publisher.send(2);
    assert_eq!(first.try_recv(), Some(1));
    assert_eq!(second.try_recv(), Some(1));
    assert_eq!(first.try_recv(), Some(2));

    publisher.send(3);
    assert_eq!(first.try_recv(), Some(3));
    assert_eq!(second.try_recv(), Some(2));
    assert_eq!(second.try_recv(), Some(3));
    assert_eq!(first.try_recv(), None);
}

#[test]
fn broadcast_poll_registers_new_values() {
    let (publisher, mut subscriber) = broadcast::<i32>(1);
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);

    assert!(matches!(Pin::new(&mut subscriber).poll(cx), Poll::Pending));

    publisher.send(5);
    assert!(matches!(Pin::new(&mut subscriber).poll(cx), Poll::Ready(5)));

    publisher.send(6);
    publisher.send(7);
    assert!(matches!(Pin::new(&mut subscriber).poll(cx), Poll::Ready(7)));
}

#[test]
fn broadcast_close_stops_further_sends() {
    let (publisher, mut subscriber) = broadcast::<i32>(2);

    publisher.send(1);
    publisher.close();
    publisher.send(2);

    assert_eq!(subscriber.try_recv(), Some(1));
    assert_eq!(subscriber.try_recv(), None);
}

#[test]
#[cfg(not(target_os = "none"))]
fn broadcast_notifies_waiter_and_replaces_waker() {
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

    let (publisher, mut subscriber) = broadcast::<i32>(1);
    let first = Arc::new(Counter::default());
    let second = Arc::new(Counter::default());
    let first_waker = Waker::from(first.clone());
    let second_waker = Waker::from(second.clone());
    let mut first_cx = Context::from_waker(&first_waker);
    let mut second_cx = Context::from_waker(&second_waker);

    assert!(matches!(
        Pin::new(&mut subscriber).poll(&mut first_cx),
        Poll::Pending
    ));
    assert!(matches!(
        Pin::new(&mut subscriber).poll(&mut second_cx),
        Poll::Pending
    ));

    publisher.send(1);
    assert!(matches!(
        Pin::new(&mut subscriber).poll(&mut second_cx),
        Poll::Ready(1)
    ));

    assert_eq!(first.calls.load(Ordering::Relaxed), 0);
    assert_eq!(second.calls.load(Ordering::Relaxed), 1);
}

#[test]
#[cfg(not(target_os = "none"))]
fn broadcast_drop_removes_waiter() {
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

    let (publisher, subscriber) = broadcast::<i32>(1);
    let counter = Arc::new(Counter::default());
    {
        let mut dropped = subscriber.clone();
        let waker = Waker::from(counter.clone());
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            Pin::new(&mut dropped).poll(&mut cx),
            Poll::Pending
        ));
    }

    publisher.send(10);

    assert_eq!(counter.calls.load(Ordering::Relaxed), 0);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut subscriber = subscriber;
    assert!(matches!(
        Pin::new(&mut subscriber).poll(&mut cx),
        Poll::Ready(10)
    ));
}
