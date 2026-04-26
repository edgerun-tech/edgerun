use edgerun_bare_rt::JoinSet;

#[test]
fn join_set_new() {
    let set: JoinSet<i32> = JoinSet::new();
    let _ = set;
}