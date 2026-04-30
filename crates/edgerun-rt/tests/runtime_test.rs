use edgerun_rt::Builder;
use edgerun_rt::{run_queue, spawn};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[test]
fn runtime_block_on_with_await() {
    let rt = Builder::new_multi_thread().build().unwrap();

    async fn task() -> i32 {
        let a = async { 10 }.await;
        let b = async { 32 }.await;
        a + b
    }

    let result = rt.block_on(Box::pin(task()));
    assert_eq!(result, 42);
}

#[test]
fn runtime_join_handles_work() {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .build()
        .unwrap();
    assert_eq!(rt.worker_count(), 2);

    let result = rt.block_on(async {
        let h1 = rt.spawn(async { 1 });
        let h2 = rt.spawn(async { 2 });

        h1.await.unwrap() + h2.await.unwrap()
    });

    assert_eq!(result, 3);
}

#[test]
fn runtime_spawn_local_runs() {
    let rt = Builder::new_multi_thread().build().unwrap();

    let result = rt.block_on(async { rt.spawn_local(async { 7 + 1 }).await.unwrap() });

    assert_eq!(result, 8);
}

#[test]
fn run_queue_runs_nested_spawn_on_next_cycle() {
    let outer_started = Arc::new(AtomicBool::new(false));
    let inner_started = Arc::new(AtomicBool::new(false));
    let outer_started2 = outer_started.clone();
    let inner_started2 = inner_started.clone();

    spawn(async move {
        outer_started2.store(true, Ordering::SeqCst);
        spawn(async move {
            inner_started2.store(true, Ordering::SeqCst);
        });
    });

    run_queue();
    assert!(outer_started.load(Ordering::SeqCst));
    assert!(!inner_started.load(Ordering::SeqCst));

    run_queue();
    assert!(inner_started.load(Ordering::SeqCst));
}
