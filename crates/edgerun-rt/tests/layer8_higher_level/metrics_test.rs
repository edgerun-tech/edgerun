// Test RuntimeMetrics with the actual runtime.
use edgerun_rt::{spawn, Builder, RuntimeHandle};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    test_initial_metrics();
    test_spawn_and_complete();
    test_active_tasks();
    test_queued_tasks();
    test_abort_metrics();
    test_blocking_pool_metrics();
    test_metrics_via_handle();
    test_metrics_snapshot();
    test_many_tasks();
    println!("All RuntimeMetrics tests passed!");
}

fn test_initial_metrics() {
    println!("  test_initial_metrics...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let m = rt.metrics();
    assert_eq!(m.active_tasks(), 0);
    assert_eq!(m.queued_tasks(), 0);
    assert_eq!(m.total_spawned(), 0);
    assert_eq!(m.total_completed(), 0);
    assert_eq!(m.total_aborted(), 0);
    assert_eq!(m.blocking_threads(), 4);
    assert_eq!(m.blocking_active(), 0);
    println!("  test_initial_metrics OK");
}

fn test_spawn_and_complete() {
    println!("  test_spawn_and_complete...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let before_spawned = h.metrics().total_spawned();
        let before_completed = h.metrics().total_completed();

        let task_h = spawn(async { 42 });
        let _ = task_h.await;
        edgerun_rt::sleep(Duration::from_millis(20)).await;

        let m = h.metrics();
        assert_eq!(m.total_spawned() - before_spawned, 1);
        assert_eq!(m.total_completed() - before_completed, 1);
    });
    println!("  test_spawn_and_complete OK");
}

fn test_active_tasks() {
    println!("  test_active_tasks...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();

        let _h = spawn(async move {
            c.store(1, Ordering::SeqCst);
            edgerun_rt::sleep(Duration::from_millis(200)).await;
        });

        edgerun_rt::sleep(Duration::from_millis(50)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        let m = h.metrics();
        assert!(
            m.active_tasks() >= 1,
            "should have at least 1 active task, got {}",
            m.active_tasks()
        );
    });
    println!("  test_active_tasks OK");
}

fn test_queued_tasks() {
    println!("  test_queued_tasks...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let before = h.metrics().total_spawned();

        // Spawn tasks that complete (not yield forever).
        let mut handles = vec![];
        for i in 0..5 {
            let hh = spawn(async move {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                i
            });
            handles.push(hh);
        }

        let m = h.metrics();
        assert_eq!(m.total_spawned() - before, 5);

        for hh in handles {
            let _ = hh.await;
        }
    });
    println!("  test_queued_tasks OK");
}

fn test_abort_metrics() {
    println!("  test_abort_metrics...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let before_aborted = h.metrics().total_aborted();
        let before_completed = h.metrics().total_completed();

        let handle = spawn(async {
            edgerun_rt::sleep(Duration::from_secs(10)).await;
        });

        edgerun_rt::sleep(Duration::from_millis(20)).await;
        handle.abort();
        let _ = handle.await;
        edgerun_rt::sleep(Duration::from_millis(20)).await;

        let m = h.metrics();
        assert_eq!(m.total_aborted() - before_aborted, 1);
        assert_eq!(m.total_completed() - before_completed, 1);
    });
    println!("  test_abort_metrics OK");
}

fn test_blocking_pool_metrics() {
    println!("  test_blocking_pool_metrics...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let m = h.metrics();
        assert_eq!(m.blocking_threads(), 4);

        let started = Arc::new(AtomicUsize::new(0));
        let s = started.clone();
        let hh = h.spawn_blocking(move || {
            s.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(100));
        });

        edgerun_rt::sleep(Duration::from_millis(20)).await;
        assert_eq!(started.load(Ordering::SeqCst), 1);
        let m = h.metrics();
        assert!(
            m.blocking_active() >= 1,
            "should have active blocking task, got {}",
            m.blocking_active()
        );

        let _ = hh.await;
        let m = h.metrics();
        assert_eq!(m.blocking_active(), 0);
    });
    println!("  test_blocking_pool_metrics OK");
}

fn test_metrics_via_handle() {
    println!("  test_metrics_via_handle...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let handle = rt.handle();
    let _h = handle.spawn(async { 42 });
    let m = handle.metrics();
    assert_eq!(m.total_spawned(), 1);
    println!("  test_metrics_via_handle OK");
}

fn test_metrics_snapshot() {
    println!("  test_metrics_snapshot...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let m1 = h.metrics();
        let before = m1.total_spawned();

        let _h1 = spawn(async { 1 });
        let _h2 = spawn(async { 2 });
        let _h3 = spawn(async { 3 });

        let m2 = h.metrics();
        assert_eq!(m2.total_spawned() - before, 3);
    });
    println!("  test_metrics_snapshot OK");
}

fn test_many_tasks() {
    println!("  test_many_tasks...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let h = rt.handle();
    rt.block_on(async move {
        let before = h.metrics().total_spawned();
        let before_completed = h.metrics().total_completed();

        let mut handles = vec![];
        for i in 0..50 {
            let hh = spawn(async move {
                edgerun_rt::sleep(Duration::from_millis(5)).await;
                i
            });
            handles.push(hh);
        }
        for hh in handles {
            let _ = hh.await;
        }
        edgerun_rt::sleep(Duration::from_millis(30)).await;

        let m = h.metrics();
        assert_eq!(m.total_spawned() - before, 50);
        assert_eq!(m.total_completed() - before_completed, 50);
        assert_eq!(m.total_aborted(), 0);
    });
    println!("  test_many_tasks OK");
}
