use edgerun_rt::{AsyncReadExt, AsyncTcpListener, AsyncWriteExt, Runtime};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

static STEP: AtomicUsize = AtomicUsize::new(0);

fn step(n: usize, msg: &str) {
    STEP.store(n, Ordering::SeqCst);
    eprintln!("  [step {}] {}", n, msg);
    std::io::stderr().flush().unwrap();
}

fn main() {
    eprintln!("Starting test server on 127.0.0.1:17890");
    std::io::stderr().flush().unwrap();
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    edgerun_rt::spawn(async {
        let mut i = 0;
        loop {
            edgerun_rt::sleep(std::time::Duration::from_millis(500)).await;
            i += 1;
            eprintln!(
                "  [heartbeat {} | last_step={}]",
                i,
                STEP.load(Ordering::SeqCst)
            );
        }
    });

    rt.block_on(async {
        step(1, "block_on started");

        let listener = AsyncTcpListener::bind("127.0.0.1:17890").expect("bind failed");
        step(2, "bound, waiting for accept");

        let accept_result = listener.accept().await;
        step(
            3,
            &format!(
                "accept returned: {:?}",
                accept_result.as_ref().map(|(_, a)| a)
            ),
        );

        let (mut stream, addr) = match accept_result {
            Ok(v) => v,
            Err(e) => {
                step(99, &format!("Accept error: {e}"));
                return;
            }
        };
        step(4, &format!("Accepted from {addr}"));

        let mut buf = [0u8; 100];
        step(5, "waiting for read");
        match stream.read(&mut buf).await {
            Ok(n) => step(6, &format!("Read {n} bytes: {:?}", &buf[..n])),
            Err(e) => step(99, &format!("Read error: {e}")),
        }
    });
    step(100, "block_on completed");
}
