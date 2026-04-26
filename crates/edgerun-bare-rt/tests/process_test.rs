use edgerun_bare_rt::process::Command;

#[test]
fn command_new() {
    let mut cmd = Command::new("echo");
    let _ = cmd;
}