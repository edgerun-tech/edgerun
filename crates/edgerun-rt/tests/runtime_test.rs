use edgerun_rt::Builder;

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
    let rt = Builder::new_multi_thread().build().unwrap();

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
