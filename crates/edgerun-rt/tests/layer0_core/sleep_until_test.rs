// Test sleep_until and timeout_at with the actual runtime.
use edgerun_rt::{sleep_until, timeout_at, SleepUntil, Runtime, spawn};
use std::time::{Duration, Instant as StdInstant};

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_sleep_until_basic();
        test_sleep_until_deadline_method();
        test_sleep_until_accuracy();
        test_sleep_until_past_returns_immediately();
        test_timeout_at_success();
        test_timeout_at_expires();
        test_timeout_at_wrapped_sleep();
        test_timeout_at_past_expires_immediately();
        println!("All sleep_until/timeout_at tests passed!");
    });
}

fn test_sleep_until_basic() {
    println!("  test_sleep_until_basic...");
    let deadline = StdInstant::now() + Duration::from_millis(50);
    let h = spawn(async move {
        sleep_until(deadline).await;
        let elapsed = deadline.elapsed();
        println!("    slept until deadline, overshoot: {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(100));
    });
    std::thread::sleep(Duration::from_millis(150));
    drop(h);
    println!("  test_sleep_until_basic OK");
}

fn test_sleep_until_deadline_method() {
    println!("  test_sleep_until_deadline_method...");
    let deadline = StdInstant::now() + Duration::from_millis(30);
    let fut = sleep_until(deadline);
    assert_eq!(fut.deadline(), deadline);
    println!("  test_sleep_until_deadline_method OK");
}

fn test_sleep_until_accuracy() {
    println!("  test_sleep_until_accuracy...");
    let mut handles = vec![];

    for i in 1..=5 {
        let ms = i as u64 * 20;
        let deadline = StdInstant::now() + Duration::from_millis(ms);
        let h = spawn(async move {
            let start = StdInstant::now();
            sleep_until(deadline).await;
            let elapsed = start.elapsed();
            println!("    sleep_until {}ms took {:?}", ms, elapsed);
            assert!(elapsed >= Duration::from_millis(ms.saturating_sub(5)));
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(200));
    for h in handles {
        drop(h);
    }
    println!("  test_sleep_until_accuracy OK");
}

fn test_sleep_until_past_returns_immediately() {
    println!("  test_sleep_until_past_returns_immediately...");
    let deadline = StdInstant::now() - Duration::from_millis(10);
    let h = spawn(async move {
        let start = StdInstant::now();
        sleep_until(deadline).await;
        let elapsed = start.elapsed();
        println!("    past deadline returned in {:?}", elapsed);
        assert!(elapsed < Duration::from_millis(10));
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_sleep_until_past_returns_immediately OK");
}

fn test_timeout_at_success() {
    println!("  test_timeout_at_success...");
    let deadline = StdInstant::now() + Duration::from_millis(200);
    let h = spawn(async move {
        let result = timeout_at(deadline, async {
            edgerun_rt::sleep(Duration::from_millis(20)).await;
            "done"
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
        println!("    timeout_at completed with value");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_timeout_at_success OK");
}

fn test_timeout_at_expires() {
    println!("  test_timeout_at_expires...");
    let deadline = StdInstant::now() + Duration::from_millis(30);
    let h = spawn(async move {
        let result = timeout_at(deadline, async {
            edgerun_rt::sleep(Duration::from_secs(100)).await;
            "should not reach"
        }).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(format!("{}", err), "deadline elapsed");
        println!("    timeout_at expired as expected");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_timeout_at_expires OK");
}

fn test_timeout_at_wrapped_sleep() {
    println!("  test_timeout_at_wrapped_sleep...");
    let deadline = StdInstant::now() + Duration::from_millis(200);
    let h = spawn(async move {
        let result = timeout_at(deadline, async {
            edgerun_rt::sleep(Duration::from_millis(30)).await;
            42
        }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_timeout_at_wrapped_sleep OK");
}

fn test_timeout_at_past_expires_immediately() {
    println!("  test_timeout_at_past_expires_immediately...");
    let deadline = StdInstant::now() - Duration::from_millis(10);
    let h = spawn(async move {
        let start = StdInstant::now();
        let result = timeout_at(deadline, async { 99 }).await;
        let elapsed = start.elapsed();
        assert!(result.is_err());
        assert!(elapsed < Duration::from_millis(10));
        println!("    past deadline expired in {:?}", elapsed);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_timeout_at_past_expires_immediately OK");
}
