use edgerun_bare_rt::CancellationToken;

#[test]
fn cancellation_token_new() {
    let token = CancellationToken::new();
    let _ = token;
}

#[test]
fn cancellation_token_is_cancelled() {
    let token = CancellationToken::new();
    assert!(!token.is_cancelled());
}