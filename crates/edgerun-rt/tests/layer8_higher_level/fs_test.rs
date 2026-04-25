// Test async fs module with the actual runtime.
use edgerun_rt::{fs, Runtime};
use std::time::Duration;

fn make_temp_dir() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let id = std::process::id();
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("edgerun_rt_fs_test_{}_{}", id, n))
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_dir_all(path);
}

fn main() {
    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        test_write_and_read();
        test_read_to_string();
        test_remove_file();
        test_rename_file();
        test_create_dir_and_remove();
        test_create_dir_all_and_remove();
        test_copy_file();
        test_exists();
        test_metadata();
        test_canonicalize();
        println!("All fs tests passed!");
    });
}

fn test_write_and_read() {
    println!("  test_write_and_read...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&path, b"hello fs").await.expect("write failed");
        let data = fs::read(&path).await.expect("read failed");
        assert_eq!(&data, b"hello fs");
    });
    println!("  test_write_and_read OK");
}

fn test_read_to_string() {
    println!("  test_read_to_string...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("text.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&path, "unicode: 你好 🌍")
            .await
            .expect("write failed");
        let text = fs::read_to_string(&path)
            .await
            .expect("read_to_string failed");
        assert_eq!(text, "unicode: 你好 🌍");
    });
    println!("  test_read_to_string OK");
}

fn test_remove_file() {
    println!("  test_remove_file...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("remove_me.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&path, b"delete me").await.expect("write failed");
        assert!(path.exists());
        fs::remove_file(&path).await.expect("remove_file failed");
        assert!(!path.exists());
    });
    println!("  test_remove_file OK");
}

fn test_rename_file() {
    println!("  test_rename_file...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let old_path = dir.join("old.txt");
    let new_path = dir.join("new.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&old_path, b"renamed")
            .await
            .expect("write failed");
        fs::rename(&old_path, &new_path)
            .await
            .expect("rename failed");
        assert!(!old_path.exists());
        assert!(new_path.exists());
        let data = fs::read(&new_path).await.expect("read failed");
        assert_eq!(&data, b"renamed");
    });
    println!("  test_rename_file OK");
}

fn test_create_dir_and_remove() {
    println!("  test_create_dir_and_remove...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let subdir = dir.join("subdir");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::create_dir(&subdir).await.expect("create_dir failed");
        assert!(subdir.is_dir());
        fs::remove_dir(&subdir).await.expect("remove_dir failed");
        assert!(!subdir.exists());
    });
    println!("  test_create_dir_and_remove OK");
}

fn test_create_dir_all_and_remove() {
    println!("  test_create_dir_all_and_remove...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deep = dir.join("a/b/c/d");
    let base_a = dir.join("a");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::create_dir_all(&deep)
            .await
            .expect("create_dir_all failed");
        assert!(deep.is_dir());
        fs::remove_dir_all(&base_a)
            .await
            .expect("remove_dir_all failed");
        assert!(!base_a.exists());
    });
    println!("  test_create_dir_all_and_remove OK");
}

fn test_copy_file() {
    println!("  test_copy_file...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("src.bin");
    let dst = dir.join("dst.bin");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&src, b"binary data").await.expect("write failed");
        let n = fs::copy(&src, &dst).await.expect("copy failed");
        assert_eq!(n, 11);
        let data = fs::read(&dst).await.expect("read failed");
        assert_eq!(&data, b"binary data");
    });
    println!("  test_copy_file OK");
}

fn test_exists() {
    println!("  test_exists...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("exists.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        assert!(!fs::exists(&path).await);
        fs::write(&path, b"here").await.expect("write failed");
        assert!(fs::exists(&path).await);
        fs::remove_file(&path).await.expect("remove failed");
        assert!(!fs::exists(&path).await);
    });
    println!("  test_exists OK");
}

fn test_metadata() {
    println!("  test_metadata...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("meta.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        let content = b"metadata test content";
        fs::write(&path, content).await.expect("write failed");
        let meta = fs::metadata(&path).await.expect("metadata failed");
        assert!(meta.is_file());
        assert_eq!(meta.len(), content.len() as u64);
    });
    println!("  test_metadata OK");
}

fn test_canonicalize() {
    println!("  test_canonicalize...");
    let dir = make_temp_dir();
    let _cleanup = Cleanup(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("canon.txt");

    let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
    rt.block_on(async move {
        fs::write(&path, b"canonical").await.expect("write failed");
        let canon = fs::canonicalize(&path).await.expect("canonicalize failed");
        assert!(canon.is_absolute());
        assert!(canon.ends_with("canon.txt"));
    });
    println!("  test_canonicalize OK");
}

struct Cleanup<'a>(&'a std::path::Path);
impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        cleanup(self.0);
    }
}
