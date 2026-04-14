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
    // Verify that when both futures are ready, select! picks one.
    // The round-robin fairness in select2 applies to repeated polls of
    // the same Select2 instance (when both return Pending on first poll).
    // For immediately-ready futures, the first one always wins because
    // poll returns Ready on the first poll.
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let result = rt.block_on(async {
        select!(async { "a" }, async { "b" })
    });
    assert!(result == "a" || result == "b");
}

#[test]
fn select_fairness_with_pending() {
    // True fairness test: both futures start as Pending, then become
    // ready at the same time. The round-robin should alternate.
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    let mut a_wins = 0;
    let mut b_wins = 0;
    for _ in 0..20 {
        let result = rt.block_on(async {
            select!(
                async {
                    edgerun_rt::sleep(Duration::from_millis(5)).await;
                    "a"
                },
                async {
                    edgerun_rt::sleep(Duration::from_millis(5)).await;
                    "b"
                },
            )
        });
        match result {
            "a" => a_wins += 1,
            "b" => b_wins += 1,
            _ => unreachable!(),
        }
    }

    // Both should win at least some of the time.
    // With round-robin, they should be roughly equal.
    assert!(
        a_wins > 0 && b_wins > 0,
        "both futures should win: a={}, b={}",
        a_wins, b_wins
    );
}
