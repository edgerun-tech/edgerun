use edgerun_bare_rt::RateLimiter;

#[test]
fn rate_limiter_new() {
    let limiter = RateLimiter::new(10, 1000);
    let _ = limiter;
}