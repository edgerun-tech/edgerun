// Test join! macro with the actual runtime.
use edgerun_rt::{Runtime, join};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_join_one().await;
        test_join_two().await;
        test_join_three().await;
        test_join_four().await;
        test_join_concurrent_execution().await;
        test_join_with_sleep().await;
        println!("All join! tests passed!");
    });
}

async fn test_join_one() {
    println!("  test_join_one...");
    let result = join!(async { 42 });
    assert_eq!(result, 42);
    println!("  test_join_one OK");
}

async fn test_join_two() {
    println!("  test_join_two...");
    let (a, b) = join!(
        async { 1 },
        async { 2 },
    );
    assert_eq!(a, 1);
    assert_eq!(b, 2);
    println!("  test_join_two OK");
}

async fn test_join_three() {
    println!("  test_join_three...");
    let (a, b, c) = join!(
        async { "hello" },
        async { 42 },
        async { vec![1, 2, 3] },
    );
    assert_eq!(a, "hello");
    assert_eq!(b, 42);
    assert_eq!(c, vec![1, 2, 3]);
    println!("  test_join_three OK");
}

async fn test_join_four() {
    println!("  test_join_four...");
    let (a, b, c, d) = join!(
        async { 10 },
        async { 20 },
        async { 30 },
        async { 40 },
    );
    assert_eq!(a, 10);
    assert_eq!(b, 20);
    assert_eq!(c, 30);
    assert_eq!(d, 40);
    println!("  test_join_four OK");
}

async fn test_join_concurrent_execution() {
    println!("  test_join_concurrent_execution...");
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
    let elapsed = start.elapsed();
    assert_eq!(a, "done1");
    assert_eq!(b, "done2");
    assert!(
        elapsed < Duration::from_millis(180),
        "concurrent join should take ~100ms, took {:?}",
        elapsed
    );
    println!("  test_join_concurrent_execution OK (took {:?})", elapsed);
}

async fn test_join_with_sleep() {
    println!("  test_join_with_sleep...");
    let (a, b, c) = join!(
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
    );
    assert_eq!(a, 100);
    assert_eq!(b, 200);
    assert_eq!(c, 300);
    println!("  test_join_with_sleep OK");
}
