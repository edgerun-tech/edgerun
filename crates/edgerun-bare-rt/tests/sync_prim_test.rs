use edgerun_bare_rt::{SyncMutex, Condvar};

#[test]
fn condvar_new() {
    let condvar = Condvar::new();
    let _ = condvar;
}

#[test]
fn sync_mutex_new() {
    let mutex = SyncMutex::new(0i32);
    let _ = mutex;
}

#[test]
fn sync_rwlock_new() {
    use edgerun_bare_rt::SyncRwLock;
    let rwlock = SyncRwLock::new(0i32);
    let _ = rwlock;
}