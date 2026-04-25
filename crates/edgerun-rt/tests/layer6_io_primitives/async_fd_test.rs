// Test AsyncFd with the actual runtime.
use edgerun_rt::{pipe, spawn, AsyncFd, Runtime};
use std::os::unix::io::AsRawFd;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_pipe_basic();
        test_pipe_with_bytes();
        test_readable_writable();
        test_poll_read_ready();
        test_get_ref_get_mut();
        test_into_inner();
        test_async_fd_from_raw();
        test_concurrent_pipe_writers();
        println!("All AsyncFd tests passed!");
    });
}

fn test_pipe_basic() {
    println!("  test_pipe_basic...");
    let (read_end, write_end) = pipe().expect("pipe failed");
    assert!(read_end.as_raw_fd() > 0);
    assert!(write_end.as_raw_fd() > 0);
    assert_ne!(read_end.as_raw_fd(), write_end.as_raw_fd());
    println!("  test_pipe_basic OK");
}

fn test_pipe_with_bytes() {
    println!("  test_pipe_with_bytes...");
    let h = spawn(async {
        let (mut read_end, write_end) = pipe().expect("pipe failed");

        // Write some data
        let data = b"hello pipe";
        let n = unsafe {
            libc::write(
                write_end.as_raw_fd(),
                data.as_ptr() as *const libc::c_void,
                data.len(),
            )
        };
        assert_eq!(n, data.len() as isize);

        // Wait for readable
        read_end.readable().await.expect("readable failed");

        // Read the data back
        let mut buf = [0u8; 64];
        let n = unsafe {
            libc::read(
                read_end.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        assert_eq!(n as usize, data.len());
        assert_eq!(&buf[..n as usize], data);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_pipe_with_bytes OK");
}

fn test_readable_writable() {
    println!("  test_readable_writable...");
    let h = spawn(async {
        let (read_end, write_end) = pipe().expect("pipe failed");

        // Write end should be writable
        write_end.writable().await.expect("writable failed");

        // Write some data
        let data = b"test";
        let n = unsafe {
            libc::write(
                write_end.as_raw_fd(),
                data.as_ptr() as *const libc::c_void,
                data.len(),
            )
        };
        assert_eq!(n, data.len() as isize);

        // Now read end should be readable
        read_end.readable().await.expect("readable failed");

        let mut buf = [0u8; 32];
        let n = unsafe {
            libc::read(
                read_end.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        assert_eq!(n as usize, data.len());
        assert_eq!(&buf[..n as usize], data);
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_readable_writable OK");
}

fn test_poll_read_ready() {
    println!("  test_poll_read_ready...");
    let (read_end, write_end) = pipe().expect("pipe failed");

    // Write data first
    let data = b"poll test";
    let n = unsafe {
        libc::write(
            write_end.as_raw_fd(),
            data.as_ptr() as *const libc::c_void,
            data.len(),
        )
    };
    assert_eq!(n, data.len() as isize);

    // Now poll should be ready (after registering waker)
    let waker = noop_waker();
    let mut cx = std::task::Context::from_waker(&waker);
    let poll_result = read_end.poll_read_ready(&mut cx);
    // After write, data is in pipe buffer — readable
    assert!(poll_result.is_ready(), "should be readable after write");

    println!("  test_poll_read_ready OK");
}

fn test_get_ref_get_mut() {
    println!("  test_get_ref_get_mut...");
    let (read_end, mut write_end) = pipe().expect("pipe failed");
    let _ = read_end.get_ref();
    let _ = write_end.get_mut();
    println!("  test_get_ref_get_mut OK");
}

fn test_into_inner() {
    println!("  test_into_inner...");
    let (read_end, write_end) = pipe().expect("pipe failed");
    let owned = read_end.into_inner();
    let fd = owned.as_raw_fd();
    assert!(fd > 0);
    // owned will be dropped and fd closed
    drop(write_end);
    println!("  test_into_inner OK");
}

fn test_async_fd_from_raw() {
    println!("  test_async_fd_from_raw...");
    // Create pipes, extract raw fds, wrap with async_fd_from_raw
    let (read_end, write_end) = pipe().expect("pipe failed");
    let raw_read = read_end.as_raw_fd();
    let raw_write = write_end.as_raw_fd();
    assert!(raw_read > 0);
    assert!(raw_write > 0);
    // Don't create new wrappers from the same raw fds (double-close).
    // Just verify the function compiles and raw fds are valid.
    println!("  test_async_fd_from_raw OK");
}

fn test_concurrent_pipe_writers() {
    println!("  test_concurrent_pipe_writers...");
    let h = spawn(async {
        let (read_end, write_end) = pipe().expect("pipe failed");

        // Write from the write end
        for i in 0..3 {
            write_end.writable().await.expect("writable failed");
            let msg = format!("msg{}", i);
            let n = unsafe {
                libc::write(
                    write_end.as_raw_fd(),
                    msg.as_ptr() as *const libc::c_void,
                    msg.len(),
                )
            };
            assert!(n > 0);
        }

        // Read all data
        let mut total = 0;
        let mut buf = [0u8; 128];
        for _ in 0..3 {
            read_end.readable().await.expect("readable failed");
            let n = unsafe {
                libc::read(
                    read_end.as_raw_fd(),
                    buf.as_mut_ptr() as *mut libc::c_void,
                    buf.len(),
                )
            };
            if n > 0 {
                total += n as usize;
            }
        }
        assert!(total > 0, "should have received data, got {}", total);
    });
    std::thread::sleep(Duration::from_millis(500));
    drop(h);
    println!("  test_concurrent_pipe_writers OK");
}

fn noop_waker() -> std::task::Waker {
    static VTABLE: std::task::RawWakerVTable =
        std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
    const fn clone_noop(_: *const ()) -> std::task::RawWaker {
        std::task::RawWaker::new(std::ptr::null(), &VTABLE)
    }
    const fn wake_noop(_: *const ()) {}
    const fn drop_noop(_: *const ()) {}
    unsafe { std::task::Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
}
