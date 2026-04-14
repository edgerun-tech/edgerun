// Test that Runtime threads shut down cleanly when dropped (no thread leaks).
use edgerun_rt::{Builder, Runtime};
use std::time::Duration;

fn count_threads() -> usize {
    // Count threads in the current process via /proc/self/task
    let entries = std::fs::read_dir("/proc/self/task").unwrap();
    entries.count()
}

fn main() {
    test_drop_shuts_down_threads();
    test_explicit_shutdown_then_drop();
    test_multiple_runtime_drops();
    println!("All shutdown tests passed!");
}

fn test_drop_shuts_down_threads() {
    println!("  test_drop_shuts_down_threads...");
    let initial_threads = count_threads();

    {
        let _rt = Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(2)
            .build()
            .unwrap();
        // Runtime is alive — should have extra threads: reactor + 2 workers + 2 blocking
        let alive_threads = count_threads();
        assert!(
            alive_threads > initial_threads,
            "runtime should create additional threads"
        );
        // Drop without calling shutdown() — Drop impl should clean up.
    }

    // Give threads a moment to exit.
    std::thread::sleep(Duration::from_millis(200));

    let after_threads = count_threads();
    assert_eq!(
        after_threads, initial_threads,
        "all runtime threads should have exited after drop, expected {} got {}",
        initial_threads, after_threads
    );
    println!("  test_drop_shuts_down_threads OK");
}

fn test_explicit_shutdown_then_drop() {
    println!("  test_explicit_shutdown_then_drop...");
    let initial_threads = count_threads();

    {
        let rt = Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .unwrap();
        rt.shutdown();
        // Explicit shutdown called — threads should be joining now.
    }
    // Dropping after shutdown should be a no-op (idempotent).

    std::thread::sleep(Duration::from_millis(200));

    let after_threads = count_threads();
    assert_eq!(
        after_threads, initial_threads,
        "threads should have exited after explicit shutdown + drop"
    );
    println!("  test_explicit_shutdown_then_drop OK");
}

fn test_multiple_runtime_drops() {
    println!("  test_multiple_runtime_drops...");
    let initial_threads = count_threads();

    for i in 0..5 {
        let rt = Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .unwrap();
        // Use the runtime briefly.
        rt.block_on(async {
            edgerun_rt::sleep(Duration::from_millis(10)).await;
        });
        // Drop it — should clean up all threads.
        drop(rt);

        std::thread::sleep(Duration::from_millis(100));
        let current = count_threads();
        assert_eq!(
            current, initial_threads,
            "iteration {}: threads should return to baseline after drop, got {}",
            i, current
        );
    }
    println!("  test_multiple_runtime_drops OK");
}
