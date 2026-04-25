// Test JoinSet with the actual runtime.
use edgerun_rt::{spawn, JoinSet, Runtime};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_spawn_and_join_next();
        test_join_next_empty();
        test_join_all();
        test_tasks_complete_out_of_order();
        test_abort_all();
        test_len_and_is_empty();
        test_join_set_with_sleep();
        test_concurrent_spawn_and_join();
        println!("All JoinSet tests passed!");
    });
}

fn test_spawn_and_join_next() {
    println!("  test_spawn_and_join_next...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        set.spawn(async { 42 });

        let result = set.join_next().await;
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
        assert!(set.is_empty());
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_spawn_and_join_next OK");
}

fn test_join_next_empty() {
    println!("  test_join_next_empty...");
    let h = spawn(async {
        let mut set: JoinSet<i32> = JoinSet::new();
        let result = set.join_next().await;
        assert!(result.is_none());
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_join_next_empty OK");
}

fn test_join_all() {
    println!("  test_join_all...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        for i in 0..5 {
            set.spawn(async move { i * 10 });
        }
        let results = set.join_all().await;
        assert_eq!(results.len(), 5);
        let mut vals: Vec<i32> = results.into_iter().map(|r| r.unwrap()).collect();
        vals.sort();
        assert_eq!(vals, vec![0, 10, 20, 30, 40]);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_join_all OK");
}

fn test_tasks_complete_out_of_order() {
    println!("  test_tasks_complete_out_of_order...");
    let h = spawn(async {
        let mut set = JoinSet::new();

        // Slow task first
        set.spawn(async {
            edgerun_rt::sleep(Duration::from_millis(50)).await;
            "slow"
        });
        // Fast task second
        set.spawn(async { "fast" });

        // Fast should complete first
        let first = set.join_next().await.unwrap().unwrap();
        assert_eq!(
            first, "fast",
            "fast task should complete first, got {}",
            first
        );

        let second = set.join_next().await.unwrap().unwrap();
        assert_eq!(second, "slow");

        assert!(set.is_empty());
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_tasks_complete_out_of_order OK");
}

fn test_abort_all() {
    println!("  test_abort_all...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        for _ in 0..5 {
            set.spawn(async {
                edgerun_rt::sleep(Duration::from_secs(100)).await;
                42
            });
        }
        assert_eq!(set.len(), 5);
        set.abort_all();
        assert_eq!(set.len(), 0);
        let result = set.join_next().await;
        assert!(result.is_none());
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_abort_all OK");
}

fn test_len_and_is_empty() {
    println!("  test_len_and_is_empty...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);

        set.spawn(async { 1 });
        set.spawn(async { 2 });
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());

        set.join_next().await;
        assert_eq!(set.len(), 1);

        set.join_next().await;
        assert!(set.is_empty());
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_len_and_is_empty OK");
}

fn test_join_set_with_sleep() {
    println!("  test_join_set_with_sleep...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        let counter = Arc::new(AtomicUsize::new(0));

        for i in 0..3 {
            let c = counter.clone();
            set.spawn(async move {
                edgerun_rt::sleep(Duration::from_millis(10 * (i as u64 + 1))).await;
                c.fetch_add(1, Ordering::SeqCst);
                i
            });
        }

        let mut results = Vec::new();
        while let Some(result) = set.join_next().await {
            results.push(result.unwrap());
        }
        assert_eq!(results.len(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_join_set_with_sleep OK");
}

fn test_concurrent_spawn_and_join() {
    println!("  test_concurrent_spawn_and_join...");
    let h = spawn(async {
        let mut set = JoinSet::new();
        let done = Arc::new(AtomicUsize::new(0));

        // Spawn 10 tasks
        for i in 0..10 {
            let d = done.clone();
            set.spawn(async move {
                edgerun_rt::sleep(Duration::from_millis(5)).await;
                d.fetch_add(1, Ordering::SeqCst);
                i
            });
        }

        // Join them as they complete
        let mut count = 0;
        while let Some(result) = set.join_next().await {
            assert!(result.is_ok());
            count += 1;
        }
        assert_eq!(count, 10);
        assert_eq!(done.load(Ordering::SeqCst), 10);
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_concurrent_spawn_and_join OK");
}
