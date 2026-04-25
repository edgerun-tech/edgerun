// Test Cursor with the actual runtime.
use edgerun_rt::{spawn, AsyncReadExt, AsyncWriteExt, Cursor, Runtime};
use std::io::SeekFrom;
use std::time::Duration;

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async {
        test_cursor_read_slice();
        test_cursor_read_partial();
        test_cursor_read_eof();
        test_cursor_position();
        test_cursor_seek_start();
        test_cursor_seek_current();
        test_cursor_seek_end();
        test_cursor_write_vec();
        test_cursor_write_extend();
        test_cursor_read_write_vec();
        test_cursor_get_ref();
        test_cursor_into_inner();
        println!("All Cursor tests passed!");
    });
}

fn test_cursor_read_slice() {
    println!("  test_cursor_read_slice...");
    let h = spawn(async {
        let data = b"hello cursor";
        let mut cursor = Cursor::new(data.as_slice());
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"hello cursor");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_read_slice OK");
}

fn test_cursor_read_partial() {
    println!("  test_cursor_read_partial...");
    let h = spawn(async {
        let data = b"hello cursor";
        let mut cursor = Cursor::new(data.as_slice());
        let mut buf = [0u8; 5];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
        assert_eq!(cursor.position(), 5);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_read_partial OK");
}

fn test_cursor_read_eof() {
    println!("  test_cursor_read_eof...");
    let h = spawn(async {
        let data = b"hi";
        let mut cursor = Cursor::new(data.as_slice());
        let mut buf = [0u8; 64];
        // First read gets "hi"
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 2);
        // Second read should be EOF
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(n, 0);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_read_eof OK");
}

fn test_cursor_position() {
    println!("  test_cursor_position...");
    let h = spawn(async {
        let data = b"abcdefgh";
        let mut cursor = Cursor::new(data.as_slice());
        cursor.set_position(3);
        assert_eq!(cursor.position(), 3);
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"defgh");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_position OK");
}

fn test_cursor_seek_start() {
    println!("  test_cursor_seek_start...");
    let h = spawn(async {
        let data = b"abcdefgh";
        let mut cursor = Cursor::new(data.as_slice());
        let pos = cursor.seek(SeekFrom::Start(4)).expect("seek failed");
        assert_eq!(pos, 4);
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"efgh");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_seek_start OK");
}

fn test_cursor_seek_current() {
    println!("  test_cursor_seek_current...");
    let h = spawn(async {
        let data = b"abcdefgh";
        let mut cursor = Cursor::new(data.as_slice());
        // Move forward 2
        let pos = cursor.seek(SeekFrom::Current(2)).expect("seek failed");
        assert_eq!(pos, 2);
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"cdefgh");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_seek_current OK");
}

fn test_cursor_seek_end() {
    println!("  test_cursor_seek_end...");
    let h = spawn(async {
        let data = b"abcdefgh";
        let mut cursor = Cursor::new(data.as_slice());
        // Seek to 3 bytes from end
        let pos = cursor.seek(SeekFrom::End(-3)).expect("seek failed");
        assert_eq!(pos, 5);
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"fgh");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_seek_end OK");
}

fn test_cursor_write_vec() {
    println!("  test_cursor_write_vec...");
    let h = spawn(async {
        let mut cursor = Cursor::new(vec![0u8; 16]);
        cursor.set_position(2);
        cursor.write_all(b"hello").await.expect("write failed");
        assert_eq!(cursor.position(), 7);
        let data = cursor.into_inner();
        assert_eq!(&data[2..7], b"hello");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_write_vec OK");
}

fn test_cursor_write_extend() {
    println!("  test_cursor_write_extend...");
    let h = spawn(async {
        let mut cursor = Cursor::new(Vec::new());
        cursor.write_all(b"extend me").await.expect("write failed");
        let data = cursor.into_inner();
        assert_eq!(&data, b"extend me");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_write_extend OK");
}

fn test_cursor_read_write_vec() {
    println!("  test_cursor_read_write_vec...");
    let h = spawn(async {
        let mut cursor = Cursor::new(b"original data".to_vec());

        // Read first 8 bytes
        let mut buf = [0u8; 64];
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"original data");

        // Seek to beginning and overwrite
        cursor.seek(SeekFrom::Start(0)).expect("seek failed");
        cursor.write_all(b"new").await.expect("write failed");

        // Read back
        cursor.seek(SeekFrom::Start(0)).expect("seek failed");
        let n = cursor.read(&mut buf).await.expect("read failed");
        assert_eq!(&buf[..n], b"newginal data");
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_read_write_vec OK");
}

fn test_cursor_get_ref() {
    println!("  test_cursor_get_ref...");
    let h = spawn(async {
        let data = vec![1, 2, 3];
        let cursor = Cursor::new(data.clone());
        assert_eq!(cursor.get_ref(), &data);
        let mut cursor2 = Cursor::new(data.clone());
        assert_eq!(cursor2.get_mut(), &data);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_get_ref OK");
}

fn test_cursor_into_inner() {
    println!("  test_cursor_into_inner...");
    let h = spawn(async {
        let data = vec![4, 5, 6];
        let cursor = Cursor::new(data.clone());
        let inner = cursor.into_inner();
        assert_eq!(inner, data);
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(h);
    println!("  test_cursor_into_inner OK");
}
