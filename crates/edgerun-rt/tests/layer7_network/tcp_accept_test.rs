// Minimal test: verify the runtime can accept a TCP connection and read/write.
use edgerun_rt::{spawn, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWriteExt, Runtime};
use std::sync::Arc;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        println!("Starting test server on 127.0.0.1:17890");
        let listener = AsyncTcpListener::bind("127.0.0.1:17890").expect("bind failed");
        println!("Listening. Waiting for accept...");

        let (stream, addr) = listener.accept().await.expect("accept failed");
        println!("Accepted connection from {}", addr);

        let mut stream: Arc<AsyncTcpStream> = stream;

        // Echo back: read 100 bytes, write them back
        let mut buf = [0u8; 100];
        let n = stream.read(&mut buf).await.expect("read failed");
        println!("Read {} bytes: {:?}", n, &buf[..n]);

        stream.write_all(&buf[..n]).await.expect("write failed");
        println!("Echoed back {} bytes", n);
    });
    println!("Test passed!");
}
