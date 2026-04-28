use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use edgerun_rt::{mpsc_channel, noop_waker};

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
