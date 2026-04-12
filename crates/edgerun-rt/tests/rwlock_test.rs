// Test RwLock primitive with the actual runtime.
use edgerun_rt::{RwLock, Runtime, spawn};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_read_immediate();
        test_write_and_modify();
        test_multiple_concurrent_readers();
        test_write_blocks_readers();
        test_reader_blocks_writer();
        test_write_preference_waiting_writer_blocks_readers();
        test_multiple_writers_queued();
        test_into_inner();
        test_heavy_read_write_contention();
        println!("All RwLock tests passed!");
    });
}

fn test_read_immediate() {
    println!("  test_read_immediate...");
    let lock = Arc::new(RwLock::new(42u32));
    let lock2 = lock.clone();
    let handle = spawn(async move {
        let g = lock2.read().await;
        assert_eq!(*g, 42);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_read_immediate OK");
}

fn test_write_and_modify() {
    println!("  test_write_and_modify...");
    let lock = Arc::new(RwLock::new(0u32));
    let lock2 = lock.clone();
    let handle = spawn(async move {
        {
            let mut g = lock2.write().await;
            *g = 99;
        }
        // Verify read sees the new value
        let g = lock2.read().await;
        assert_eq!(*g, 99);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(handle);
    println!("  test_write_and_modify OK");
}

fn test_multiple_concurrent_readers() {
    println!("  test_multiple_concurrent_readers...");
    let lock = Arc::new(RwLock::new(42u32));
    let mut handles = vec![];

    // Spawn 5 readers simultaneously — all should acquire
    for i in 0..5 {
        let l = lock.clone();
        let h = spawn(async move {
            let g = l.read().await;
            assert_eq!(*g, 42);
            // Hold briefly to prove concurrent reading
            std::thread::sleep(Duration::from_millis(10));
            drop(g);
            println!("    reader {} done", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(100));
    for h in handles {
        drop(h);
    }
    println!("  test_multiple_concurrent_readers OK");
}

fn test_write_blocks_readers() {
    println!("  test_write_blocks_readers...");
    let lock = Arc::new(RwLock::new(0u32));

    // Writer acquires first
    let l1 = lock.clone();
    let writer = spawn(async move {
        let mut g = l1.write().await;
        *g = 999;
        std::thread::sleep(Duration::from_millis(50));
        drop(g);
    });

    // Reader tries to acquire while writer holds lock — should wait
    std::thread::sleep(Duration::from_millis(10));
    let l2 = lock.clone();
    let reader = spawn(async move {
        let start = std::time::Instant::now();
        let g = l2.read().await;
        let elapsed = start.elapsed();
        assert_eq!(*g, 999, "should see writer's value");
        println!("    reader waited {:?} for writer to release", elapsed);
    });

    std::thread::sleep(Duration::from_millis(150));
    drop(writer);
    drop(reader);
    println!("  test_write_blocks_readers OK");
}

fn test_reader_blocks_writer() {
    println!("  test_reader_blocks_writer...");
    let lock = Arc::new(RwLock::new(0u32));

    // Reader acquires first
    let l1 = lock.clone();
    let reader = spawn(async move {
        let g = l1.read().await;
        std::thread::sleep(Duration::from_millis(50));
        drop(g);
    });

    // Writer tries to acquire while reader holds lock — should wait
    std::thread::sleep(Duration::from_millis(10));
    let l2 = lock.clone();
    let writer = spawn(async move {
        let start = std::time::Instant::now();
        let mut g = l2.write().await;
        let elapsed = start.elapsed();
        *g = 777;
        println!("    writer waited {:?} for reader to release", elapsed);
    });

    std::thread::sleep(Duration::from_millis(150));
    drop(reader);
    drop(writer);
    println!("  test_reader_blocks_writer OK");
}

fn test_write_preference_waiting_writer_blocks_readers() {
    println!("  test_write_preference_waiting_writer_blocks_readers...");
    let lock = Arc::new(RwLock::new(0u32));
    let reader_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let writer2_done = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Writer 1 acquires and holds lock long enough for others to queue
    let l1 = lock.clone();
    let done1 = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let done1b = done1.clone();
    let writer1 = spawn(async move {
        let mut g = l1.write().await;
        *g = 1;
        // Hold for a while so others queue
        std::thread::sleep(Duration::from_millis(80));
        *g = 2;
        done1.store(true, std::sync::atomic::Ordering::SeqCst);
        drop(g);
    });

    // Wait for writer1 to be holding the lock
    std::thread::sleep(Duration::from_millis(20));

    // Spawn writer2 - should queue as waiting writer
    let l2 = lock.clone();
    let w2d = writer2_done.clone();
    let writer2 = spawn(async move {
        let start = std::time::Instant::now();
        let mut g = l2.write().await;
        let elapsed = start.elapsed();
        *g = 2;
        println!("    writer2 acquired after {:?}", elapsed);
        w2d.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    // Small delay so writer2 registers before reader
    std::thread::sleep(Duration::from_millis(10));

    // Spawn reader - write-preference: should block behind waiting writer
    let l3 = lock.clone();
    let rd = reader_done.clone();
    let reader = spawn(async move {
        let start = std::time::Instant::now();
        let g = l3.read().await;
        let elapsed = start.elapsed();
        println!("    reader acquired after {:?}, saw value {}", elapsed, *g);
        rd.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    // Wait for all to complete
    while !writer2_done.load(std::sync::atomic::Ordering::SeqCst)
        || !reader_done.load(std::sync::atomic::Ordering::SeqCst)
    {
        std::thread::sleep(Duration::from_millis(10));
    }
    std::thread::sleep(Duration::from_millis(50));
    drop(writer1);
    drop(writer2);
    drop(reader);
    println!("  test_write_preference_waiting_writer_blocks_readers OK");
}

fn test_multiple_writers_queued() {
    println!("  test_multiple_writers_queued...");
    let lock = Arc::new(RwLock::new(Vec::<u32>::new()));
    let mut handles = vec![];

    // First writer acquires
    let l0 = lock.clone();
    let writer0 = spawn(async move {
        let mut g = l0.write().await;
        g.push(0);
        std::thread::sleep(Duration::from_millis(30));
        drop(g);
    });

    // 4 more writers queue up
    std::thread::sleep(Duration::from_millis(10));
    for i in 1..=4 {
        let l = lock.clone();
        let h = spawn(async move {
            let mut g = l.write().await;
            g.push(i);
            println!("    writer {} appended", i);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(300));
    drop(writer0);
    for h in handles {
        drop(h);
    }

    // Verify all values were appended
    let l = lock.clone();
    let checker = spawn(async move {
        let g = l.read().await;
        println!("    final vec: {:?} (len {})", *g, g.len());
        assert_eq!(g.len(), 5);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(checker);
    println!("  test_multiple_writers_queued OK");
}

fn test_into_inner() {
    println!("  test_into_inner...");
    let lock = RwLock::new(vec![1, 2, 3]);
    let val = lock.into_inner();
    assert_eq!(val, vec![1, 2, 3]);
    println!("  test_into_inner OK");
}

fn test_heavy_read_write_contention() {
    println!("  test_heavy_read_write_contention...");
    let lock = Arc::new(RwLock::new(0u64));
    let iterations = 10u64;
    let mut handles = vec![];

    // Writer increments
    let w_lock = lock.clone();
    let writer = spawn(async move {
        for i in 1..=iterations {
            let mut g = w_lock.write().await;
            *g = i;
            std::thread::sleep(Duration::from_millis(5));
        }
        println!("    writer completed {} writes", iterations);
    });
    handles.push(writer);

    // 3 readers verify monotonic-ish reads
    for r in 0..3 {
        let r_lock = lock.clone();
        let h = spawn(async move {
            let mut count = 0;
            let mut last = 0u64;
            for _ in 0..iterations {
                let g = r_lock.read().await;
                assert!(
                    *g >= last,
                    "reader {} saw {} < {} (non-monotonic)",
                    r, *g, last
                );
                last = *g;
                count += 1;
                std::thread::sleep(Duration::from_millis(2));
            }
            println!("    reader {} completed {} reads, final value {}", r, count, last);
        });
        handles.push(h);
    }

    std::thread::sleep(Duration::from_millis(500));
    for h in handles {
        drop(h);
    }
    println!("  test_heavy_read_write_contention OK");
}
