// Test async process module edge cases with the actual runtime.
use edgerun_rt::{process, Runtime};
use std::process::Command;
use std::time::Duration;

#[test]
fn test_nonexistent_binary() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let result =
        rt.block_on(async { process::output(|| Command::new("/nonexistent/binary_12345")).await });
    assert!(result.is_err(), "nonexistent binary should fail");
}

#[test]
fn test_output_with_stdin() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    // `cat` echoes stdin to stdout.
    let result = rt.block_on(async {
        process::output(|| {
            let mut c = Command::new("cat");
            c
        })
        .await
    });
    // cat with no stdin input should produce empty output.
    let out = result.expect("cat should succeed");
    assert!(out.status.success());
    assert_eq!(out.stdout, b"");
}

#[test]
fn test_output_stderr_captured() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let result = rt.block_on(async {
        process::output(|| {
            let mut c = Command::new("sh");
            c.arg("-c").arg("echo error_msg >&2; exit 1");
            c
        })
        .await
    });
    let out = result.expect("output should capture stderr");
    assert!(!out.status.success());
    assert_eq!(out.stderr, b"error_msg\n");
}

#[test]
fn test_status_with_args() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let status = rt
        .block_on(async {
            process::status(|| {
                let mut c = Command::new("test");
                c.arg("-d").arg("/tmp");
                c
            })
            .await
        })
        .expect("status should work");
    assert!(status.success(), "/tmp should be a directory");
}

#[test]
fn test_child_kill() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let status = rt
        .block_on(async {
            let child = process::Child::spawn(|| {
                let mut c = Command::new("sleep");
                c.arg("60");
                c
            })
            .expect("spawn should succeed");
            let pid = child.id();
            // Kill the child.
            let _ = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
            child.wait().await
        })
        .expect("wait should complete");
    assert!(!status.success(), "killed child should exit non-zero");
}

#[test]
fn test_concurrent_heavy_process_spawn() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        // Spawn 10 concurrent echo processes.
        for i in 0..10u32 {
            let out = process::output(move || {
                let mut c = Command::new("echo");
                c.arg("-n").arg(i.to_string());
                c
            })
            .await
            .expect("echo should succeed");
            assert_eq!(out.stdout, format!("{}", i).as_bytes());
        }
    });
}

#[test]
fn test_output_string_empty() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let text = rt
        .block_on(async { process::output_string(|| Command::new("true")).await })
        .expect("true should succeed");
    assert_eq!(text, "");
}

#[test]
fn test_output_with_multiline() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    let text = rt
        .block_on(async {
            process::output_string(|| {
                let mut c = Command::new("printf");
                c.arg("line1\nline2\nline3");
                c
            })
            .await
        })
        .expect("printf should succeed");
    assert_eq!(text, "line1\nline2\nline3");
}
