use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{noop_waker, Signal, SignalKind};

#[test]
fn signal_kind_variants() {
    let _ = SignalKind::Interrupt;
    let _ = SignalKind::Termination;
    let _ = SignalKind::Child;
}

#[test]
fn signal_wakes_on_raise_and_rearms() {
    let mut signal = Signal::new();
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(matches!(Pin::new(&mut signal).poll(&mut cx), Poll::Pending));

    signal.raise();
    assert!(matches!(
        Pin::new(&mut signal).poll(&mut cx),
        Poll::Ready(())
    ));

    signal.clear();
    assert!(matches!(Pin::new(&mut signal).poll(&mut cx), Poll::Pending));
}

#[test]
fn signal_wakes_multiple_waiters() {
    let mut signal = Signal::new();
    let waker = noop_waker();
    let mut cx1 = Context::from_waker(&waker);
    let mut cx2 = Context::from_waker(&waker);

    assert!(matches!(
        Pin::new(&mut signal).poll(&mut cx1),
        Poll::Pending
    ));
    assert!(matches!(
        Pin::new(&mut signal).poll(&mut cx2),
        Poll::Pending
    ));

    signal.raise();

    assert!(matches!(
        Pin::new(&mut signal).poll(&mut cx1),
        Poll::Ready(())
    ));
    assert!(matches!(
        Pin::new(&mut signal).poll(&mut cx2),
        Poll::Ready(())
    ));
}
