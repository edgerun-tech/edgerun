// Test OnceCell with the actual runtime.
use edgerun_rt::{spawn, OnceCell, Runtime};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

static INIT_COUNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_once_cell_new_uninit();
        test_once_cell_get_or_init();
        test_once_cell_get_returns_value();
        test_once_cell_initialized_flag();
        test_once_cell_get_mut();
        test_once_cell_into_inner();
        test_once_cell_async_wait();
        test_once_cell_concurrent_spawn();
        test_once_cell_wait_already_initialized();
        println!("All OnceCell tests passed!");
    });
}

fn test_once_cell_new_uninit() {
    println!("  test_once_cell_new_uninit...");
    let cell: OnceCell<String> = OnceCell::new();
    assert!(!cell.initialized());
    assert!(cell.get().is_none());
    println!("  test_once_cell_new_uninit OK");
}

fn test_once_cell_get_or_init() {
    println!("  test_once_cell_get_or_init...");
    INIT_COUNT.store(0, Ordering::SeqCst);
    let cell: OnceCell<usize> = OnceCell::new();

    let v1 = cell.get_or_init(|| {
        INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        42
    });
    assert_eq!(*v1, 42);
    assert_eq!(INIT_COUNT.load(Ordering::SeqCst), 1);

    // Second call should not run closure.
    let v2 = cell.get_or_init(|| {
        INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        99
    });
    assert_eq!(*v2, 42);
    assert_eq!(INIT_COUNT.load(Ordering::SeqCst), 1);
    println!("  test_once_cell_get_or_init OK");
}

fn test_once_cell_get_returns_value() {
    println!("  test_once_cell_get_returns_value...");
    let cell: OnceCell<String> = OnceCell::new();
    assert!(cell.get().is_none());
    cell.get_or_init(|| String::from("hello"));
    assert_eq!(cell.get().unwrap(), "hello");
    println!("  test_once_cell_get_returns_value OK");
}

fn test_once_cell_initialized_flag() {
    println!("  test_once_cell_initialized_flag...");
    let cell: OnceCell<i32> = OnceCell::new();
    assert!(!cell.initialized());
    cell.get_or_init(|| 1);
    assert!(cell.initialized());
    println!("  test_once_cell_initialized_flag OK");
}

fn test_once_cell_get_mut() {
    println!("  test_once_cell_get_mut...");
    let mut cell: OnceCell<Vec<i32>> = OnceCell::new();
    assert!(cell.get_mut().is_none());
    cell.get_or_init(|| vec![1, 2, 3]);
    let v = cell.get_mut().expect("should be Some");
    v.push(4);
    assert_eq!(cell.get().unwrap(), &[1, 2, 3, 4]);
    println!("  test_once_cell_get_mut OK");
}

fn test_once_cell_into_inner() {
    println!("  test_once_cell_into_inner...");
    let cell: OnceCell<String> = OnceCell::new();
    assert!(OnceCell::<String>::new().into_inner().is_none());
    cell.get_or_init(|| String::from("value"));
    let v = cell.into_inner().expect("should be Some");
    assert_eq!(v, "value");
    println!("  test_once_cell_into_inner OK");
}

fn test_once_cell_async_wait() {
    println!("  test_once_cell_async_wait...");
    INIT_COUNT.store(0, Ordering::SeqCst);
    let cell: Arc<OnceCell<usize>> = Arc::new(OnceCell::new());
    let cell2 = cell.clone();

    // Initialize from this task.
    cell.get_or_init(|| {
        INIT_COUNT.fetch_add(1, Ordering::SeqCst);
        100
    });

    // Now wait should return Ready.
    let h = spawn(async move {
        cell2.wait().await;
        assert_eq!(*cell2.get().unwrap(), 100);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_once_cell_async_wait OK");
}

fn test_once_cell_concurrent_spawn() {
    println!("  test_once_cell_concurrent_spawn...");
    INIT_COUNT.store(0, Ordering::SeqCst);
    let cell: Arc<OnceCell<usize>> = Arc::new(OnceCell::new());
    let cell2 = cell.clone();
    let cell3 = cell.clone();

    // One task initializes.
    let init_task = spawn(async move {
        cell.get_or_init(|| {
            INIT_COUNT.fetch_add(1, Ordering::SeqCst);
            777
        });
    });

    // Other task waits.
    let wait_task = spawn(async move {
        cell2.wait().await;
        assert_eq!(*cell2.get().unwrap(), 777);
    });

    std::thread::sleep(Duration::from_millis(200));
    drop(init_task);
    drop(wait_task);
    assert_eq!(INIT_COUNT.load(Ordering::SeqCst), 1);
    println!("  test_once_cell_concurrent_spawn OK");
}

fn test_once_cell_wait_already_initialized() {
    println!("  test_once_cell_wait_already_initialized...");
    let cell: Arc<OnceCell<i32>> = Arc::new(OnceCell::new());
    let cell2 = cell.clone();

    cell.get_or_init(|| 42);

    // Wait on already-initialized cell should return immediately.
    let h = spawn(async move {
        cell2.wait().await;
        assert_eq!(*cell2.get().unwrap(), 42);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_once_cell_wait_already_initialized OK");
}
