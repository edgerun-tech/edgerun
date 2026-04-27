use core::time::Duration;
use edgerun_rt::interval;

#[test]
fn interval_creates() {
    let _ = interval(Duration::from_secs(1));
}
