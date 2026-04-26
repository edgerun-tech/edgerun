use edgerun_bare_rt::{MissedTickBehavior, Interval};

#[test]
fn missed_tick_behavior() {
    let _ = MissedTickBehavior::skip();
    let _ = MissedTickBehavior::backlog();
}