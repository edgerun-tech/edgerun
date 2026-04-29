use super::*;

#[test]
fn extract_first_secret_parses_bytes() {
    let secret_bytes: Vec<Val> = b"user:pass".iter().map(|&b| Val::Y(b)).collect();
    let secret_val = Val::Str(vec![
        Val::O("/session/s1".into()),
        Val::Dict(vec![]),
        Val::Arr(secret_bytes),
        Val::S("text/plain".into()),
    ]);

    let reply = Msg::ret(1, "test").body(
        vec![Val::Dict(vec![(
            Val::O("/org/freedesktop/secrets/collections/registry/abc".into()),
            Val::Var(Box::new(secret_val)),
        )])],
        "a{o(v)}",
    );

    let secret = extract_first_secret(&reply).unwrap();
    assert_eq!(secret, b"user:pass");
}
