use edgerun_bare_rt::Builder;

#[test]
fn runtime_builder_multi_thread() {
    let rt = Builder::new_multi_thread().build();
    let _ = rt;
}