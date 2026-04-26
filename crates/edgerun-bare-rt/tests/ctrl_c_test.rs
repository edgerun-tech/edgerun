use edgerun_bare_rt::CtrlC;

#[test]
fn ctrl_c_new() {
    let _ = CtrlC::new();
}