use edgerun_bare_rt::{Runtime, spawn};

#[test]
fn spawn_compiles() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    
    let handle = rt.spawn(async { 42 });
    
    assert!(!handle.is_finished());
}