// Test join! macro with the actual runtime — migrated to standard #[test] harness.
use edgerun_rt::{join, Runtime};
use std::time::Duration;

#[test]
fn join_one() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let result = rt.block_on(async { join!(async { 42 }) });
    assert_eq!(result, 42);
}

#[test]
fn join_two() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b) = rt.block_on(async { join!(async { 1 }, async { 2 }) });
    assert_eq!(a, 1);
    assert_eq!(b, 2);
}

#[test]
fn join_three() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b, c) =
        rt.block_on(async { join!(async { "hello" }, async { 42 }, async { vec![1, 2, 3] }) });
    assert_eq!(a, "hello");
    assert_eq!(b, 42);
    assert_eq!(c, vec![1, 2, 3]);
}

#[test]
fn join_four() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b, c, d) =
        rt.block_on(async { join!(async { 10 }, async { 20 }, async { 30 }, async { 40 }) });
    assert_eq!(a, 10);
    assert_eq!(b, 20);
    assert_eq!(c, 30);
    assert_eq!(d, 40);
}

#[test]
fn join_concurrent_execution() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b, elapsed) = rt.block_on(async {
        let start = std::time::Instant::now();
        let (a, b) = join!(
            async {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
                "done1"
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(100)).await;
                "done2"
            },
        );
        (a, b, start.elapsed())
    });
    assert_eq!(a, "done1");
    assert_eq!(b, "done2");
    assert!(
        elapsed < Duration::from_millis(180),
        "concurrent join should take ~100ms, took {:?}",
        elapsed
    );
}

#[test]
fn join_with_sleep() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b, c) = rt.block_on(async {
        join!(
            async {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                100
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                200
            },
            async {
                edgerun_rt::sleep(Duration::from_millis(10)).await;
                300
            },
        )
    });
    assert_eq!(a, 100);
    assert_eq!(b, 200);
    assert_eq!(c, 300);
}

#[test]
fn join_error_propagation() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let (a, b) = rt.block_on(async {
        join!(async { Ok::<i32, &'static str>(42) }, async {
            Err::<i32, &'static str>("boom")
        },)
    });
    assert_eq!(a, Ok(42));
    assert_eq!(b, Err("boom"));
}
