// Test Barrier primitive with the actual runtime.
use edgerun_rt::{Barrier, Runtime, spawn};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_barrier_single_task();
        test_barrier_two_tasks();
        test_barrier_three_tasks();
        test_barrier_reusable();
        test_barrier_with_delayed_arrivals();
        test_barrier_n_method();
        test_barrier_many_waiters();
        test_barrier_concurrent_waits();
        test_barrier_leader_identification();
        test_barrier_multiple_gens();
        println!("All Barrier tests passed!");
    });
}

fn test_barrier_single_task() {
    println!("  test_barrier_single_task...");
    let barrier = Arc::new(Barrier::new(1));
    let b = barrier.clone();
    let handle = spawn(async move {
        let result = b.wait().await;
        assert!(result.is_leader, "single task should be leader");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_barrier_single_task OK");
}

fn test_barrier_two_tasks() {
    println!("  test_barrier_two_tasks...");
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = vec![];
    let leader_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for i in 0..2 {
        let b = barrier.clone();
        let lc = leader_count.clone();
        let h = spawn(async move {
            let result = b.wait().await;
            if result.is_leader {
                lc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                println!("    task {} is leader", i);
            } else {
                println!("    task {} is follower", i);
            }
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(100));
    let leaders = leader_count.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(leaders, 1, "exactly one task should be leader");
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_two_tasks OK");
}

fn test_barrier_three_tasks() {
    println!("  test_barrier_three_tasks...");
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];
    let reached = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let leader_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for i in 0..3 {
        let b = barrier.clone();
        let rc = reached.clone();
        let lc = leader_count.clone();
        let h = spawn(async move {
            let result = b.wait().await;
            rc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if result.is_leader {
                lc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(100));
    let arrived = reached.load(std::sync::atomic::Ordering::SeqCst);
    let leaders = leader_count.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(arrived, 3, "all 3 tasks should pass barrier");
    assert_eq!(leaders, 1, "exactly one leader");
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_three_tasks OK");
}

fn test_barrier_reusable() {
    println!("  test_barrier_reusable...");
    let barrier = Arc::new(Barrier::new(2));
    let gen_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // First generation
    for gen in 0..3 {
        let b = barrier.clone();
        let gc = gen_count.clone();
        let h1 = spawn(async move {
            b.wait().await;
            gc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        let b2 = barrier.clone();
        let h2 = spawn(async move {
            b2.wait().await;
        });

        std::thread::sleep(Duration::from_millis(50));
        drop(h1);
        drop(h2);
        println!("    generation {} passed", gen);
    }

    let total = gen_count.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(total, 3, "3 generations should each count 1 pass");
    println!("  test_barrier_reusable OK");
}

fn test_barrier_with_delayed_arrivals() {
    println!("  test_barrier_with_delayed_arrivals...");
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];

    // First two arrive immediately
    for i in 0..2 {
        let b = barrier.clone();
        let h = spawn(async move {
            let start = std::time::Instant::now();
            b.wait().await;
            let elapsed = start.elapsed();
            println!("    early task {} waited {:?}", i, elapsed);
        });
        handles.push(h);
    }

    // Third arrives after delay
    std::thread::sleep(Duration::from_millis(50));
    let b = barrier.clone();
    let late = spawn(async move {
        let start = std::time::Instant::now();
        b.wait().await;
        let elapsed = start.elapsed();
        println!("    late task waited {:?}", elapsed);
    });
    handles.push(late);

    std::thread::sleep(Duration::from_millis(150));
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_with_delayed_arrivals OK");
}

fn test_barrier_n_method() {
    println!("  test_barrier_n_method...");
    let barrier = Barrier::new(5);
    assert_eq!(barrier.n(), 5);
    let barrier2 = Barrier::new(1);
    assert_eq!(barrier2.n(), 1);
    println!("  test_barrier_n_method OK");
}

fn test_barrier_many_waiters() {
    println!("  test_barrier_many_waiters...");
    let n = 10;
    let barrier = Arc::new(Barrier::new(n));
    let mut handles = vec![];
    let leader_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for i in 0..n {
        let b = barrier.clone();
        let lc = leader_count.clone();
        let h = spawn(async move {
            let result = b.wait().await;
            if result.is_leader {
                lc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(100));
    let leaders = leader_count.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(leaders, 1, "exactly one leader among {} waiters", n);
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_many_waiters OK");
}

fn test_barrier_concurrent_waits() {
    println!("  test_barrier_concurrent_waits...");
    let barrier = Arc::new(Barrier::new(4));
    let mut handles = vec![];

    // All 4 spawn at nearly the same time
    for i in 0..4 {
        let b = barrier.clone();
        let h = spawn(async move {
            let start = std::time::Instant::now();
            b.wait().await;
            let elapsed = start.elapsed();
            println!("    task {} passed barrier in {:?}", i, elapsed);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(100));
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_concurrent_waits OK");
}

fn test_barrier_leader_identification() {
    println!("  test_barrier_leader_identification...");
    let barrier = Arc::new(Barrier::new(2));
    let leaders = Arc::new(std::sync::Mutex::new(Vec::new()));

    for i in 0..2 {
        let b = barrier.clone();
        let l = leaders.clone();
        let h = spawn(async move {
            let result = b.wait().await;
            l.lock().unwrap().push((i, result.is_leader));
        });
        std::thread::sleep(Duration::from_millis(10));
        drop(h);
    }

    std::thread::sleep(Duration::from_millis(50));
    let leaders = leaders.lock().unwrap();
    assert_eq!(leaders.len(), 2, "both tasks should report");
    let leader_count = leaders.iter().filter(|(_, l)| *l).count();
    assert_eq!(leader_count, 1, "exactly one leader");
    println!("  test_barrier_leader_identification OK");
}

fn test_barrier_multiple_gens() {
    println!("  test_barrier_multiple_gens...");
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = vec![];
    let total_passes = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // 2 tasks, each waiting 3 times
    for i in 0..2 {
        let b = barrier.clone();
        let tp = total_passes.clone();
        let h = spawn(async move {
            for gen in 0..3 {
                b.wait().await;
                tp.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                println!("    task {} completed generation {}", i, gen);
            }
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(200));
    let passes = total_passes.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(passes, 6, "2 tasks x 3 generations = 6 passes");
    for h in handles {
        drop(h);
    }
    println!("  test_barrier_multiple_gens OK");
}
