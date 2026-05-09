#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteError;

impl fmt::Display for QuoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("shell argument contains NUL byte")
    }
}

pub fn split(input: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_word = false;

    while let Some(ch) = chars.next() {
        match ch {
            c if c.is_whitespace() => {
                if in_word {
                    words.push(current);
                    current = String::new();
                    in_word = false;
                }
            }
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => current.push(c),
                        None => return None,
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.peek().copied() {
                            Some('$' | '`' | '"' | '\\' | '\n') => {
                                current.push(chars.next().expect("peeked char exists"));
                            }
                            Some(_) => current.push('\\'),
                            None => current.push('\\'),
                        },
                        Some(c) => current.push(c),
                        None => return None,
                    }
                }
            }
            '\\' => {
                in_word = true;
                match chars.next() {
                    Some(c) => current.push(c),
                    None => current.push('\\'),
                }
            }
            c => {
                in_word = true;
                current.push(c);
            }
        }
    }

    if in_word {
        words.push(current);
    }

    Some(words)
}

pub fn try_join<I, S>(words: I) -> Result<String, QuoteError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = String::new();
    for word in words {
        if !out.is_empty() {
            out.push(' ');
        }
        append_quoted(&mut out, word.as_ref())?;
    }
    Ok(out)
}

fn append_quoted(out: &mut String, word: &str) -> Result<(), QuoteError> {
    if word.as_bytes().contains(&0) {
        return Err(QuoteError);
    }
    if word.is_empty() {
        out.push_str("''");
        return Ok(());
    }
    if word.bytes().all(is_safe_unquoted_byte) {
        out.push_str(word);
        return Ok(());
    }

    out.push('\'');
    for ch in word.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    Ok(())
}

fn is_safe_unquoted_byte(byte: u8) -> bool {
    matches!(
        byte,
        b'a'..=b'z'
            | b'A'..=b'Z'
            | b'0'..=b'9'
            | b'_'
            | b'@'
            | b'%'
            | b'+'
            | b'='
            | b':'
            | b','
            | b'.'
            | b'/'
            | b'-'
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn splits_quotes_and_escapes() {
        assert_eq!(
            split(r#"bash -lc 'echo "hello world"'"#),
            Some(vec![
                "bash".into(),
                "-lc".into(),
                r#"echo "hello world""#.into()
            ])
        );
        assert_eq!(
            split(r#"cat "pkg\src\main.rs""#),
            Some(vec!["cat".into(), r#"pkg\src\main.rs"#.into()])
        );
    }

    #[test]
    fn rejects_unclosed_quotes() {
        assert_eq!(split("'unterminated"), None);
        assert_eq!(split("\"unterminated"), None);
    }

    #[test]
    fn joins_shell_words() {
        assert_eq!(
            try_join(["git", "grep", "foo bar"]).unwrap(),
            "git grep 'foo bar'"
        );
        assert_eq!(try_join([""]).unwrap(), "''");
        assert_eq!(try_join(["can't"]).unwrap(), "'can'\\''t'");
    }
}
