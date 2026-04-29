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

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn parses_run_options_with_command_tail() {
    let (opts, image, cmd) = parse_run_args(&args(&[
        "--rm",
        "-it",
        "-e",
        "A=1",
        "--env=B=2",
        "--dns=1.1.1.1",
        "--add-host",
        "db:10.0.0.2",
        "--pull=never",
        "alpine:latest",
        "sh",
        "-lc",
        "echo ok",
    ]))
    .unwrap();

    assert!(opts.rm);
    assert!(opts.interactive);
    assert!(opts.tty);
    assert_eq!(opts.env, vec!["A=1".to_string(), "B=2".to_string()]);
    assert_eq!(opts.dns, vec!["1.1.1.1".to_string()]);
    assert_eq!(
        opts.add_hosts,
        vec![("db".to_string(), "10.0.0.2".to_string())]
    );
    assert_eq!(opts.pull_policy, PullPolicy::Never);
    assert_eq!(image, "alpine:latest");
    assert_eq!(cmd, args(&["sh", "-lc", "echo ok"]));
}

#[test]
fn run_stops_parsing_after_image() {
    let (opts, image, cmd) = parse_run_args(&args(&[
        "--name",
        "demo",
        "alpine:latest",
        "--not-a-run-flag",
        "value",
    ]))
    .unwrap();

    assert_eq!(opts.name.as_deref(), Some("demo"));
    assert_eq!(image, "alpine:latest");
    assert_eq!(cmd, args(&["--not-a-run-flag", "value"]));
}
