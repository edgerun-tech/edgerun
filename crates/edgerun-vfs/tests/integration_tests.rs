//! Integration tests for edgerun-vfs

use edgerun_vfs::{FineGrainedVFS, VirtualFileSystem};
use std::path::Path;

struct TempDir {
    path: std::path::PathBuf,
}

impl TempDir {
    fn new() -> std::io::Result<Self> {
        let base = std::env::temp_dir();
        for id in 0..100 {
            let path = base.join(format!("edgerun-vfs-itest-{}-{}", std::process::id(), id));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "could not allocate temporary test directory",
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn create_test_files(dir: &TempDir, count: usize) {
    for i in 0..count {
        let content = format!("fn test_{}() {{\n// line 1\n// line 2\n}}", i);
        std::fs::write(dir.path().join(format!("file_{}.rs", i)), content).unwrap();
    }
}

#[test]
fn test_vfs_load_and_read() {
    let tmp = TempDir::new().unwrap();
    create_test_files(&tmp, 10);

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    assert_eq!(vfs.files().count(), 10);

    let content = vfs.read_str(Path::new("file_0.rs")).unwrap();
    assert!(content.contains("fn test_0()"));
}

#[test]
fn test_vfs_load_binary_files() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("text.txt"), "hello world").unwrap();
    std::fs::write(tmp.path().join("binary.dat"), vec![0u8, 255, 128, 64]).unwrap();

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    assert_eq!(vfs.files().count(), 2);
    assert!(vfs.is_text(Path::new("text.txt")));
    assert!(!vfs.is_text(Path::new("binary.dat")));

    let bytes = vfs.read(Path::new("binary.dat")).unwrap();
    assert_eq!(&*bytes, &[0u8, 255, 128, 64]);
}

#[test]
fn test_vfs_edit_and_persist() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

    let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    vfs.edit(Path::new("test.txt"), |c| {
        c.push_str(" - modified");
        Ok(())
    })
    .unwrap();

    assert!(vfs
        .read_str(Path::new("test.txt"))
        .unwrap()
        .contains("modified"));

    let result = vfs.persist().unwrap();
    assert_eq!(result.persisted, 1);

    let disk_content = std::fs::read(tmp.path().join("test.txt")).unwrap();
    assert_eq!(disk_content, b"initial - modified");
}

#[test]
fn test_vfs_grep_across_files() {
    let tmp = TempDir::new().unwrap();
    create_test_files(&tmp, 100);

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let matches = vfs.grep("fn test_");
    assert_eq!(matches.len(), 100);

    let matches = vfs.grep("line 1");
    assert_eq!(matches.len(), 100);
}

#[test]
fn test_vfs_grep_skips_binary() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("hello.txt"), "fn main() {}").unwrap();
    std::fs::write(tmp.path().join("data.bin"), vec![0u8, 159, 146, 150]).unwrap();

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let matches = vfs.grep("fn");
    assert_eq!(matches.len(), 1);
    assert!(matches[0].path.to_str().unwrap().contains("hello.txt"));
}

#[test]
fn test_vfs_glob_patterns() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("file1.rs"), "content").unwrap();
    std::fs::write(tmp.path().join("file2.rs"), "content").unwrap();
    std::fs::write(tmp.path().join("file3.txt"), "content").unwrap();

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let rs_files = vfs.glob("*.rs");
    assert_eq!(rs_files.len(), 2);

    let txt_files = vfs.glob("*.txt");
    assert_eq!(txt_files.len(), 1);
}

#[test]
fn test_vfs_delete_and_rollback() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

    let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    assert!(vfs.exists(Path::new("test.txt")));

    vfs.delete(Path::new("test.txt"));
    assert!(!vfs.exists(Path::new("test.txt")));
    assert!(vfs.read_str(Path::new("test.txt")).is_none());

    let result = vfs.persist().unwrap();
    assert_eq!(result.deleted, 1);
    assert!(!tmp.path().join("test.txt").exists());
}

#[test]
fn test_fine_grained_concurrent_operations() {
    let tmp = TempDir::new().unwrap();
    create_test_files(&tmp, 5);

    let vfs = FineGrainedVFS::load(tmp.path()).unwrap();

    let vfs = std::sync::Arc::new(vfs);
    let mut handles = vec![];

    for i in 0..5 {
        let vfs = vfs.clone();
        let handle = std::thread::spawn(move || {
            vfs.edit(Path::new(&format!("file_{}.rs", i)), |c| {
                c.push_str(&format!("\n// edited by thread {}", i));
                Ok(())
            })
        });
        handles.push(handle);
    }

    for handle in handles {
        assert!(handle.join().unwrap().is_ok());
    }

    for i in 0..5 {
        let content = vfs.read_str(Path::new(&format!("file_{}.rs", i))).unwrap();
        assert!(content.contains(&format!("edited by thread {}", i)));
    }
}

#[test]
fn test_vfs_memory_stats() {
    let tmp = TempDir::new().unwrap();
    create_test_files(&tmp, 10);

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let stats = vfs.memory_stats();

    assert_eq!(stats.file_count, 10);
    assert!(stats.memory_bytes > 0);
    assert!(stats.avg_file_size > 0);
}

#[test]
fn test_vfs_changes_tracking() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

    let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let changes = vfs.changes();
    assert!(changes.added.is_empty());
    assert!(changes.modified.is_empty());

    vfs.write(Path::new("test.txt"), "modified".to_string())
        .unwrap();
    let changes = vfs.changes();
    assert!(!changes.modified.is_empty());

    vfs.write_bytes(Path::new("new.bin"), vec![0u8, 1, 2, 3])
        .unwrap();
    let changes = vfs.changes();
    assert!(!changes.added.is_empty());
}

#[test]
fn test_vfs_total_lines() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("test.txt"), "line1\nline2\nline3\n").unwrap();

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    assert_eq!(vfs.total_lines(), 3);
}

#[test]
fn test_vfs_find_files_containing() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("file1.txt"), "hello world").unwrap();
    std::fs::write(tmp.path().join("file2.txt"), "goodbye world").unwrap();
    std::fs::write(tmp.path().join("file3.txt"), "hello again").unwrap();

    let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let files = vfs.find_files_containing("hello");
    assert_eq!(files.len(), 2);

    let files = vfs.find_files_containing("WORLD");
    assert_eq!(files.len(), 2);
}

#[test]
fn test_vfs_write_and_read_binary() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("data.bin"), vec![0u8; 100]).unwrap();

    let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

    // Read binary file
    let bytes = vfs.read(Path::new("data.bin")).unwrap();
    assert_eq!(bytes.len(), 100);

    // Write binary data that is NOT valid UTF-8
    vfs.write_bytes(Path::new("data.bin"), vec![0u8, 159, 146, 150])
        .unwrap();
    let bytes = vfs.read(Path::new("data.bin")).unwrap();
    assert_eq!(&*bytes, &[0u8, 159, 146, 150]);

    // read_str should return None for non-UTF-8 content
    assert!(vfs.read_str(Path::new("data.bin")).is_none());
}

#[test]
fn test_vfs_edit_binary_file_fails() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("data.bin"), vec![0u8, 255, 128]).unwrap();

    let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
    let result = vfs.edit(Path::new("data.bin"), |_c| Ok(()));
    assert!(result.is_err());
}
