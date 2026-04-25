// PROVE: The reactor thread sleeps for the default 100ms timeout
// when the timer heap is empty, and new timers registered during that
// sleep don't wake it up.
//
// If sleep(1ms) takes ~100ms, the reactor is ignoring newly registered timers.
//
// Fix: the reactor needs an eventfd/self-pipe so that register_timer
// can wake it when a timer's deadline is sooner than the current epoll timeout.

use edgerun_rt::{sleep, Runtime};
use std::time::{Duration, Instant};

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

    // Test 1: Sleep immediately after runtime starts (timer heap was empty).
    // If the reactor calculated its 100ms timeout before this timer was
    // registered, the sleep will take ~100ms instead of ~1ms.
    println!("Test 1: sleep(1ms) right after runtime build");
    let t0 = Instant::now();
    rt.block_on(sleep(Duration::from_millis(1)));
    let elapsed = t0.elapsed();
    println!(
        "  took {:?} (should be ~1ms, if ~100ms → reactor wakeup bug)",
        elapsed
    );
    assert!(
        elapsed < Duration::from_millis(20),
        "BUG PROVEN: sleep(1ms) took {:?} — reactor did not wake for new timer",
        elapsed
    );

    // Test 2: Two sleeps back-to-back. After test 1, the reactor has
    // processed a timer, so the heap might be in a different state.
    println!("\nTest 2: second sleep(1ms) after first completed");
    let t0 = Instant::now();
    rt.block_on(sleep(Duration::from_millis(1)));
    let elapsed = t0.elapsed();
    println!("  took {:?} (should be ~1ms)", elapsed);

    // Test 3: Concurrent sleeps — these register timers while the reactor
    // is potentially blocked on the first one's timeout.
    println!("\nTest 3: 5 concurrent sleeps (1ms each)");
    let t0 = Instant::now();
    let mut handles = vec![];
    for _ in 0..5 {
        let h = rt.spawn(async {
            sleep(Duration::from_millis(1)).await;
        });
        handles.push(h);
    }
    for h in handles {
        rt.block_on(h).unwrap();
    }
    let elapsed = t0.elapsed();
    println!(
        "  all 5 completed in {:?} (should be ~1-5ms, not ~100ms)",
        elapsed
    );

    // Test 4: Rapid fire — spawn a sleep, immediately spawn another.
    // The second timer is registered while the reactor is waiting for the first.
    println!("\nTest 4: rapid fire — 10 back-to-back block_on(sleep(1ms))");
    let mut total = Duration::ZERO;
    for i in 0..10 {
        let t0 = Instant::now();
        rt.block_on(sleep(Duration::from_millis(1)));
        let elapsed = t0.elapsed();
        total += elapsed;
        println!("  #{}: {:?}", i + 1, elapsed);
    }
    let avg = total / 10;
    println!(
        "  average: {:?} (should be ~1ms, if ~100ms → reactor wakeup bug)",
        avg
    );

    println!("\n=== All probe tests completed ===");
}
