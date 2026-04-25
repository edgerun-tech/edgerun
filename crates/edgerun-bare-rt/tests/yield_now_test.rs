#[test]
fn yield_first_pending() {
    use core::task::Poll;
    use core::pin::Pin;
    let waker = crate::waker::noop_waker();
    let cx = &mut core::task::Context::from_waker(&waker);
    let mut fut = crate::yield_now::YieldNow::new();
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(fut.yielded);
}

#[test]
fn yield_second_ready() {
    use core::task::Poll;
    use core::pin::Pin;
    let waker = crate::waker::noop_waker();
    let cx = &mut core::task::Context::from_waker(&waker);
    let mut fut = crate::yield_now::YieldNow::new();
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut fut).poll(cx), Poll::Ready(())));
}