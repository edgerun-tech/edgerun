// Test read_line and Lines with the actual runtime.
use edgerun_rt::{spawn, AsyncReadExt, AsyncWriteExt, Cursor, DuplexStream, Runtime};
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_read_line_basic();
        test_read_line_no_trailing_newline();
        test_read_line_empty();
        test_read_line_crlf();
        test_lines_basic();
        test_lines_no_trailing_newline();
        test_lines_crlf();
        test_read_line_cursor();
        test_lines_cursor();
        test_read_line_multiple_calls();
        println!("All read_line/Lines tests passed!");
    });
}

fn test_read_line_basic() {
    println!("  test_read_line_basic...");
    let h = spawn(async {
        let (mut a, b) = DuplexStream::channel();
        let mut reader = b;

        a.write_all(b"first line\nsecond line\n")
            .await
            .expect("write failed");

        let mut line = String::new();
        let n = reader.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "first line\n");
        assert_eq!(n, 11);

        line.clear();
        let n = reader.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "second line\n");
        assert_eq!(n, 12);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_read_line_basic OK");
}

fn test_read_line_no_trailing_newline() {
    println!("  test_read_line_no_trailing_newline...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();
        a.write_all(b"no newline").await.expect("write failed");
        a.shutdown().await.expect("shutdown failed");

        let mut line = String::new();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "no newline");
        assert_eq!(n, 10);

        // Next read should be EOF
        line.clear();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(n, 0);
        assert!(line.is_empty());
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_read_line_no_trailing_newline OK");
}

fn test_read_line_empty() {
    println!("  test_read_line_empty...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();
        a.write_all(b"\n").await.expect("write failed");

        let mut line = String::new();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "\n");
        assert_eq!(n, 1);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_read_line_empty OK");
}

fn test_read_line_crlf() {
    println!("  test_read_line_crlf...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();
        a.write_all(b"crlf line\r\n").await.expect("write failed");

        let mut line = String::new();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        // "crlf line\r\n" = 11 bytes
        assert_eq!(line, "crlf line\r\n");
        assert_eq!(n, 11);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_read_line_crlf OK");
}

fn test_lines_basic() {
    println!("  test_lines_basic...");
    let h = spawn(async {
        let (mut a, b) = DuplexStream::channel();
        a.write_all(b"line1\nline2\nline3\n")
            .await
            .expect("write failed");
        a.shutdown().await.expect("shutdown failed");

        let mut lines = b.lines();
        let l1 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l1, "line1");
        let l2 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l2, "line2");
        let l3 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l3, "line3");
        let l4 = lines.next_line().await.expect("next_line failed");
        assert!(l4.is_none(), "expected EOF, got {:?}", l4);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_lines_basic OK");
}

fn test_lines_no_trailing_newline() {
    println!("  test_lines_no_trailing_newline...");
    let h = spawn(async {
        let (mut a, b) = DuplexStream::channel();
        a.write_all(b"line1\nline2").await.expect("write failed");
        a.shutdown().await.expect("shutdown failed");

        let mut lines = b.lines();
        let l1 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l1, "line1");
        let l2 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l2, "line2");
        let l3 = lines.next_line().await.expect("next_line failed");
        assert!(l3.is_none(), "expected EOF, got {:?}", l3);
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_lines_no_trailing_newline OK");
}

fn test_lines_crlf() {
    println!("  test_lines_crlf...");
    let h = spawn(async {
        let (mut a, b) = DuplexStream::channel();
        a.write_all(b"crlf1\r\ncrlf2\r\n")
            .await
            .expect("write failed");
        a.shutdown().await.expect("shutdown failed");

        let mut lines = b.lines();
        let l1 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l1, "crlf1");
        let l2 = lines
            .next_line()
            .await
            .expect("next_line failed")
            .expect("expected line");
        assert_eq!(l2, "crlf2");
    });
    std::thread::sleep(Duration::from_millis(200));
    drop(h);
    println!("  test_lines_crlf OK");
}

fn test_read_line_cursor() {
    println!("  test_read_line_cursor...");
    let h = spawn(async {
        let mut cursor = Cursor::new(b"alpha\nbeta\ngamma".as_slice());
        let mut line = String::new();

        let n = cursor.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "alpha\n");
        assert_eq!(n, 6);

        line.clear();
        let n = cursor.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "beta\n");
        assert_eq!(n, 5);

        line.clear();
        let n = cursor.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "gamma");
        assert_eq!(n, 5);
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_read_line_cursor OK");
}

fn test_lines_cursor() {
    println!("  test_lines_cursor...");
    let h = spawn(async {
        let cursor = Cursor::new(b"one\ntwo\nthree\n".as_slice());
        let mut lines = cursor.lines();

        assert_eq!(lines.next_line().await.unwrap().unwrap(), "one");
        assert_eq!(lines.next_line().await.unwrap().unwrap(), "two");
        assert_eq!(lines.next_line().await.unwrap().unwrap(), "three");
        assert!(lines.next_line().await.unwrap().is_none());
    });
    std::thread::sleep(Duration::from_millis(100));
    drop(h);
    println!("  test_lines_cursor OK");
}

fn test_read_line_multiple_calls() {
    println!("  test_read_line_multiple_calls...");
    let h = spawn(async {
        let (mut a, mut b) = DuplexStream::channel();

        // Write lines one at a time
        a.write_all(b"line1\n").await.expect("write1 failed");
        let mut line = String::new();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "line1\n");

        a.write_all(b"line2\n").await.expect("write2 failed");
        line.clear();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "line2\n");

        a.write_all(b"line3").await.expect("write3 failed");
        a.shutdown().await.expect("shutdown failed");
        line.clear();
        let n = b.read_line(&mut line).await.expect("read_line failed");
        assert_eq!(line, "line3");
    });
    std::thread::sleep(Duration::from_millis(300));
    drop(h);
    println!("  test_read_line_multiple_calls OK");
}
