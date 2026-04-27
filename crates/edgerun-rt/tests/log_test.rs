use edgerun_rt::Level;

#[test]
fn log_level() {
    let _ = Level::Error;
    let _ = Level::Warn;
    let _ = Level::Info;
    let _ = Level::Debug;
    let _ = Level::Trace;
}
