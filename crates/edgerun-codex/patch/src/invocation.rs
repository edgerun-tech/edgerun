use std::collections::HashMap;
use std::path::Path;

use crate::absolute_path::AbsolutePathBuf;
use crate::file_system::ExecutorFileSystem;

use crate::ApplyPatchAction;
use crate::ApplyPatchArgs;
use crate::ApplyPatchError;
use crate::ApplyPatchFileChange;
use crate::ApplyPatchFileUpdate;
use crate::IoError;
use crate::MaybeApplyPatchVerified;
use crate::parser::Hunk;
use crate::parser::ParseError;
use crate::parser::parse_patch;
use crate::unified_diff_from_chunks;

const APPLY_PATCH_COMMANDS: [&str; 2] = ["apply_patch", "applypatch"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApplyPatchShell {
    Unix,
}

#[derive(Debug, PartialEq)]
pub enum MaybeApplyPatch {
    Body(ApplyPatchArgs),
    ShellParseError(ExtractHeredocError),
    PatchParseError(ParseError),
    NotApplyPatch,
}

#[derive(Debug, PartialEq)]
pub enum ExtractHeredocError {
    CommandDidNotStartWithApplyPatch,
    FailedToParsePatchIntoAst,
    FailedToFindHeredocBody,
}

fn classify_shell_name(shell: &str) -> Option<String> {
    std::path::Path::new(shell)
        .file_stem()
        .and_then(|name| name.to_str())
        .map(str::to_ascii_lowercase)
}

fn classify_shell(shell: &str, flag: &str) -> Option<ApplyPatchShell> {
    classify_shell_name(shell).and_then(|name| match name.as_str() {
        "bash" | "zsh" | "sh" if matches!(flag, "-lc" | "-c") => Some(ApplyPatchShell::Unix),
        _ => None,
    })
}

fn can_skip_flag(shell: &str, flag: &str) -> bool {
    let _ = shell;
    let _ = flag;
    false
}

fn parse_shell_script(argv: &[String]) -> Option<(ApplyPatchShell, &str)> {
    match argv {
        [shell, flag, script] => classify_shell(shell, flag).map(|shell_type| {
            let script = script.as_str();
            (shell_type, script)
        }),
        [shell, skip_flag, flag, script] if can_skip_flag(shell, skip_flag) => {
            classify_shell(shell, flag).map(|shell_type| {
                let script = script.as_str();
                (shell_type, script)
            })
        }
        _ => None,
    }
}

fn extract_apply_patch_from_shell(
    shell: ApplyPatchShell,
    script: &str,
) -> std::result::Result<(String, Option<String>), ExtractHeredocError> {
    match shell {
        ApplyPatchShell::Unix => extract_apply_patch_from_bash(script),
    }
}

// TODO: make private once we remove tests in lib.rs
pub fn maybe_parse_apply_patch(argv: &[String]) -> MaybeApplyPatch {
    match argv {
        // Direct invocation: apply_patch <patch>
        [cmd, body] if APPLY_PATCH_COMMANDS.contains(&cmd.as_str()) => match parse_patch(body) {
            Ok(source) => MaybeApplyPatch::Body(source),
            Err(e) => MaybeApplyPatch::PatchParseError(e),
        },
        // Shell heredoc form: (optional `cd <path> &&`) apply_patch <<'EOF' ...
        _ => match parse_shell_script(argv) {
            Some((shell, script)) => match extract_apply_patch_from_shell(shell, script) {
                Ok((body, workdir)) => match parse_patch(&body) {
                    Ok(mut source) => {
                        source.workdir = workdir;
                        MaybeApplyPatch::Body(source)
                    }
                    Err(e) => MaybeApplyPatch::PatchParseError(e),
                },
                Err(ExtractHeredocError::CommandDidNotStartWithApplyPatch) => {
                    MaybeApplyPatch::NotApplyPatch
                }
                Err(e) => MaybeApplyPatch::ShellParseError(e),
            },
            None => MaybeApplyPatch::NotApplyPatch,
        },
    }
}

/// cwd must be an absolute path so that we can resolve relative paths in the
/// patch.
pub async fn maybe_parse_apply_patch_verified(
    argv: &[String],
    cwd: &AbsolutePathBuf,
    fs: &dyn ExecutorFileSystem,
    sandbox: Option<&crate::file_system::FileSystemSandboxContext>,
) -> MaybeApplyPatchVerified {
    // Detect a raw patch body passed directly as the command or as the body of a shell
    // script. In these cases, report an explicit error rather than applying the patch.
    if let [body] = argv
        && parse_patch(body).is_ok()
    {
        return MaybeApplyPatchVerified::CorrectnessError(ApplyPatchError::ImplicitInvocation);
    }
    if let Some((_, script)) = parse_shell_script(argv)
        && parse_patch(script).is_ok()
    {
        return MaybeApplyPatchVerified::CorrectnessError(ApplyPatchError::ImplicitInvocation);
    }

    match maybe_parse_apply_patch(argv) {
        MaybeApplyPatch::Body(ApplyPatchArgs {
            patch,
            hunks,
            workdir,
        }) => {
            let effective_cwd = workdir
                .as_ref()
                .map(|dir| cwd.join(Path::new(dir)))
                .unwrap_or_else(|| cwd.clone());
            let mut changes = HashMap::new();
            for hunk in hunks {
                let path = hunk.resolve_path(&effective_cwd);
                match hunk {
                    Hunk::AddFile { contents, .. } => {
                        changes.insert(
                            path.into_path_buf(),
                            ApplyPatchFileChange::Add { content: contents },
                        );
                    }
                    Hunk::DeleteFile { .. } => {
                        let content = match fs.read_file_text(&path, sandbox).await {
                            Ok(content) => content,
                            Err(e) => {
                                return MaybeApplyPatchVerified::CorrectnessError(
                                    ApplyPatchError::IoError(IoError {
                                        context: format!("Failed to read {}", path.display()),
                                        source: e,
                                    }),
                                );
                            }
                        };
                        changes.insert(
                            path.into_path_buf(),
                            ApplyPatchFileChange::Delete { content },
                        );
                    }
                    Hunk::UpdateFile {
                        move_path, chunks, ..
                    } => {
                        let ApplyPatchFileUpdate {
                            unified_diff,
                            content: contents,
                            ..
                        } = match unified_diff_from_chunks(&path, &chunks, fs, sandbox).await {
                            Ok(diff) => diff,
                            Err(e) => {
                                return MaybeApplyPatchVerified::CorrectnessError(e);
                            }
                        };
                        changes.insert(
                            path.into_path_buf(),
                            ApplyPatchFileChange::Update {
                                unified_diff,
                                move_path: move_path
                                    .as_ref()
                                    .map(|p| effective_cwd.join(p).into_path_buf()),
                                new_content: contents,
                            },
                        );
                    }
                }
            }
            MaybeApplyPatchVerified::Body(ApplyPatchAction {
                changes,
                patch,
                cwd: effective_cwd,
            })
        }
        MaybeApplyPatch::ShellParseError(e) => MaybeApplyPatchVerified::ShellParseError(e),
        MaybeApplyPatch::PatchParseError(e) => MaybeApplyPatchVerified::CorrectnessError(e.into()),
        MaybeApplyPatch::NotApplyPatch => MaybeApplyPatchVerified::NotApplyPatch,
    }
}

/// Extract the heredoc body (and optional `cd` workdir) from a shell script
/// that invokes the apply_patch tool using a heredoc.
///
/// Supported top‑level forms (must be the only top‑level statement):
/// - `apply_patch <<'EOF'\n...\nEOF`
/// - `cd <path> && apply_patch <<'EOF'\n...\nEOF`
///
/// Notes about matching:
/// - Parsed with a strict EdgeRun scanner, anchored to the whole script.
/// - The connector between `cd` and `apply_patch` must be `&&` (not `|` or `||`).
/// - Exactly one positional `word` argument is allowed for `cd` (no flags, no quoted
///   strings, no second argument).
/// - The apply command is validated in‑query via `#any-of?` to allow `apply_patch`
///   or `applypatch`.
/// - Preceding or trailing commands (e.g., `echo ...;` or `... && echo done`) do not match.
///
/// Returns `(heredoc_body, Some(path))` when the `cd` variant matches, or
/// `(heredoc_body, None)` for the direct form. Errors are returned if the script
/// cannot be parsed or does not match the allowed patterns.
fn extract_apply_patch_from_bash(
    src: &str,
) -> std::result::Result<(String, Option<String>), ExtractHeredocError> {
    parse_apply_patch_heredoc(src).ok_or(ExtractHeredocError::CommandDidNotStartWithApplyPatch)
}

fn parse_apply_patch_heredoc(src: &str) -> Option<(String, Option<String>)> {
    let mut scanner = ShellScanner::new(src);
    scanner.skip_ws();

    let workdir = if scanner.consume_word("cd") {
        scanner.skip_ws();
        let path = scanner.parse_word()?;
        scanner.skip_ws();
        if scanner.parse_word().is_some() {
            return None;
        }
        if !scanner.consume("&&") {
            return None;
        }
        scanner.skip_ws();
        Some(path)
    } else {
        None
    };

    let command = scanner.parse_word()?;
    if !APPLY_PATCH_COMMANDS.contains(&command.as_str()) {
        return None;
    }
    scanner.skip_ws();
    if scanner.parse_word().is_some() {
        return None;
    }
    if !scanner.consume("<<") || scanner.consume("<") {
        return None;
    }
    scanner.skip_ws();
    let delimiter = scanner.parse_heredoc_delimiter()?;
    scanner.skip_rest_of_line()?;
    let body = scanner.take_heredoc_body(&delimiter)?;
    scanner.skip_ws();
    if scanner.is_eof() {
        Some((body, workdir))
    } else {
        None
    }
}

struct ShellScanner<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> ShellScanner<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, pos: 0 }
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

    fn consume_word(&mut self, expected: &str) -> bool {
        let start = self.pos;
        let Some(word) = self.parse_word() else {
            return false;
        };
        if word == expected {
            true
        } else {
            self.pos = start;
            false
        }
    }

    fn parse_word(&mut self) -> Option<String> {
        self.skip_ws();
        match self.peek()? {
            '\'' => self.parse_quoted('\''),
            '"' => self.parse_quoted('"'),
            ch if is_shell_word_char(ch) => {
                let start = self.pos;
                while matches!(self.peek(), Some(ch) if is_shell_word_char(ch)) {
                    self.bump();
                }
                Some(self.src[start..self.pos].to_string())
            }
            _ => None,
        }
    }

    fn parse_quoted(&mut self, quote: char) -> Option<String> {
        if self.bump()? != quote {
            return None;
        }
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == quote {
                let out = self.src[start..self.pos].to_string();
                self.bump();
                return Some(out);
            }
            if matches!(ch, '$' | '`' | '\\') {
                return None;
            }
            self.bump();
        }
        None
    }

    fn parse_heredoc_delimiter(&mut self) -> Option<String> {
        self.parse_word()
    }

    fn skip_rest_of_line(&mut self) -> Option<()> {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                self.bump();
                return Some(());
            }
            if ch == '\r' {
                self.bump();
                let _ = self.consume("\n");
                return Some(());
            }
            if !ch.is_whitespace() {
                return None;
            }
            self.bump();
        }
        Some(())
    }

    fn take_heredoc_body(&mut self, delimiter: &str) -> Option<String> {
        let body_start = self.pos;
        loop {
            if self.is_eof() {
                return None;
            }
            let line_start = self.pos;
            while let Some(ch) = self.peek() {
                if ch == '\n' || ch == '\r' {
                    break;
                }
                self.bump();
            }
            let line = &self.src[line_start..self.pos];
            let line_end = self.pos;
            self.skip_rest_of_line()?;
            if line == delimiter {
                let body_end = if line_start > body_start
                    && self.src.as_bytes().get(line_start - 1) == Some(&b'\n')
                {
                    line_start - 1
                } else {
                    line_end
                };
                return Some(self.src[body_start..body_end].to_string());
            }
        }
    }
}

fn is_shell_word_char(ch: char) -> bool {
    !ch.is_whitespace()
        && !matches!(
            ch,
            '\'' | '"' | '$' | '`' | '\\' | '&' | '|' | ';' | '<' | '>'
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system::LOCAL_FS;
    use crate::unified_diff_from_chunks;
    use assert_matches::assert_matches;
    use std::fs;
    use std::path::PathBuf;
    use std::string::ToString;
    use tempfile::tempdir;

    /// Helper to construct a patch with the given body.
    fn wrap_patch(body: &str) -> String {
        format!("*** Begin Patch\n{body}\n*** End Patch")
    }

    fn strs_to_strings(strs: &[&str]) -> Vec<String> {
        strs.iter().map(ToString::to_string).collect()
    }

    // Test helpers to reduce repetition when building bash -lc heredoc scripts
    fn args_bash(script: &str) -> Vec<String> {
        strs_to_strings(&["bash", "-lc", script])
    }

    fn heredoc_script(prefix: &str) -> String {
        format!(
            "{prefix}apply_patch <<'PATCH'\n*** Begin Patch\n*** Add File: foo\n+hi\n*** End Patch\nPATCH"
        )
    }

    fn expected_single_add() -> Vec<Hunk> {
        vec![Hunk::AddFile {
            path: PathBuf::from("foo"),
            contents: "hi\n".to_string(),
        }]
    }

    fn assert_match_args(args: Vec<String>, expected_workdir: Option<&str>) {
        match maybe_parse_apply_patch(&args) {
            MaybeApplyPatch::Body(ApplyPatchArgs { hunks, workdir, .. }) => {
                assert_eq!(workdir.as_deref(), expected_workdir);
                assert_eq!(hunks, expected_single_add());
            }
            result => panic!("expected MaybeApplyPatch::Body got {result:?}"),
        }
    }

    fn assert_match(script: &str, expected_workdir: Option<&str>) {
        let args = args_bash(script);
        assert_match_args(args, expected_workdir);
    }

    fn assert_not_match(script: &str) {
        let args = args_bash(script);
        assert_matches!(
            maybe_parse_apply_patch(&args),
            MaybeApplyPatch::NotApplyPatch
        );
    }

    #[edgerun_tokio::test]
    async fn test_implicit_patch_single_arg_is_error() {
        let patch = "*** Begin Patch\n*** Add File: foo\n+hi\n*** End Patch".to_string();
        let args = vec![patch];
        let dir = tempdir().unwrap();
        assert_matches!(
            maybe_parse_apply_patch_verified(
                &args,
                &AbsolutePathBuf::from_absolute_path(dir.path()).unwrap(),
                LOCAL_FS.as_ref(),
                /*sandbox*/ None,
            )
            .await,
            MaybeApplyPatchVerified::CorrectnessError(ApplyPatchError::ImplicitInvocation)
        );
    }

    #[edgerun_tokio::test]
    async fn test_implicit_patch_bash_script_is_error() {
        let script = "*** Begin Patch\n*** Add File: foo\n+hi\n*** End Patch";
        let args = args_bash(script);
        let dir = tempdir().unwrap();
        assert_matches!(
            maybe_parse_apply_patch_verified(
                &args,
                &AbsolutePathBuf::from_absolute_path(dir.path()).unwrap(),
                LOCAL_FS.as_ref(),
                /*sandbox*/ None,
            )
            .await,
            MaybeApplyPatchVerified::CorrectnessError(ApplyPatchError::ImplicitInvocation)
        );
    }

    #[edgerun_tokio::test]
    async fn test_literal() {
        let args = strs_to_strings(&[
            "apply_patch",
            r#"*** Begin Patch
*** Add File: foo
+hi
*** End Patch
"#,
        ]);

        match maybe_parse_apply_patch(&args) {
            MaybeApplyPatch::Body(ApplyPatchArgs { hunks, .. }) => {
                assert_eq!(
                    hunks,
                    vec![Hunk::AddFile {
                        path: PathBuf::from("foo"),
                        contents: "hi\n".to_string()
                    }]
                );
            }
            result => panic!("expected MaybeApplyPatch::Body got {result:?}"),
        }
    }

    #[edgerun_tokio::test]
    async fn test_literal_applypatch() {
        let args = strs_to_strings(&[
            "applypatch",
            r#"*** Begin Patch
*** Add File: foo
+hi
*** End Patch
"#,
        ]);

        match maybe_parse_apply_patch(&args) {
            MaybeApplyPatch::Body(ApplyPatchArgs { hunks, .. }) => {
                assert_eq!(
                    hunks,
                    vec![Hunk::AddFile {
                        path: PathBuf::from("foo"),
                        contents: "hi\n".to_string()
                    }]
                );
            }
            result => panic!("expected MaybeApplyPatch::Body got {result:?}"),
        }
    }

    #[edgerun_tokio::test]
    async fn test_heredoc() {
        assert_match(&heredoc_script(""), /*expected_workdir*/ None);
    }

    #[edgerun_tokio::test]
    async fn test_heredoc_non_login_shell() {
        let script = heredoc_script("");
        let args = strs_to_strings(&["bash", "-c", &script]);
        assert_match_args(args, /*expected_workdir*/ None);
    }

    #[edgerun_tokio::test]
    async fn test_heredoc_applypatch() {
        let args = strs_to_strings(&[
            "bash",
            "-lc",
            r#"applypatch <<'PATCH'
*** Begin Patch
*** Add File: foo
+hi
*** End Patch
PATCH"#,
        ]);

        match maybe_parse_apply_patch(&args) {
            MaybeApplyPatch::Body(ApplyPatchArgs { hunks, workdir, .. }) => {
                assert_eq!(workdir, None);
                assert_eq!(
                    hunks,
                    vec![Hunk::AddFile {
                        path: PathBuf::from("foo"),
                        contents: "hi\n".to_string()
                    }]
                );
            }
            result => panic!("expected MaybeApplyPatch::Body got {result:?}"),
        }
    }

    #[edgerun_tokio::test]
    async fn test_heredoc_with_leading_cd() {
        assert_match(&heredoc_script("cd foo && "), Some("foo"));
    }

    #[edgerun_tokio::test]
    async fn test_cd_with_semicolon_is_ignored() {
        assert_not_match(&heredoc_script("cd foo; "));
    }

    #[edgerun_tokio::test]
    async fn test_cd_or_apply_patch_is_ignored() {
        assert_not_match(&heredoc_script("cd bar || "));
    }

    #[edgerun_tokio::test]
    async fn test_cd_pipe_apply_patch_is_ignored() {
        assert_not_match(&heredoc_script("cd bar | "));
    }

    #[edgerun_tokio::test]
    async fn test_cd_single_quoted_path_with_spaces() {
        assert_match(&heredoc_script("cd 'foo bar' && "), Some("foo bar"));
    }

    #[edgerun_tokio::test]
    async fn test_cd_double_quoted_path_with_spaces() {
        assert_match(&heredoc_script("cd \"foo bar\" && "), Some("foo bar"));
    }

    #[edgerun_tokio::test]
    async fn test_echo_and_apply_patch_is_ignored() {
        assert_not_match(&heredoc_script("echo foo && "));
    }

    #[edgerun_tokio::test]
    async fn test_apply_patch_with_arg_is_ignored() {
        let script = "apply_patch foo <<'PATCH'\n*** Begin Patch\n*** Add File: foo\n+hi\n*** End Patch\nPATCH";
        assert_not_match(script);
    }

    #[edgerun_tokio::test]
    async fn test_double_cd_then_apply_patch_is_ignored() {
        assert_not_match(&heredoc_script("cd foo && cd bar && "));
    }

    #[edgerun_tokio::test]
    async fn test_cd_two_args_is_ignored() {
        assert_not_match(&heredoc_script("cd foo bar && "));
    }

    #[edgerun_tokio::test]
    async fn test_cd_then_apply_patch_then_extra_is_ignored() {
        let script = format!("{} && echo done", heredoc_script("cd bar && "));
        assert_not_match(&script);
    }

    #[edgerun_tokio::test]
    async fn test_echo_then_cd_and_apply_patch_is_ignored() {
        // Ensure preceding commands before the `cd && apply_patch <<...` sequence do not match.
        assert_not_match(&heredoc_script("echo foo; cd bar && "));
    }

    #[edgerun_tokio::test]
    async fn test_unified_diff_last_line_replacement() {
        // Replace the very last line of the file.
        let dir = tempdir().unwrap();
        let path = dir.path().join("last.txt");
        fs::write(&path, "foo\nbar\nbaz\n").unwrap();

        let patch = wrap_patch(&format!(
            r#"*** Update File: {}
@@
 foo
 bar
-baz
+BAZ
"#,
            path.display()
        ));

        let patch = parse_patch(&patch).unwrap();
        let chunks = match patch.hunks.as_slice() {
            [Hunk::UpdateFile { chunks, .. }] => chunks,
            _ => panic!("Expected a single UpdateFile hunk"),
        };

        let path_abs = AbsolutePathBuf::from_absolute_path(&path).unwrap();
        let diff =
            unified_diff_from_chunks(&path_abs, chunks, LOCAL_FS.as_ref(), /*sandbox*/ None)
                .await
                .unwrap();
        let expected_diff = r#"@@ -2,2 +2,2 @@
 bar
-baz
+BAZ
"#;
        let expected = ApplyPatchFileUpdate {
            unified_diff: expected_diff.to_string(),
            original_content: "foo\nbar\nbaz\n".to_string(),
            content: "foo\nbar\nBAZ\n".to_string(),
        };
        assert_eq!(expected, diff);
    }

    #[edgerun_tokio::test]
    async fn test_unified_diff_insert_at_eof() {
        // Insert a new line at end‑of‑file.
        let dir = tempdir().unwrap();
        let path = dir.path().join("insert.txt");
        fs::write(&path, "foo\nbar\nbaz\n").unwrap();

        let patch = wrap_patch(&format!(
            r#"*** Update File: {}
@@
+quux
*** End of File
"#,
            path.display()
        ));

        let patch = parse_patch(&patch).unwrap();
        let chunks = match patch.hunks.as_slice() {
            [Hunk::UpdateFile { chunks, .. }] => chunks,
            _ => panic!("Expected a single UpdateFile hunk"),
        };

        let path_abs = AbsolutePathBuf::from_absolute_path(&path).unwrap();
        let diff =
            unified_diff_from_chunks(&path_abs, chunks, LOCAL_FS.as_ref(), /*sandbox*/ None)
                .await
                .unwrap();
        let expected_diff = r#"@@ -3 +3,2 @@
 baz
+quux
"#;
        let expected = ApplyPatchFileUpdate {
            unified_diff: expected_diff.to_string(),
            original_content: "foo\nbar\nbaz\n".to_string(),
            content: "foo\nbar\nbaz\nquux\n".to_string(),
        };
        assert_eq!(expected, diff);
    }

    #[edgerun_tokio::test]
    async fn test_apply_patch_should_resolve_absolute_paths_in_cwd() {
        let session_dir = tempdir().unwrap();
        let relative_path = "source.txt";

        // Note that we need this file to exist for the patch to be "verified"
        // and parsed correctly.
        let session_file_path = session_dir.path().join(relative_path);
        fs::write(&session_file_path, "session directory content\n").unwrap();

        let argv = vec![
            "apply_patch".to_string(),
            r#"*** Begin Patch
*** Update File: source.txt
@@
-session directory content
+updated session directory content
*** End Patch"#
                .to_string(),
        ];

        let result = maybe_parse_apply_patch_verified(
            &argv,
            &AbsolutePathBuf::from_absolute_path(session_dir.path()).unwrap(),
            LOCAL_FS.as_ref(),
            /*sandbox*/ None,
        )
        .await;

        // Verify the patch contents - as otherwise we may have pulled contents
        // from the wrong file (as we're using relative paths)
        assert_eq!(
            result,
            MaybeApplyPatchVerified::Body(ApplyPatchAction {
                changes: HashMap::from([(
                    session_dir.path().join(relative_path),
                    ApplyPatchFileChange::Update {
                        unified_diff: r#"@@ -1 +1 @@
-session directory content
+updated session directory content
"#
                        .to_string(),
                        move_path: None,
                        new_content: "updated session directory content\n".to_string(),
                    },
                )]),
                patch: argv[1].clone(),
                cwd: AbsolutePathBuf::from_absolute_path(session_dir.path()).unwrap(),
            })
        );
    }

    #[edgerun_tokio::test]
    async fn test_apply_patch_resolves_move_path_with_effective_cwd() {
        let session_dir = tempdir().unwrap();
        let worktree_rel = "alt";
        let worktree_dir = session_dir.path().join(worktree_rel);
        fs::create_dir_all(&worktree_dir).unwrap();

        let source_name = "old.txt";
        let dest_name = "renamed.txt";
        let source_path = worktree_dir.join(source_name);
        fs::write(&source_path, "before\n").unwrap();

        let patch = wrap_patch(&format!(
            r#"*** Update File: {source_name}
*** Move to: {dest_name}
@@
-before
+after"#
        ));

        let shell_script = format!("cd {worktree_rel} && apply_patch <<'PATCH'\n{patch}\nPATCH");
        let argv = vec!["bash".into(), "-lc".into(), shell_script];

        let result = maybe_parse_apply_patch_verified(
            &argv,
            &AbsolutePathBuf::from_absolute_path(session_dir.path()).unwrap(),
            LOCAL_FS.as_ref(),
            /*sandbox*/ None,
        )
        .await;
        let action = match result {
            MaybeApplyPatchVerified::Body(action) => action,
            other => panic!("expected verified body, got {other:?}"),
        };

        assert_eq!(action.cwd.as_path(), worktree_dir.as_path());

        let source_path = worktree_dir.join(source_name);
        let change = action
            .changes()
            .get(source_path.as_path())
            .expect("source file change present");

        match change {
            ApplyPatchFileChange::Update { move_path, .. } => {
                assert_eq!(
                    move_path.as_deref(),
                    Some(worktree_dir.join(dest_name).as_path())
                );
            }
            other => panic!("expected update change, got {other:?}"),
        }
    }

    #[edgerun_tokio::test]
    async fn test_unreadable_destinations_still_verify() {
        let session_dir = tempdir().unwrap();
        fs::write(session_dir.path().join("binary.dat"), [0xff, 0xfe, 0xfd]).unwrap();
        let cwd = AbsolutePathBuf::from_absolute_path(session_dir.path()).unwrap();
        let add_argv = vec![
            "apply_patch".to_string(),
            "*** Begin Patch\n*** Add File: binary.dat\n+text\n*** End Patch".to_string(),
        ];
        fs::write(session_dir.path().join("source.txt"), "before\n").unwrap();
        let move_argv = vec![
            "apply_patch".to_string(),
            "*** Begin Patch\n*** Update File: source.txt\n*** Move to: binary.dat\n@@\n-before\n+after\n*** End Patch".to_string(),
        ];

        for argv in [add_argv, move_argv] {
            let result = maybe_parse_apply_patch_verified(
                &argv,
                &cwd,
                LOCAL_FS.as_ref(),
                /*sandbox*/ None,
            )
            .await;

            assert!(matches!(result, MaybeApplyPatchVerified::Body(_)));
        }
    }

    #[cfg(unix)]
    #[edgerun_tokio::test]
    async fn test_delete_symlink_still_verifies() {
        use std::os::unix::fs::symlink;

        let session_dir = tempdir().unwrap();
        fs::write(session_dir.path().join("target.txt"), "target\n").unwrap();
        symlink(
            session_dir.path().join("target.txt"),
            session_dir.path().join("link.txt"),
        )
        .unwrap();
        let argv = vec![
            "apply_patch".to_string(),
            "*** Begin Patch\n*** Delete File: link.txt\n*** End Patch".to_string(),
        ];

        let result = maybe_parse_apply_patch_verified(
            &argv,
            &AbsolutePathBuf::from_absolute_path(session_dir.path()).unwrap(),
            LOCAL_FS.as_ref(),
            /*sandbox*/ None,
        )
        .await;

        assert!(matches!(result, MaybeApplyPatchVerified::Body(_)));
    }
}
