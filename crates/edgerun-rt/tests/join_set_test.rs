use edgerun_rt::JoinSet;

#[test]
fn join_set_new() {
    let set: JoinSet<i32> = JoinSet::new();
    let _ = set;
}

#[test]
fn join_set_spawns_and_tracks() {
    let mut set: JoinSet<i32> = JoinSet::new();

    let handle = set
        .spawn(async { 42 })
        .unwrap_or_else(|| panic!("joinset should accept task"));
    assert_eq!(set.len(), 1);
    set.abort();
    assert!(handle.is_finished());
    assert!(set.is_empty());
}
