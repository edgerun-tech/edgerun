//! Async TLS loopback test.

use std::sync::Arc;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

static NOOP_VTABLE: &RawWakerVTable = &RawWakerVTable::new(
    |_| RawWaker::new(std::ptr::null(), NOOP_VTABLE),
    |_| {},
    |_| {},
    |_| {},
);

fn noop_waker() -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), NOOP_VTABLE)) }
}

/// Verify async TLS client can talk to async TLS server.
#[test]
fn verify_async_tls_loopback() {
    use edgerun_rt::Runtime;
    use edgerun_tls::{AsyncTlsStream, AsyncTlsServerStream};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();

    let (tx, rx) = std::sync::mpsc::channel::<std::net::TcpStream>();
    let done = Arc::new(std::sync::Barrier::new(2));
    let done_clone = done.clone();

    // Server thread: accept TCP, send it to main thread via channel
    let server_thread = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let _ = tx.send(stream);
        done_clone.wait();
    });

    // Client: connect TCP
    let tcp = std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();
    tcp.set_nonblocking(true).unwrap();

    let cert = edgerun_tls::certificate_gen::generate_self_signed(&["localhost"]).unwrap();

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let received = rt.block_on(async {
        let server_tcp = rx.recv().expect("server didn't accept");
        server_tcp.set_nonblocking(true).ok();
        let arc_server = Arc::new(edgerun_rt::AsyncTcpStream::from_std(server_tcp).unwrap());
        let arc_client = Arc::new(edgerun_rt::AsyncTcpStream::from_std(tcp).unwrap());

        // Server handshake
        let mut server_tls = AsyncTlsServerStream::accept(arc_server, &cert).await
            .expect("server handshake failed");

        // Client handshake
        let mut client_tls = AsyncTlsStream::client(arc_client, "localhost", &[], None).await
            .expect("client handshake failed");

        // Client writes
        let msg = b"HELLO FROM CLIENT";
        loop {
            let p = Pin::new(&mut client_tls).poll_write(&mut cx, msg);
            if let Poll::Ready(Ok(n)) = p {
                assert_eq!(n, msg.len());
                break;
            }
        }
        loop {
            let p = Pin::new(&mut client_tls).poll_flush(&mut cx);
            if let Poll::Ready(Ok(())) = p { break; }
        }

        // Server reads
        let mut buf = [0u8; 1024];
        loop {
            let p = Pin::new(&mut server_tls).poll_read(&mut cx, &mut buf);
            if let Poll::Ready(Ok(n)) = p {
                if n > 0 {
                    return String::from_utf8_lossy(&buf[..n]).to_string();
                }
            }
        }
    });

    done.wait();
    server_thread.join().unwrap();
    assert_eq!(received, "HELLO FROM CLIENT");
}
