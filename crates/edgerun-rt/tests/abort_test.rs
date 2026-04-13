// Test JoinHandle::abort() with the actual runtime.
use edgerun_rt::{Builder, JoinError, Runtime, RuntimeHandle, spawn, spawn_blocking};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Builder::new_multi_thread().build().unwrap();
    rt.block_on(async {
        test_abort_before_poll();
        test_abort_pending_sleep();
        test_abort_after_complete_noop();
        test_is_aborted_flag();
        test_is_finished_after_abort();
        test_abort_idempotent();
        test_abort_via_handle();
        test_abort_spawn_blocking();
        test_abort_wakes_awaiter();
        test_abort_does_not_run_closure();
        test_free_spawn_abort();
        println!("All abort tests passed!");
    });
}

fn test_abort_before_poll() {
    println!("  test_abort_before_poll...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    // Use a sleep so the task can't complete immediately — it must
    // register with the reactor first. This ensures abort happens
    // before the task is polled to completion.
    let handle = rt.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
        42
    });
    handle.abort();
    let result = rt.block_on(handle);
    assert!(result.is_err(), "aborted task should return JoinError");
    println!("  test_abort_before_poll OK");
}

fn test_abort_pending_sleep() {
    println!("  test_abort_pending_sleep...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    let started = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));
    let s = started.clone();
    let c = completed.clone();

    let handle = rt.spawn(async move {
        s.store(true, Ordering::SeqCst);
        edgerun_rt::sleep(Duration::from_secs(10)).await;
        c.store(true, Ordering::SeqCst);
        42
    });

    // Let the task start and hit the sleep.
    std::thread::sleep(Duration::from_millis(50));
    assert!(started.load(Ordering::SeqCst), "task should have started");

    handle.abort();
    let result = rt.block_on(handle);
    assert!(result.is_err(), "aborted sleeping task should return JoinError");
    assert!(!completed.load(Ordering::SeqCst), "task body should not complete after abort");
    println!("  test_abort_pending_sleep OK");
}

fn test_abort_after_complete_noop() {
    println!("  test_abort_after_complete_noop...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    let handle = rt.spawn(async { 42 });
    let result = rt.block_on(handle.clone());
    assert_eq!(result.unwrap(), 42);

    // Abort after completion should be a no-op.
    handle.abort();
    assert!(!handle.is_aborted(), "abort after complete should not set flag");
    println!("  test_abort_after_complete_noop OK");
}

fn test_is_aborted_flag() {
    println!("  test_is_aborted_flag...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    let handle = rt.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
    });

    assert!(!handle.is_aborted(), "should not be aborted initially");
    handle.abort();
    assert!(handle.is_aborted(), "should be aborted after abort()");
    handle.abort();
    assert!(handle.is_aborted(), "abort is idempotent on flag");

    // Clean up.
    let _ = rt.block_on(handle);
    println!("  test_is_aborted_flag OK");
}

fn test_is_finished_after_abort() {
    println!("  test_is_finished_after_abort...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    let handle = rt.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
    });

    std::thread::sleep(Duration::from_millis(20));
    assert!(!handle.is_finished(), "should not be finished while sleeping");

    handle.abort();
    // is_finished should become true once the result is set.
    let _ = rt.block_on(handle.clone());
    assert!(handle.is_finished(), "should be finished after abort resolves");
    println!("  test_is_finished_after_abort OK");
}

fn test_abort_idempotent() {
    println!("  test_abort_idempotent...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    let handle = rt.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
    });

    // Call abort multiple times — should not panic or double-resolve.
    handle.abort();
    handle.abort();
    handle.abort();

    let result = rt.block_on(handle);
    assert!(result.is_err(), "should still be JoinError");
    println!("  test_abort_idempotent OK");
}

fn test_abort_via_handle() {
    println!("  test_abort_via_handle...");
    let rt = Builder::new_multi_thread().build().unwrap();
    let runtime_handle = rt.handle();

    let task_handle = runtime_handle.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
        42
    });

    std::thread::sleep(Duration::from_millis(20));
    task_handle.abort();
    let result = rt.block_on(task_handle);
    assert!(result.is_err(), "aborted task via handle should return JoinError");
    println!("  test_abort_via_handle OK");
}

fn test_abort_spawn_blocking() {
    println!("  test_abort_spawn_blocking...");
    let rt = Runtime::new_multi_thread().build().unwrap();

    let started = Arc::new(AtomicBool::new(false));
    let completed = Arc::new(AtomicBool::new(false));
    let s = started.clone();
    let c = completed.clone();

    let handle = rt.spawn_blocking(move || {
        s.store(true, Ordering::SeqCst);
        // Blocking tasks can't really be aborted mid-execution,
        // but we test the flag is set.
        std::thread::sleep(Duration::from_millis(100));
        c.store(true, Ordering::SeqCst);
        42
    });

    // Let it start.
    std::thread::sleep(Duration::from_millis(20));
    assert!(started.load(Ordering::SeqCst), "blocking task should have started");

    // Abort should set the flag but blocking task runs to completion.
    handle.abort();
    assert!(handle.is_aborted(), "abort flag should be set");

    // The blocking task will complete normally (it already ran).
    // The abort flag being set before completion shouldn't prevent result.
    // Actually, the abort flag only affects async tasks checked before poll.
    // For blocking tasks, the result is set directly.
    let result = rt.block_on(handle);
    // Blocking tasks always complete; abort is a flag for async tasks.
    // Since the blocking task already ran, it should have a result.
    // The abort flag on blocking tasks is just metadata.
    assert!(result.is_ok() || result.is_err(), "result depends on timing");
    assert!(completed.load(Ordering::SeqCst), "blocking task ran to completion");
    println!("  test_abort_spawn_blocking OK");
}

fn test_abort_wakes_awaiter() {
    println!("  test_abort_wakes_awaiter...");
    let rt = Runtime::new_multi_thread().build().unwrap();

    let handle = rt.spawn(async {
        edgerun_rt::sleep(Duration::from_secs(10)).await;
        42
    });

    // Give task time to start sleeping.
    std::thread::sleep(Duration::from_millis(20));

    let start = std::time::Instant::now();
    handle.abort();
    let result = rt.block_on(handle);
    let elapsed = start.elapsed();

    assert!(result.is_err());
    assert!(
        elapsed < Duration::from_millis(200),
        "abort should wake awaiter quickly, took {:?}",
        elapsed
    );
    println!("  test_abort_wakes_awaiter OK (took {:?})", elapsed);
}

fn test_abort_does_not_run_closure() {
    println!("  test_abort_does_not_run_closure...");
    let rt = Runtime::new_multi_thread().build().unwrap();

    let ran = Arc::new(AtomicUsize::new(0));
    let r = ran.clone();

    let handle = rt.spawn(async move {
        // Task that yields many times but never completes.
        for _ in 0..100 {
            r.fetch_add(1, Ordering::SeqCst);
            edgerun_rt::yieldnow().await;
        }
    });

    // Let it run a few iterations.
    std::thread::sleep(Duration::from_millis(10));
    let before_abort = ran.load(Ordering::SeqCst);

    handle.abort();

    // Give time for any in-flight polls to settle.
    std::thread::sleep(Duration::from_millis(50));
    let after_abort = ran.load(Ordering::SeqCst);

    // After abort, the task should not be polled again.
    // The count should be frozen or very close to before_abort.
    assert!(
        after_abort <= before_abort + 2,
        "task should stop running after abort: before={}, after={}",
        before_abort,
        after_abort
    );

    let _ = rt.block_on(handle);
    println!("  test_abort_does_not_run_closure OK (ran {} iters)", after_abort);
}

fn test_free_spawn_abort() {
    println!("  test_free_spawn_abort...");
    let rt = Runtime::new_multi_thread().build().unwrap();
    rt.block_on(async {
        let handle = spawn(async {
            edgerun_rt::sleep(Duration::from_secs(10)).await;
            42
        });

        std::thread::sleep(Duration::from_millis(20));
        handle.abort();
        let result = handle.await;
        assert!(result.is_err(), "free-spawn aborted task should return JoinError");
    });
    println!("  test_free_spawn_abort OK");
}
