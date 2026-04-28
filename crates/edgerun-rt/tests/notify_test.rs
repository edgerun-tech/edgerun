use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
#[cfg(not(target_os = "none"))]
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
#[cfg(not(target_os = "none"))]
use std::task::{Wake, Waker};
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

#[test]
#[cfg(not(target_os = "none"))]
fn notify_multiple_pending_polls_do_not_duplicate_wakeups() {
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

    let notify = Notify::new();
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());
    let cx = &mut Context::from_waker(&waker);
    let mut notified = notify.notified();

    assert!(matches!(Pin::new(&mut notified).poll(cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut notified).poll(cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut notified).poll(cx), Poll::Pending));

    notify.notify_waiters();

    assert_eq!(counter.calls.load(Ordering::Relaxed), 1);
}

#[test]
#[cfg(not(target_os = "none"))]
fn notify_replaces_polling_waker_without_duplicates() {
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

    let notify = Notify::new();
    let first = Arc::new(Counter::default());
    let second = Arc::new(Counter::default());
    let first_waker = Waker::from(first.clone());
    let second_waker = Waker::from(second.clone());
    let mut notified = notify.notified();

    assert!(matches!(
        Pin::new(&mut notified).poll(&mut Context::from_waker(&first_waker)),
        Poll::Pending
    ));
    assert!(matches!(
        Pin::new(&mut notified).poll(&mut Context::from_waker(&second_waker)),
        Poll::Pending
    ));

    notify.notify_waiters();

    assert_eq!(first.calls.load(Ordering::Relaxed), 0);
    assert_eq!(second.calls.load(Ordering::Relaxed), 1);
}

#[test]
#[cfg(not(target_os = "none"))]
fn notify_dropped_waiter_is_removed() {
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

    let notify = Notify::new();
    let counter = Arc::new(Counter::default());
    {
        let waker = Waker::from(counter.clone());
        let mut notified = notify.notified();
        assert!(matches!(
            Pin::new(&mut notified).poll(&mut Context::from_waker(&waker)),
            Poll::Pending
        ));
    }

    notify.notify_waiters();

    assert_eq!(counter.calls.load(Ordering::Relaxed), 0);
}
