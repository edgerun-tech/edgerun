# edgerun-shlex deletion

`crates/utility/edgerun-shlex` is deleted after extracting its useful portable
word-splitting behavior to `shell-word-scan.wat`.

WAT owner:

- `standards/build/wasm/codec-primitives/shell-word-scan.wat`
- standard id `300070`
- runner `standards/runners/shell-word-scan-smoke.js`

Covered behavior:

- ASCII whitespace word separation.
- Single quote literal spans.
- Double quote handling.
- Backslash escaping outside quotes.
- Double-quoted backslash policy for `$`, `` ` ``, `"`, `\`, and LF.
- Empty quoted words.
- Unterminated quote rejection.
- Output-cap failure.

The deleted Rust value was `Vec<String>` allocation, iterator/string API shape,
and shell quoting/join helpers. The Rust crate did not implement comment
handling; `#` remains a literal word byte in the WAT scanner. Existing Codex
shell callers are deliberate caller-demolition fallout and should not cause this
crate to be restored.
