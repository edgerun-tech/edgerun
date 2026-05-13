use std::path::PathBuf;

use crate::shell_detect::ShellType;
use crate::shell_detect::detect_shell_type;

pub struct ShellParse {
    has_error: bool,
}

/// Parse the provided shell source using EdgeRun's conservative shell scanner.
pub fn try_parse_shell(shell_lc_arg: &str) -> Option<ShellParse> {
    Some(ShellParse {
        has_error: shell_lc_arg.trim().is_empty(),
    })
}

/// Parse a script which may contain multiple simple commands joined only by
/// the safe logical/pipe/sequencing operators: `&&`, `||`, `;`, `|`.
pub fn try_parse_word_only_commands_sequence(
    parsed: &ShellParse,
    src: &str,
) -> Option<Vec<Vec<String>>> {
    if parsed.has_error {
        return None;
    }
    parse_word_only_commands_sequence(src)
}

pub fn extract_bash_command(command: &[String]) -> Option<(&str, &str)> {
    let [shell, flag, script] = command else {
        return None;
    };
    if !matches!(flag.as_str(), "-lc" | "-c")
        || !matches!(
            detect_shell_type(&PathBuf::from(shell)),
            Some(ShellType::Zsh) | Some(ShellType::Bash) | Some(ShellType::Sh)
        )
    {
        return None;
    }
    Some((shell, script))
}

/// Returns the sequence of plain commands within a `bash -lc "..."` or
/// `zsh -lc "..."` invocation when the script only contains word-only commands
/// joined by safe operators.
pub fn parse_shell_lc_plain_commands(command: &[String]) -> Option<Vec<Vec<String>>> {
    let (_, script) = extract_bash_command(command)?;
    parse_word_only_commands_sequence(script)
}

/// Returns the parsed argv for a single shell command in a here-doc style
/// script (`<<`), as long as the script contains exactly one command.
pub fn parse_shell_lc_single_command_prefix(command: &[String]) -> Option<Vec<String>> {
    let (_, script) = extract_bash_command(command)?;
    parse_single_heredoc_command_prefix(script)
}

fn parse_word_only_commands_sequence(src: &str) -> Option<Vec<Vec<String>>> {
    let mut scanner = ShellScanner::new(src);
    let mut commands = Vec::new();
    let mut expect_command = true;

    loop {
        scanner.skip_ws();
        if scanner.is_eof() {
            return if expect_command { None } else { Some(commands) };
        }
        if !expect_command {
            match scanner.parse_operator()? {
                ShellOperator::CommandSeparator => {
                    expect_command = true;
                    continue;
                }
            }
        }

        let command = scanner.parse_command_words()?;
        if command.is_empty() {
            return None;
        }
        commands.push(command);
        expect_command = false;
    }
}

fn parse_single_heredoc_command_prefix(script: &str) -> Option<Vec<String>> {
    let mut scanner = ShellScanner::new(script);
    scanner.skip_ws();

    if scanner.starts_with_assignment_like_word() {
        return None;
    }

    let command = scanner.parse_command_words_until_heredoc()?;
    if command.is_empty() || scanner.saw_unsafe_construct {
        return None;
    }

    scanner.skip_ws();
    if scanner.consume(">>") || scanner.consume(">") {
        return None;
    }
    if !scanner.consume("<<") {
        return None;
    }
    if scanner.consume("<") {
        // herestring (`<<<`) is not treated as a safe heredoc prefix.
        return None;
    }
    scanner.skip_ws();
    let delimiter = scanner.parse_heredoc_delimiter()?;
    scanner.skip_until_line_end();
    if scanner.saw_unsafe_construct {
        return None;
    }
    scanner.consume_newline();

    if !scanner.skip_heredoc_body(&delimiter) {
        return None;
    }
    scanner.skip_ws();
    if !scanner.is_eof() {
        return None;
    }

    Some(command)
}

#[derive(Clone, Copy)]
enum ShellOperator {
    CommandSeparator,
}

struct ShellScanner<'a> {
    src: &'a str,
    pos: usize,
    saw_unsafe_construct: bool,
}

impl<'a> ShellScanner<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            saw_unsafe_construct: false,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.bump();
        }
    }

    fn consume(&mut self, expected: &str) -> bool {
        if self.rest().starts_with(expected) {
            self.pos += expected.len();
            true
        } else {
            false
        }
    }

    fn parse_operator(&mut self) -> Option<ShellOperator> {
        self.skip_ws();
        for op in ["&&", "||", ";", "|"] {
            if self.consume(op) {
                self.skip_ws();
                if self.is_eof() || self.rest().starts_with(';') || self.rest().starts_with('|') {
                    return None;
                }
                return Some(ShellOperator::CommandSeparator);
            }
        }
        None
    }

    fn parse_command_words(&mut self) -> Option<Vec<String>> {
        if self.starts_with_assignment_like_word() {
            return None;
        }

        let mut words = Vec::new();
        loop {
            self.skip_ws();
            if self.is_eof() || self.starts_operator() {
                break;
            }
            if self.starts_redirect_or_unsafe() {
                return None;
            }
            words.push(self.parse_shell_word()?);
        }
        Some(words)
    }

    fn parse_command_words_until_heredoc(&mut self) -> Option<Vec<String>> {
        let mut words = Vec::new();
        loop {
            self.skip_ws();
            if self.is_eof() || self.rest().starts_with("<<") {
                break;
            }
            if self.starts_operator() || self.starts_redirect_or_unsafe() {
                return None;
            }
            words.push(self.parse_shell_word()?);
        }
        Some(words)
    }

    fn parse_shell_word(&mut self) -> Option<String> {
        let mut word = String::new();
        let start = self.pos;

        while let Some(ch) = self.peek() {
            if ch.is_whitespace()
                || self.starts_operator()
                || self.rest().starts_with("<<")
                || matches!(ch, '>' | '<' | '(' | ')' | '{' | '}' | '`')
            {
                break;
            }

            match ch {
                '\'' => word.push_str(&self.parse_single_quoted()?),
                '"' => word.push_str(&self.parse_double_quoted()?),
                '$' | '\\' => return None,
                _ => {
                    word.push(ch);
                    self.bump();
                }
            }
        }

        if self.pos == start || word.is_empty() {
            None
        } else if words_looks_like_assignment(&word) {
            None
        } else {
            Some(word)
        }
    }

    fn parse_single_quoted(&mut self) -> Option<String> {
        if !self.consume("'") {
            return None;
        }
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '\'' {
                let content = self.src[start..self.pos].to_string();
                self.bump();
                return Some(content);
            }
            self.bump();
        }
        None
    }

    fn parse_double_quoted(&mut self) -> Option<String> {
        if !self.consume("\"") {
            return None;
        }
        let start = self.pos;
        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    let content = self.src[start..self.pos].to_string();
                    self.bump();
                    return Some(content);
                }
                '$' | '`' | '\\' => return None,
                _ => {
                    self.bump();
                }
            }
        }
        None
    }

    fn parse_heredoc_delimiter(&mut self) -> Option<String> {
        if matches!(self.peek(), Some('\'' | '"')) {
            let quote = self.bump()?;
            let start = self.pos;
            while let Some(ch) = self.peek() {
                if ch == quote {
                    let delimiter = self.src[start..self.pos].to_string();
                    self.bump();
                    return if delimiter.is_empty() {
                        None
                    } else {
                        Some(delimiter)
                    };
                }
                self.bump();
            }
            return None;
        }

        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                break;
            }
            if matches!(
                ch,
                '$' | '`' | '\\' | '>' | '<' | '(' | ')' | ';' | '&' | '|'
            ) {
                return None;
            }
            self.bump();
        }
        if self.pos == start {
            None
        } else {
            Some(self.src[start..self.pos].to_string())
        }
    }

    fn skip_until_line_end(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }
            if !ch.is_whitespace() {
                self.saw_unsafe_construct = true;
            }
            self.bump();
        }
    }

    fn consume_newline(&mut self) {
        if self.consume("\r\n") {
            return;
        }
        let _ = self.consume("\n") || self.consume("\r");
    }

    fn skip_heredoc_body(&mut self, delimiter: &str) -> bool {
        loop {
            if self.is_eof() {
                return false;
            }
            let line_start = self.pos;
            self.skip_until_line_end();
            let line = &self.src[line_start..self.pos];
            self.consume_newline();
            if line == delimiter {
                return true;
            }
        }
    }

    fn starts_operator(&self) -> bool {
        ["&&", "||", ";", "|"]
            .iter()
            .any(|op| self.rest().starts_with(op))
    }

    fn starts_redirect_or_unsafe(&self) -> bool {
        matches!(
            self.peek(),
            Some('>' | '<' | '(' | ')' | '{' | '}' | '`' | '&' | '$')
        )
    }

    fn starts_with_assignment_like_word(&self) -> bool {
        let rest = self.rest().trim_start();
        let first = rest
            .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '&' | '|'))
            .next()
            .unwrap_or_default();
        words_looks_like_assignment(first)
    }
}

fn words_looks_like_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
        && !name.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_seq(src: &str) -> Option<Vec<Vec<String>>> {
        let tree = try_parse_shell(src)?;
        try_parse_word_only_commands_sequence(&tree, src)
    }

    #[test]
    fn accepts_single_simple_command() {
        let cmds = parse_seq("ls -1").unwrap();
        assert_eq!(cmds, vec![vec!["ls".to_string(), "-1".to_string()]]);
    }

    #[test]
    fn accepts_multiple_commands_with_allowed_operators() {
        let src = "ls && pwd; echo 'hi there' | wc -l";
        let cmds = parse_seq(src).unwrap();
        let expected: Vec<Vec<String>> = vec![
            vec!["ls".to_string()],
            vec!["pwd".to_string()],
            vec!["echo".to_string(), "hi there".to_string()],
            vec!["wc".to_string(), "-l".to_string()],
        ];
        assert_eq!(cmds, expected);
    }

    #[test]
    fn extracts_double_and_single_quoted_strings() {
        let cmds = parse_seq("echo \"hello world\"").unwrap();
        assert_eq!(
            cmds,
            vec![vec!["echo".to_string(), "hello world".to_string()]]
        );

        let cmds2 = parse_seq("echo 'hi there'").unwrap();
        assert_eq!(
            cmds2,
            vec![vec!["echo".to_string(), "hi there".to_string()]]
        );
    }

    #[test]
    fn accepts_double_quoted_strings_with_newlines() {
        let cmds = parse_seq("git commit -m \"line1\nline2\"").unwrap();
        assert_eq!(
            cmds,
            vec![vec![
                "git".to_string(),
                "commit".to_string(),
                "-m".to_string(),
                "line1\nline2".to_string(),
            ]]
        );
    }

    #[test]
    fn accepts_mixed_quote_concatenation() {
        assert_eq!(
            parse_seq(r#"echo "/usr"'/'"local"/bin"#).unwrap(),
            vec![vec!["echo".to_string(), "/usr/local/bin".to_string()]]
        );
        assert_eq!(
            parse_seq(r#"echo '/usr'"/"'local'/bin"#).unwrap(),
            vec![vec!["echo".to_string(), "/usr/local/bin".to_string()]]
        );
    }

    #[test]
    fn rejects_double_quoted_strings_with_expansions() {
        assert!(parse_seq(r#"echo "hi ${USER}""#).is_none());
        assert!(parse_seq(r#"echo "$HOME""#).is_none());
    }

    #[test]
    fn accepts_numbers_as_words() {
        let cmds = parse_seq("echo 123 456").unwrap();
        assert_eq!(
            cmds,
            vec![vec![
                "echo".to_string(),
                "123".to_string(),
                "456".to_string()
            ]]
        );
    }

    #[test]
    fn rejects_parentheses_and_subshells() {
        assert!(parse_seq("(ls)").is_none());
        assert!(parse_seq("ls || (pwd && echo hi)").is_none());
    }

    #[test]
    fn rejects_redirections_and_unsupported_operators() {
        assert!(parse_seq("ls > out.txt").is_none());
        assert!(parse_seq("echo hi & echo bye").is_none());
    }

    #[test]
    fn rejects_command_and_process_substitutions_and_expansions() {
        assert!(parse_seq("echo $(pwd)").is_none());
        assert!(parse_seq("echo `pwd`").is_none());
        assert!(parse_seq("echo $HOME").is_none());
        assert!(parse_seq("echo \"hi $USER\"").is_none());
    }

    #[test]
    fn rejects_variable_assignment_prefix() {
        assert!(parse_seq("FOO=bar ls").is_none());
    }

    #[test]
    fn rejects_trailing_operator_parse_error() {
        assert!(parse_seq("ls &&").is_none());
    }

    #[test]
    fn rejects_empty_command_position_with_leading_operator() {
        assert!(parse_seq("&& ls").is_none());
    }

    #[test]
    fn rejects_empty_command_position_with_double_separator() {
        assert!(parse_seq("ls ;; pwd").is_none());
    }

    #[test]
    fn rejects_empty_command_position_with_empty_pipeline_segment() {
        assert!(parse_seq("ls | | wc").is_none());
    }

    #[test]
    fn parse_zsh_lc_plain_commands() {
        let command = vec!["zsh".to_string(), "-lc".to_string(), "ls".to_string()];
        let parsed = parse_shell_lc_plain_commands(&command).unwrap();
        assert_eq!(parsed, vec![vec!["ls".to_string()]]);
    }

    #[test]
    fn accepts_concatenated_flag_and_value() {
        let cmds = parse_seq("rg -n \"foo\" -g\"*.py\"").unwrap();
        assert_eq!(
            cmds,
            vec![vec![
                "rg".to_string(),
                "-n".to_string(),
                "foo".to_string(),
                "-g*.py".to_string(),
            ]]
        );
    }

    #[test]
    fn accepts_concatenated_flag_with_single_quotes() {
        let cmds = parse_seq("grep -n 'pattern' -g'*.txt'").unwrap();
        assert_eq!(
            cmds,
            vec![vec![
                "grep".to_string(),
                "-n".to_string(),
                "pattern".to_string(),
                "-g*.txt".to_string(),
            ]]
        );
    }

    #[test]
    fn rejects_concatenation_with_variable_substitution() {
        assert!(parse_seq("rg -g\"$VAR\" pattern").is_none());
        assert!(parse_seq("rg -g\"${VAR}\" pattern").is_none());
    }

    #[test]
    fn rejects_concatenation_with_command_substitution() {
        assert!(parse_seq("rg -g\"$(pwd)\" pattern").is_none());
        assert!(parse_seq("rg -g\"$(echo '*.py')\" pattern").is_none());
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_supports_heredoc() {
        let command = vec![
            "zsh".to_string(),
            "-lc".to_string(),
            "python3 <<'PY'\nprint('hello')\nPY".to_string(),
        ];
        let parsed = parse_shell_lc_single_command_prefix(&command);
        assert_eq!(parsed, Some(vec!["python3".to_string()]));

        let command_unquoted = vec![
            "zsh".to_string(),
            "-lc".to_string(),
            "python3 << PY\nprint('hello')\nPY".to_string(),
        ];
        let parsed_unquoted = parse_shell_lc_single_command_prefix(&command_unquoted);
        assert_eq!(parsed_unquoted, Some(vec!["python3".to_string()]));
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_multi_command_scripts() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "python3 <<'PY'\nprint('hello')\nPY\necho done".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_non_heredoc_redirects() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "echo hello > /tmp/out.txt".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_heredoc_with_extra_file_redirect() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "python3 <<'PY' > /tmp/out.txt\nprint('hello')\nPY".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_heredoc_with_variable_assignment() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "PATH=/tmp/evil:$PATH cat <<'EOF'\nhello\nEOF".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_herestring_with_chaining() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            r#"echo hello > /tmp/out.txt && cat /tmp/out.txt"#.to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_herestring_with_substitution() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            r#"python3 <<< "$(rm -rf /)""#.to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_arithmetic_shift_non_heredoc_script() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "echo $((1<<2))".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }

    #[test]
    fn parse_shell_lc_single_command_prefix_rejects_heredoc_command_with_word_expansion() {
        let command = vec![
            "bash".to_string(),
            "-lc".to_string(),
            "python3 $((1<<2)) <<'PY'\nprint('hello')\nPY".to_string(),
        ];
        assert_eq!(parse_shell_lc_single_command_prefix(&command), None);
    }
}
