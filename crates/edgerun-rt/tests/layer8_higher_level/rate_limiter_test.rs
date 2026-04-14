// Test RateLimiter with the actual runtime.
use edgerun_rt::{RateLimiter, Runtime, spawn};
use std::time::{Duration, Instant};

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_rate_limiter_new_full();
        test_rate_limiter_try_acquire_success();
        test_rate_limiter_try_acquire_failure();
        test_rate_limiter_available();
        test_rate_limiter_rate_and_capacity();
        test_rate_limiter_acquire_waits();
        test_rate_limiter_burst();
        test_rate_limiter_refill_over_time();
        test_rate_limiter_concurrent_acquire();
        test_rate_limiter_zero_rate();
        println!("All RateLimiter tests passed!");
    });
}

fn test_rate_limiter_new_full() {
    println!("  test_rate_limiter_new_full...");
    let limiter = RateLimiter::new(10.0, 20);
    assert_eq!(limiter.capacity(), 20);
    assert_eq!(limiter.rate(), 10.0);
    // Should start full.
    assert!(limiter.try_acquire(20));
    println!("  test_rate_limiter_new_full OK");
}

fn test_rate_limiter_try_acquire_success() {
    println!("  test_rate_limiter_try_acquire_success...");
    let limiter = RateLimiter::new(100.0, 100);
    assert!(limiter.try_acquire(50));
    assert!(limiter.try_acquire(50));
    // Now should be empty.
    assert!(!limiter.try_acquire(1));
    println!("  test_rate_limiter_try_acquire_success OK");
}

fn test_rate_limiter_try_acquire_failure() {
    println!("  test_rate_limiter_try_acquire_failure...");
    let limiter = RateLimiter::new(1.0, 5);
    // Drain all tokens.
    for _ in 0..5 {
        assert!(limiter.try_acquire(1));
    }
    assert!(!limiter.try_acquire(1));
    println!("  test_rate_limiter_try_acquire_failure OK");
}

fn test_rate_limiter_available() {
    println!("  test_rate_limiter_available...");
    let limiter = RateLimiter::new(10.0, 100);
    let avail = limiter.available();
    assert!((99.0..=100.0).contains(&avail));
    println!("  test_rate_limiter_available OK");
}

fn test_rate_limiter_rate_and_capacity() {
    println!("  test_rate_limiter_rate_and_capacity...");
    let limiter = RateLimiter::new(42.0, 99);
    assert_eq!(limiter.rate(), 42.0);
    assert_eq!(limiter.capacity(), 99);
    println!("  test_rate_limiter_rate_and_capacity OK");
}

fn test_rate_limiter_acquire_waits() {
    println!("  test_rate_limiter_acquire_waits...");
    let limiter = RateLimiter::new(100.0, 1);
    // Drain the single token.
    assert!(limiter.try_acquire(1));

    let start = Instant::now();
    let h = spawn(async move {
        limiter.acquire(1).await;
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    let elapsed = start.elapsed();
    // Should have waited ~10ms (1 token at 100/sec).
    assert!(elapsed > Duration::from_millis(5));
    println!("  test_rate_limiter_acquire_waits OK");
}

fn test_rate_limiter_burst() {
    println!("  test_rate_limiter_burst...");
    // Burst of 10, rate of 10/sec.
    let limiter = RateLimiter::new(10.0, 10);

    // Should be able to acquire all 10 immediately.
    for _ in 0..10 {
        assert!(limiter.try_acquire(1));
    }
    // 11th should fail.
    assert!(!limiter.try_acquire(1));
    println!("  test_rate_limiter_burst OK");
}

fn test_rate_limiter_refill_over_time() {
    println!("  test_rate_limiter_refill_over_time...");
    // 100 tokens/sec, capacity 10.
    let limiter = RateLimiter::new(100.0, 10);

    // Drain all.
    for _ in 0..10 {
        limiter.try_acquire(1);
    }
    assert!(!limiter.try_acquire(1));

    // Wait 100ms — should refill ~10 tokens.
    std::thread::sleep(Duration::from_millis(100));
    // Should have some tokens now.
    let avail = limiter.available();
    assert!(avail >= 1.0, "expected >= 1 token after 100ms, got {}", avail);
    println!("  test_rate_limiter_refill_over_time OK");
}

fn test_rate_limiter_concurrent_acquire() {
    println!("  test_rate_limiter_concurrent_acquire...");
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let limiter = Arc::new(RateLimiter::new(1000.0, 100));
    let success = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..20 {
        let limiter = limiter.clone();
        let success = success.clone();
        handles.push(spawn(async move {
            if limiter.try_acquire(5) {
                success.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    std::thread::sleep(Duration::from_millis(100));
    for h in handles {
        drop(h);
    }

    // At most 20 (100 tokens / 5 each = 20 max).
    let s = success.load(Ordering::Relaxed);
    assert!(s > 0, "expected some successes, got 0");
    assert!(s <= 20, "expected <= 20 successes, got {}", s);
    println!("  test_rate_limiter_concurrent_acquire OK");
}

fn test_rate_limiter_zero_rate() {
    println!("  test_rate_limiter_zero_rate...");
    let limiter = RateLimiter::new(0.0, 5);
    // Can acquire up to capacity.
    for _ in 0..5 {
        assert!(limiter.try_acquire(1));
    }
    // No refill, so can't acquire more.
    assert!(!limiter.try_acquire(1));
    // wait_time should be MAX since rate is 0.
    let wait = limiter.wait_time(1);
    assert_eq!(wait, Duration::MAX);
    println!("  test_rate_limiter_zero_rate OK");
}
