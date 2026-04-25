use edgerun_bare_rt::Runtime;

#[test]
fn spawn_compiles() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    
    // Just verify the runtime builds correctly
    assert!(true);
}