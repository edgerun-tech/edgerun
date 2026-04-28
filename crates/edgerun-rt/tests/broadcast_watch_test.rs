use edgerun_rt::{broadcast, noop_waker};
use core::pin::Pin;
use core::task::{Context, Poll};

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
