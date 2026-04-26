use edgerun_bare_rt::TokenBucket;

#[test]
fn token_bucket_new() {
    let bucket = TokenBucket::new(10, 10);
    let _ = bucket;
}