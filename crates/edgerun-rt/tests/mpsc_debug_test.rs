// Debug mpsc with #[test] harness (standard).
use edgerun_rt::{Runtime, mpsc};
use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn mpsc_debug_single_worker() {
    let rt = Runtime::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(1)
        .build()
        .unwrap();

    rt.block_on(async {
        let (tx, mut rx) = mpsc::channel::<usize>(1);
        let tx = Arc::new(tx);
        let sent = Arc::new(AtomicUsize::new(0));

        // 2 producers, 10 items each.
        let mut handles = vec![];
        for p in 0..2 {
            let tx = tx.clone();
            let s = sent.clone();
            let h = edgerun_rt::spawn(async move {
                for i in 0..10 {
                    tx.send(p * 100 + i).await.unwrap();
                    s.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(h);
        }

        let mut received = 0;
        let expected = 20;
        loop {
            // Use select with timeout.
            use edgerun_rt::select;
            let got = select!(
                async { rx.recv().await },
                async { edgerun_rt::sleep(Duration::from_secs(5)).await; None },
            );
            match got {
                Some(_) => { received += 1; }
                None => break,
            }
            if received >= expected { break; }
        }

        eprintln!("received={} sent={}", received, sent.load(Ordering::SeqCst));
        assert_eq!(received, expected);
        for h in handles {
            h.await.expect("producer should complete");
        }
    });
}
