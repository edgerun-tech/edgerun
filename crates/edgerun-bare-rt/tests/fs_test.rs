use edgerun_bare_rt::{Dir, Error as FsError, File};

#[test]
fn file_new() {
    let file: File = File::new();
    let _ = file;
}

#[test]
fn dir_new() {
    let dir: Dir = Dir::new();
    let _ = dir;
}
