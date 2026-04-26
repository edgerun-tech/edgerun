use edgerun_bare_rt::{interval_at, timeout};
use core::time::Duration;

#[test]
fn interval_at_creates() {
    use edgerun_bare_rt::Instant;
    let now = Instant::now();
    let _ = interval_at(now + Duration::from_secs(1), Duration::from_secs(1));
}

#[test]
fn timeout_creates() {
    use edgerun_bare_rt::Timeout;
    let _future = async {};
    let _ = timeout(Duration::from_secs(1), _future);
}