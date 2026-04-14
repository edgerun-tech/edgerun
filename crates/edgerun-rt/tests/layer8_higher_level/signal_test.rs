// Test Unix signal handling with the actual runtime.
use edgerun_rt::{Signal, SignalKind, signal, Runtime, AsyncReadExt, spawn};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_signal_kind_constructors();
        test_signal_create_succeeds();
        test_signal_clone_shares_fd();
        test_signal_send_and_recv_same_thread();
        test_signal_async_read_returns_siginfo();
        println!("All signal tests passed!");
    });
}

fn test_signal_kind_constructors() {
    println!("  test_signal_kind_constructors...");
    assert_eq!(SignalKind::terminate().as_raw(), 15);
    assert_eq!(SignalKind::interrupt().as_raw(), 2);
    assert_eq!(SignalKind::hangup().as_raw(), 1);
    assert_eq!(SignalKind::alarm().as_raw(), 14);
    assert_eq!(SignalKind::child().as_raw(), 17);
    assert_eq!(SignalKind::user_defined1().as_raw(), 10);
    assert_eq!(SignalKind::user_defined2().as_raw(), 12);
    assert_eq!(SignalKind::winch().as_raw(), 28);
    println!("  test_signal_kind_constructors OK");
}

fn test_signal_create_succeeds() {
    println!("  test_signal_create_succeeds...");
    let h = spawn(async {
        let sig = signal(SignalKind::user_defined1()).expect("create signal failed");
        drop(sig);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_signal_create_succeeds OK");
}

fn test_signal_clone_shares_fd() {
    println!("  test_signal_clone_shares_fd...");
    let h = spawn(async {
        let sig = signal(SignalKind::user_defined1()).expect("create signal failed");
        let sig2 = sig.clone();
        drop(sig);
        drop(sig2);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_signal_clone_shares_fd OK");
}

fn test_signal_send_and_recv_same_thread() {
    println!("  test_signal_send_and_recv_same_thread...");
    let h = spawn(async {
        // Create signal listener — masks SIGUSR1 for this thread.
        let mut sig = signal(SignalKind::user_defined1()).expect("create signal failed");
        
        // Send SIGUSR1 to THIS thread specifically using pthread_kill.
        // This ensures the signal goes to the thread with the mask,
        // so it's delivered via signalfd (not the default handler).
        std::thread::sleep(Duration::from_millis(50));
        let tid = unsafe { libc::pthread_self() };
        let ret = unsafe { libc::pthread_kill(tid, libc::SIGUSR1) };
        assert_eq!(ret, 0, "pthread_kill failed: {}", ret);
        
        // Receive the signal via signalfd.
        let signo = sig.recv().await.expect("recv failed");
        assert_eq!(signo, Some(libc::SIGUSR1), "expected SIGUSR1, got {:?}", signo);
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_signal_send_and_recv_same_thread OK");
}

fn test_signal_async_read_returns_siginfo() {
    println!("  test_signal_async_read_returns_siginfo...");
    let h = spawn(async {
        let mut sig = signal(SignalKind::user_defined2()).expect("create signal failed");
        
        // Send SIGUSR2 to this thread.
        std::thread::sleep(Duration::from_millis(50));
        let tid = unsafe { libc::pthread_self() };
        let ret = unsafe { libc::pthread_kill(tid, libc::SIGUSR2) };
        assert_eq!(ret, 0, "pthread_kill failed: {}", ret);
        
        // Read the signalfd_siginfo struct (128 bytes).
        let mut buf = [0u8; 256];
        let n = sig.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 128, "expected 128 bytes for signalfd_siginfo");
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_signal_async_read_returns_siginfo OK");
}
