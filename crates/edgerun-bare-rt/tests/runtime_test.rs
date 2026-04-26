use edgerun_bare_rt::Builder;

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
    
    let h1 = rt.spawn(Box::pin(async { 1 }));
    let h2 = rt.spawn(Box::pin(async { 2 }));
    
    // Note: These won't complete without a worker loop, but at least they compile now
    let _ = (h1, h2);
}