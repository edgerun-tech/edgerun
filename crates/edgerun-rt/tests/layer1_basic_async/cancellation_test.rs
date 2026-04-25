// Test CancellationToken with the actual runtime.
use edgerun_rt::{spawn, CancellationToken, Runtime};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_cancel_waits();
        test_cancel_multiple_waiters();
        test_clone_shares_cancellation();
        test_cancel_unblocks_task();
        test_cancel_cooperative_shutdown();
        test_is_cancelled_flag();
        test_cancel_idempotent();
        test_wait_blocking();
        test_child_token();
        test_cancel_with_sleep();
        println!("All CancellationToken tests passed!");
    });
}

fn test_cancel_waits() {
    println!("  test_cancel_waits...");
    let token = CancellationToken::new();
    let token2 = token.clone();

    let h = spawn(async move {
        token2.cancelled().await;
    });

    std::thread::sleep(Duration::from_millis(20));
    token.cancel();
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cancel_waits OK");
}

fn test_cancel_multiple_waiters() {
    println!("  test_cancel_multiple_waiters...");
    let token = CancellationToken::new();
    let mut handles = vec![];
    let count = Arc::new(AtomicUsize::new(0));

    for i in 0..5 {
        let t = token.clone();
        let c = count.clone();
        let h = spawn(async move {
            t.cancelled().await;
            c.fetch_add(1, Ordering::SeqCst);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(20));
    token.cancel();
    std::thread::sleep(Duration::from_millis(100));

    assert_eq!(
        count.load(Ordering::SeqCst),
        5,
        "all 5 waiters should be woken"
    );
    for h in handles {
        drop(h);
    }
    println!("  test_cancel_multiple_waiters OK");
}

fn test_clone_shares_cancellation() {
    println!("  test_clone_shares_cancellation...");
    let token1 = CancellationToken::new();
    let token2 = token1.clone();
    let token3 = token1.clone();

    assert!(!token1.is_cancelled());
    assert!(!token2.is_cancelled());

    token2.cancel();
    assert!(token1.is_cancelled());
    assert!(token3.is_cancelled());
    println!("  test_clone_shares_cancellation OK");
}

fn test_cancel_unblocks_task() {
    println!("  test_cancel_unblocks_task...");
    let token = CancellationToken::new();
    let started = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));

    let t = token.clone();
    let s = started.clone();
    let c = completed.clone();
    let h = spawn(async move {
        s.fetch_add(1, Ordering::SeqCst);
        t.cancelled().await;
        c.fetch_add(1, Ordering::SeqCst);
    });

    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(started.load(Ordering::SeqCst), 1);
    assert_eq!(
        completed.load(Ordering::SeqCst),
        0,
        "should not complete before cancel"
    );

    token.cancel();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        completed.load(Ordering::SeqCst),
        1,
        "should complete after cancel"
    );

    drop(h);
    println!("  test_cancel_unblocks_task OK");
}

fn test_cancel_cooperative_shutdown() {
    println!("  test_cancel_cooperative_shutdown...");
    let token = CancellationToken::new();
    let counter = Arc::new(AtomicUsize::new(0));

    // Task that checks cancellation cooperatively
    let t = token.clone();
    let c = counter.clone();
    let h = spawn(async move {
        for _ in 0..10 {
            if t.is_cancelled() {
                break;
            }
            c.fetch_add(1, Ordering::SeqCst);
            edgerun_rt::sleep(Duration::from_millis(5)).await;
        }
    });

    std::thread::sleep(Duration::from_millis(20));
    token.cancel();
    std::thread::sleep(Duration::from_millis(100));

    let work_done = counter.load(Ordering::SeqCst);
    assert!(
        work_done < 10,
        "task should stop early after cancel, did {} iterations",
        work_done
    );
    drop(h);
    println!("  test_cancel_cooperative_shutdown OK");
}

fn test_is_cancelled_flag() {
    println!("  test_is_cancelled_flag...");
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
    token.cancel();
    assert!(token.is_cancelled());
    token.cancel(); // idempotent
    assert!(token.is_cancelled());
    println!("  test_is_cancelled_flag OK");
}

fn test_cancel_idempotent() {
    println!("  test_cancel_idempotent...");
    let token = CancellationToken::new();
    let count = Arc::new(AtomicUsize::new(0));

    let t = token.clone();
    let c = count.clone();
    let h = spawn(async move {
        t.cancelled().await;
        c.fetch_add(1, Ordering::SeqCst);
    });

    std::thread::sleep(Duration::from_millis(20));
    // Cancel multiple times
    token.cancel();
    token.cancel();
    token.cancel();

    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        count.load(Ordering::SeqCst),
        1,
        "task should complete exactly once"
    );
    drop(h);
    println!("  test_cancel_idempotent OK");
}

fn test_wait_blocking() {
    println!("  test_wait_blocking...");
    let token = CancellationToken::new();
    let t = token.clone();

    let thread = std::thread::spawn(move || {
        t.wait();
    });

    std::thread::sleep(Duration::from_millis(20));
    token.cancel();
    thread.join().unwrap();
    println!("  test_wait_blocking OK");
}

fn test_child_token() {
    println!("  test_child_token...");
    let parent = CancellationToken::new();
    let child = parent.child_token();

    assert!(!parent.is_cancelled());
    assert!(!child.is_cancelled());

    parent.cancel();
    assert!(child.is_cancelled());
    println!("  test_child_token OK");
}

fn test_cancel_with_sleep() {
    println!("  test_cancel_with_sleep...");
    let token = CancellationToken::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let t = token.clone();
    let c = counter.clone();
    let h = spawn(async move {
        loop {
            if t.is_cancelled() {
                break;
            }
            c.fetch_add(1, Ordering::SeqCst);
            edgerun_rt::sleep(Duration::from_millis(10)).await;
        }
    });

    std::thread::sleep(Duration::from_millis(50));
    token.cancel();
    std::thread::sleep(Duration::from_millis(50));

    let work_done = counter.load(Ordering::SeqCst);
    assert!(work_done > 0, "some work should be done before cancel");
    assert!(
        work_done < 10,
        "work should stop after cancel, did {}",
        work_done
    );

    drop(h);
    println!("  test_cancel_with_sleep OK (did {} iterations)", work_done);
}
