// Test async process module with the actual runtime.
use edgerun_rt::{process, Runtime};
use std::process::Command;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_output_echo();
        test_output_true();
        test_status_success();
        test_status_failure();
        test_output_string();
        test_child_spawn_and_wait();
        test_child_id();
        test_concurrent_commands();
        println!("All process tests passed!");
    });
}

fn test_output_echo() {
    println!("  test_output_echo...");
    let h = edgerun_rt::spawn(async {
        let out = process::output(|| {
            let mut c = Command::new("echo");
            c.arg("-n").arg("hello");
            c
        })
        .await
        .expect("output failed");
        assert_eq!(out.stdout, b"hello");
        assert!(out.status.success());
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_output_echo OK");
}

fn test_output_true() {
    println!("  test_output_true...");
    let h = edgerun_rt::spawn(async {
        let out = process::output(|| Command::new("true"))
            .await
            .expect("output failed");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_output_true OK");
}

fn test_status_success() {
    println!("  test_status_success...");
    let h = edgerun_rt::spawn(async {
        let status = process::status(|| Command::new("true"))
            .await
            .expect("status failed");
        assert!(status.success());
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_status_success OK");
}

fn test_status_failure() {
    println!("  test_status_failure...");
    let h = edgerun_rt::spawn(async {
        let status = process::status(|| Command::new("false"))
            .await
            .expect("status failed");
        assert!(!status.success());
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_status_failure OK");
}

fn test_output_string() {
    println!("  test_output_string...");
    let h = edgerun_rt::spawn(async {
        let text = process::output_string(|| {
            let mut c = Command::new("printf");
            c.arg("hello world");
            c
        })
        .await
        .expect("output_string failed");
        assert_eq!(text, "hello world");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_output_string OK");
}

fn test_child_spawn_and_wait() {
    println!("  test_child_spawn_and_wait...");
    let h = edgerun_rt::spawn(async {
        let child = process::Child::spawn(|| {
            let mut c = Command::new("sleep");
            c.arg("0.1");
            c
        })
        .expect("spawn failed");
        let status = child.wait().await.expect("wait failed");
        assert!(status.success());
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_child_spawn_and_wait OK");
}

fn test_child_id() {
    println!("  test_child_id...");
    let h = edgerun_rt::spawn(async {
        let child = process::Child::spawn(|| {
            let mut c = Command::new("sleep");
            c.arg("0.1");
            c
        })
        .expect("spawn failed");
        let pid = child.id();
        assert!(pid > 0, "child PID should be > 0, got {}", pid);
        let status = child.wait().await.expect("wait failed");
        assert!(status.success());
        println!("  child PID: {}", pid);
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_child_id OK");
}

fn test_concurrent_commands() {
    println!("  test_concurrent_commands...");
    let h = edgerun_rt::spawn(async {
        let a = process::output(|| {
            let mut c = Command::new("echo");
            c.arg("-n").arg("a");
            c
        });
        let b = process::output(|| {
            let mut c = Command::new("echo");
            c.arg("-n").arg("b");
            c
        });
        let c = process::output(|| {
            let mut c = Command::new("echo");
            c.arg("-n").arg("c");
            c
        });
        let (a, b, c) = edgerun_rt::join!(a, b, c);

        assert_eq!(a.expect("a failed").stdout, b"a");
        assert_eq!(b.expect("b failed").stdout, b"b");
        assert_eq!(c.expect("c failed").stdout, b"c");
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_concurrent_commands OK");
}
