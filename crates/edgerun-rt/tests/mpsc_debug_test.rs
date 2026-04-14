// Debug mpsc with default worker count (multi-threaded).
use edgerun_rt::{Runtime, mpsc};
use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn mpsc_debug_multi_worker() {
    eprintln!("  parallelism={}", std::thread::available_parallelism().unwrap().get());
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    rt.block_on(async {
        const PRODUCERS: usize = 5;
        const ITEMS: usize = 200;
        const TOTAL: usize = PRODUCERS * ITEMS;

        let (tx, mut rx) = mpsc::channel::<usize>(32);
        let tx = Arc::new(tx);
        let sent = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for p in 0..PRODUCERS {
            let tx = tx.clone();
            let s = sent.clone();
            let h = edgerun_rt::spawn(async move {
                for i in 0..ITEMS {
                    tx.send(p * 10000 + i).await.unwrap();
                    s.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(h);
        }

        let mut received = 0;
        while received < TOTAL {
            match rx.recv().await {
                Some(v) => {
                    received += 1;
                    if received % 200 == 0 {
                        eprintln!("  received {} (sent={})", received, sent.load(Ordering::SeqCst));
                    }
                }
                None => {
                    eprintln!("  channel closed after {} items!", received);
                    break;
                }
            }
        }

        eprintln!("  total received: {}", received);
        eprintln!("  total sent: {}", sent.load(Ordering::SeqCst));

        // Set a timeout.
        let timeout = edgerun_rt::sleep(Duration::from_secs(3));
        let wait_handles = async {
            for h in handles {
                h.await.expect("producer should complete");
            }
        };

        use edgerun_rt::select;
        let completed = select!(
            async { wait_handles.await; true },
            async { timeout.await; false },
        );

        assert!(completed, "producers should have completed within timeout");
        assert_eq!(received, TOTAL, "should receive all items");
    });
}
