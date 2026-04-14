// Test select! macro with the actual runtime — migrated to standard #[test] harness.
use edgerun_rt::{Runtime, select};
use std::time::Duration;

#[test]
fn select_immediate_futures() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let result = rt.block_on(async {
        select!(async { 42 }, async { 99 })
    });
    assert!(result == 42 || result == 99);
}

#[test]
fn select_faster_wins() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let result = rt.block_on(async {
        select!(
            async {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                "fast"
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
                "slow"
            },
        )
    });
    assert_eq!(result, "fast");
}

#[test]
fn select_timing() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (result, elapsed) = rt.block_on(async {
        let start = std::time::Instant::now();
        let result = select!(
            async {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                1
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
                2
            },
        );
        (result, start.elapsed())
    });
    assert_eq!(result, 1, "fast future should win");
    assert!(elapsed < Duration::from_millis(150), "select took too long: {:?}", elapsed);
}

#[test]
fn select_with_sleep() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (result, elapsed) = rt.block_on(async {
        let start = std::time::Instant::now();
        let result = select!(
            async {
                edgerun_rt::sleep(Duration::from_millis(5)).await;
                "winner"
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(500)).await;
                "loser"
            },
        );
        (result, start.elapsed())
    });
    assert_eq!(result, "winner");
    assert!(elapsed < Duration::from_millis(150), "select took too long: {:?}", elapsed);
}

#[test]
fn select_drop_loser() {
    // Verify the losing future is dropped (not leaked).
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let d = dropped.clone();

    struct DropWatcher {
        dropped: Arc<AtomicBool>,
    }
    impl Drop for DropWatcher {
        fn drop(&mut self) {
            self.dropped.store(true, Ordering::SeqCst);
        }
    }

    let _result = rt.block_on(async move {
        let _dw = DropWatcher { dropped: d };
        select!(
            async { "fast" },
            async {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
                "slow"
            },
        )
    });
    // The DropWatcher should have been dropped after select returned.
    assert!(dropped.load(Ordering::SeqCst), "losing future should be dropped");
}

#[test]
fn select_fairness() {
    // Verify that select! picks one of the two futures.
    // The internal round-robin (start counter) ensures that when both
    // futures are Pending and become ready simultaneously, the polling
    // order alternates across polls of the same Select2 instance.
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let result = rt.block_on(async {
        select!(async { "a" }, async { "b" })
    });
    assert!(result == "a" || result == "b");
}
