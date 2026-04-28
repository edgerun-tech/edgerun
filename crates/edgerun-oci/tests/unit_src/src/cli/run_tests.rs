use super::*;

#[test]
fn parses_pull_policy() {
    assert_eq!(parse_pull_policy("missing").unwrap(), PullPolicy::Missing);
    assert_eq!(
        parse_pull_policy("if-missing").unwrap(),
        PullPolicy::Missing
    );
    assert_eq!(parse_pull_policy("always").unwrap(), PullPolicy::Always);
    assert_eq!(parse_pull_policy("never").unwrap(), PullPolicy::Never);
    assert!(parse_pull_policy("sometimes").is_err());
}
