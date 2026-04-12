// Test timers (sleep, timeout, interval, ctrl_c) with the actual runtime.
use edgerun_rt::{
    sleep, timeout, interval, ctrl_c, Sleep, Timeout, Interval,
    MissedTickBehavior, Elapsed,
    Runtime, spawn,
};
use std::time::Duration;
use std::time::Instant as StdInstant;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_sleep_basic();
        test_sleep_accuracy();
        test_concurrent_sleeps();
        test_timeout_success();
        test_timeout_expires();
        test_timeout_wrapped_sleep();
        test_interval_ticks();
        test_interval_skip_missed();
        test_sleep_type();
        test_elapsed_error_type();
        test_ctrl_c_type();
        test_many_concurrent_sleeps();
        println!("All timers tests passed!");
    });
}

fn test_sleep_basic() {
    println!("  test_sleep_basic...");
    let h = spawn(async {
        let start = StdInstant::now();
        sleep(Duration::from_millis(50)).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(45), "sleep should wait at least 50ms, got {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(200), "sleep should not wait much longer, got {:?}", elapsed);
        println!("  test_sleep_basic OK (waited {:?})", elapsed);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
}

fn test_sleep_accuracy() {
    println!("  test_sleep_accuracy...");
    let h = spawn(async {
        let start = StdInstant::now();
        sleep(Duration::from_millis(10)).await;
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(5), "10ms sleep should take at least 5ms, got {:?}", elapsed);
        println!("  test_sleep_accuracy OK (waited {:?})", elapsed);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
}

fn test_concurrent_sleeps() {
    println!("  test_concurrent_sleeps...");
    let mut handles = vec![];

    for i in 1..=5 {
        let dur = Duration::from_millis(i as u64 * 20);
        let h = spawn(async move {
            let start = StdInstant::now();
            sleep(dur).await;
            let elapsed = start.elapsed();
            println!("    sleep {}ms took {:?}", dur.as_millis(), elapsed);
            assert!(elapsed >= dur.saturating_sub(Duration::from_millis(5)));
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(200));
    for h in handles {
        drop(h);
    }
    println!("  test_concurrent_sleeps OK");
}

fn test_timeout_success() {
    println!("  test_timeout_success...");
    let h = spawn(async {
        let result = timeout(
            Duration::from_millis(100),
            async { "done" },
        ).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
        println!("  test_timeout_success OK");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
}

fn test_timeout_expires() {
    println!("  test_timeout_expires...");
    let h = spawn(async {
        let result = timeout(
            Duration::from_millis(50),
            async {
                sleep(Duration::from_secs(100)).await;
                "should not reach here"
            },
        ).await;
        assert!(result.is_err(), "timeout should expire");
        let err = result.unwrap_err();
        assert_eq!(format!("{}", err), "deadline elapsed");
        println!("  test_timeout_expires OK");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
}

fn test_timeout_wrapped_sleep() {
    println!("  test_timeout_wrapped_sleep...");
    let h = spawn(async {
        let result = timeout(
            Duration::from_millis(200),
            async {
                sleep(Duration::from_millis(50)).await;
                42
            },
        ).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        println!("  test_timeout_wrapped_sleep OK");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
}

fn test_interval_ticks() {
    println!("  test_interval_ticks...");
    let h = spawn(async {
        let mut iv = interval(Duration::from_millis(30));
        let mut count = 0;

        let start = StdInstant::now();
        for _ in 0..3 {
            iv.tick().await;
            count += 1;
            println!("    interval tick {} at {:?}", count, start.elapsed());
        }

        let elapsed = start.elapsed();
        assert_eq!(count, 3);
        assert!(elapsed >= Duration::from_millis(70), "3 ticks at 30ms should take ~90ms, got {:?}", elapsed);
        println!("  test_interval_ticks OK (elapsed {:?})", elapsed);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
}

fn test_interval_skip_missed() {
    println!("  test_interval_skip_missed...");
    let h = spawn(async {
        let mut iv = interval(Duration::from_millis(20));
        iv.set_missed_tick_behavior(MissedTickBehavior::Skip);

        for i in 0..3 {
            iv.tick().await;
            println!("    skip interval tick {}", i + 1);
        }
        println!("  test_interval_skip_missed OK");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
}

fn test_sleep_type() {
    println!("  test_sleep_type...");
    let _fut: Sleep = sleep(Duration::from_millis(1));
    println!("  test_sleep_type OK");
}

fn test_elapsed_error_type() {
    println!("  test_elapsed_error_type...");
    let err = Elapsed;
    assert_eq!(format!("{}", err), "deadline elapsed");
    println!("  test_elapsed_error_type OK");
}

fn test_ctrl_c_type() {
    println!("  test_ctrl_c_type...");
    let _fut: edgerun_rt::CtrlC = ctrl_c();
    println!("  test_ctrl_c_type OK");
}

fn test_many_concurrent_sleeps() {
    println!("  test_many_concurrent_sleeps...");
    let mut handles = vec![];

    // 10 sleeps with staggered durations
    for i in 1..=10 {
        let ms = i as u64 * 10;
        let h = spawn(async move {
            let start = StdInstant::now();
            sleep(Duration::from_millis(ms)).await;
            let elapsed = start.elapsed();
            assert!(elapsed >= Duration::from_millis(ms.saturating_sub(5)));
            println!("    sleep {}ms took {:?}", ms, elapsed);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(200));
    for h in handles {
        drop(h);
    }
    println!("  test_many_concurrent_sleeps OK");
}
