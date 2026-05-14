#![no_std]

#[macro_export]
macro_rules! assert_matches {
    ($expression:expr, $pattern:pat $(if $guard:expr)? $(,)?) => {
        match $expression {
            $pattern $(if $guard)? => {}
            ref value => panic!(
                "assertion failed: expression did not match pattern\n  expression: `{}`\n  value: `{:?}`\n  pattern: `{}`",
                stringify!($expression),
                value,
                stringify!($pattern $(if $guard)?),
            ),
        }
    };
}
