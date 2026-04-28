#[test]
fn nested_spawn_should_complete() {
    let outer = edgerun_rt::spawn(async {
        let inner = edgerun_rt::spawn(async { 10 + 5 });
        inner.await.unwrap()
    });

    let mut spins = 0usize;
    while !outer.is_finished() {
        edgerun_rt::run_queue();
        spins += 1;
        assert!(spins < 1000, "nested spawn did not complete within 1000 queue ticks");
        if !outer.is_finished() {
            assert!(edgerun_rt::pending() > 0, "runtime became idle before completion");
        }
    }

    assert!(outer.is_finished());
    assert_eq!(outer.blocking_recv().unwrap(), 15);
}
