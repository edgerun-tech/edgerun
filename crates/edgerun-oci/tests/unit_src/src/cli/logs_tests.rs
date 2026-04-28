use super::*;

#[test]
fn tail_lines_returns_requested_suffix() {
    assert_eq!(tail_lines(b"a\nb\nc\n", 2), b"b\nc\n");
    assert_eq!(tail_lines(b"a\nb\nc", 2), b"b\nc");
    assert_eq!(tail_lines(b"a\nb\nc\n", 10), b"a\nb\nc\n");
    assert_eq!(tail_lines(b"a\nb\nc\n", 0), b"");
    assert_eq!(tail_lines(b"", 2), b"");
}

#[test]
fn parse_logs_rejects_missing_tail_value() {
    let args = vec!["--tail".to_string(), "container".to_string()];
    assert!(parse_logs_args(&args).is_err());
}
