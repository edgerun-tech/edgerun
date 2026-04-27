use edgerun_rt::{Interval, MissedTickBehavior};

#[test]
fn missed_tick_behavior() {
    let _ = MissedTickBehavior::skip();
    let _ = MissedTickBehavior::backlog();
}
