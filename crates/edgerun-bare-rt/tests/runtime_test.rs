use edgerun_bare_rt::Builder;

#[test]
fn runtime_spawn_blocks() {
    let rt = Builder::new_multi_thread().build().unwrap();
    let handle = rt.spawn(async { 42 });
    let _ = handle;
}

#[test]
fn runtime_block_on() {
    let rt = Builder::new_multi_thread().build().unwrap();
    let future = Box::pin(async { 42 });
    let result = rt.block_on(future);
    assert_eq!(result, 42);
}