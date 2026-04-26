use edgerun_bare_rt::interval;
use core::time::Duration;

#[test]
fn interval_creates() {
    let _ = interval(Duration::from_secs(1));
}