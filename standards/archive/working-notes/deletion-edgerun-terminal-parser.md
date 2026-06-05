# edgerun-terminal-parser deletion

Status: deleted on 2026-06-05.

`crates/utility/edgerun-terminal-parser` was a no-std Rust facade over a small
terminal control-sequence state machine. The portable value has been extracted
to `terminal-control-scan.wat` (`standard_id = 300112`).

WAT-owned behavior:

- printable byte-span records;
- C0/DEL execute records;
- ESC sequence classification with intermediate span;
- CSI sequence classification with parameter/intermediate span and final byte;
- OSC payload span classification with BEL and ST terminator distinction;
- DCS hook, payload, and ST end records;
- incomplete ESC/CSI/OSC/DCS records;
- bounded output-cap failure.

Deleted Rust value:

- `Perform` callback trait;
- `Parser` allocation/state facade;
- `Params` nested vector API;
- Rust enum/state convenience;
- renderer-facing terminal integration assumptions.

Remaining `edgerun_terminal_parser::*` imports in `edgerun-term-core` are
intentional caller-demolition fallout. Future terminal hosts should invoke the
WAT scanner and map the returned records into their own renderer state instead
of restoring a shared Rust parser crate.

Verification:

```bash
node standards/runners/terminal-control-scan-smoke.js
```
