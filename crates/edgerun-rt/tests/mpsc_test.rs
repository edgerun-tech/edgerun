use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{mpsc_channel, run_queue, noop_waker};

#[test]
fn mpsc_send_and_recv() {
    let (sender, receiver) = mpsc_channel::<i32>(10);

    sender.try_send(42).unwrap();

    let mut recv = receiver.recv();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let poll = Pin::new(&mut recv).poll(cx);
    assert!(matches!(poll, Poll::Ready(Some(42))));
}

#[test]
fn mpsc_try_send_blocks() {
    let (sender, _receiver) = mpsc_channel::<i32>(1);

    assert!(sender.try_send(1).is_ok());
    assert!(sender.try_send(2).is_err());
}

#[test]
fn mpsc_closed_returns_none() {
    let (sender, receiver) = mpsc_channel::<i32>(10);

    drop(sender);

    let mut recv = receiver.recv();
    let waker = noop_waker();
    let cx = &mut Context::from_waker(&waker);
    let poll = Pin::new(&mut recv).poll(cx);
    assert!(matches!(poll, Poll::Ready(None)));
}

#[test]
fn mpsc_supports_concurrent_senders() {
    let (sender, receiver) = mpsc_channel::<usize>(0);
    let mut handles = Vec::new();

    for thread_idx in 0..8 {
        let sender = sender.clone();
        handles.push(std::thread::spawn(move || {
            for value in 0..128 {
                sender.try_send(thread_idx * 128 + value).unwrap();
            }
        }));
    }
    drop(sender);

    for handle in handles {
        handle.join().unwrap();
    }

    let mut values = Vec::new();
    while let Ok(value) = receiver.try_recv() {
        values.push(value);
    }
    values.sort_unstable();

    assert_eq!(values.len(), 8 * 128);
    assert_eq!(values[0], 0);
    assert_eq!(values[values.len() - 1], 8 * 128 - 1);
}

#[test]
fn mpsc_multiple_waiting_receivers_are_not_starved() {
    let (sender, receiver) = mpsc_channel::<i32>(8);
    let mut recv1 = receiver.recv();
    let mut recv2 = receiver.recv();

    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    assert!(matches!(Pin::new(&mut recv1).poll(&mut cx), Poll::Pending));
    assert!(matches!(Pin::new(&mut recv2).poll(&mut cx), Poll::Pending));

    sender.try_send(10).unwrap();

    let first = Pin::new(&mut recv1).poll(&mut cx);
    let second = Pin::new(&mut recv2).poll(&mut cx);
    let first_pending = match (first, second) {
        (Poll::Ready(Some(v)), Poll::Pending) => {
            assert_eq!(v, 10);
            true
        }
        (Poll::Pending, Poll::Ready(Some(v))) => {
            assert_eq!(v, 10);
            false
        }
        _ => panic!("expected one ready receiver and one pending"),
    };

    sender.try_send(11).unwrap();
    if first_pending {
        assert!(matches!(
            Pin::new(&mut recv2).poll(&mut cx),
            Poll::Ready(Some(11))
        ));
        assert!(matches!(
            Pin::new(&mut recv1).poll(&mut cx),
            Poll::Pending
        ));
    } else {
        assert!(matches!(
            Pin::new(&mut recv1).poll(&mut cx),
            Poll::Ready(Some(11))
        ));
        assert!(matches!(
            Pin::new(&mut recv2).poll(&mut cx),
            Poll::Pending
        ));
    }
}

#[test]
fn mpsc_multiple_senders_wait_while_full() {
    let (sender, receiver) = mpsc_channel::<i32>(1);
    sender.try_send(1).unwrap();

    let first = {
        let sender = sender.clone();
        edgerun_rt::spawn(async move { sender.send(2).await.unwrap() })
    };
    let second = {
        let sender = sender.clone();
        edgerun_rt::spawn(async move { sender.send(3).await.unwrap() })
    };

    run_queue();
    assert!(!first.is_finished());
    assert!(!second.is_finished());

    assert_eq!(receiver.try_recv().unwrap(), 1);
    run_queue();
    assert!(first.is_finished() as usize + second.is_finished() as usize > 0);

    assert!(receiver.try_recv().is_ok());
    run_queue();
    assert!(first.is_finished());
    assert!(second.is_finished());
}
