// Test select! macro with the actual runtime.
use edgerun_rt::{Runtime, select};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_select_immediate_futures().await;
        test_select_faster_wins().await;
        test_select_timing().await;
        test_select_with_sleep().await;
        println!("All select! tests passed!");
    });
}

async fn test_select_immediate_futures() {
    println!("  test_select_immediate_futures...");
    let result = select! {
        x = async { 42 } => x,
        y = async { 99 } => y,
    };
    assert!(result == 42 || result == 99);
    println!("  test_select_immediate_futures OK (got {})", result);
}

async fn test_select_faster_wins() {
    println!("  test_select_faster_wins...");
    let result = select! {
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(10)).await;
            "fast"
        } => "fast",
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(100)).await;
            "slow"
        } => "slow",
    };
    assert_eq!(result, "fast");
    println!("  test_select_faster_wins OK");
}

async fn test_select_timing() {
    println!("  test_select_timing...");
    let start = std::time::Instant::now();
    let result = select! {
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(10)).await;
            1
        } => 1,
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(100)).await;
            2
        } => 2,
    };
    let elapsed = start.elapsed();
    assert_eq!(result, 1, "fast future should win");
    assert!(elapsed < Duration::from_millis(150), "select took too long: {:?}", elapsed);
    println!("  test_select_timing OK ({:?})", elapsed);
}

async fn test_select_with_sleep() {
    println!("  test_select_with_sleep...");
    let start = std::time::Instant::now();
    let result = select! {
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(5)).await;
            "winner"
        } => "winner",
        _ = async {
            edgerun_rt::sleep(Duration::from_millis(500)).await;
            "loser"
        } => "loser",
    };
    let elapsed = start.elapsed();
    assert_eq!(result, "winner");
    assert!(elapsed < Duration::from_millis(150), "select took too long: {:?}", elapsed);
    println!("  test_select_with_sleep OK ({:?})", elapsed);
}
