# edgerun-httpdate deletion

`crates/utility/edgerun-httpdate` is deleted.

The useful portable behavior is now owned by
`standards/build/wasm/codec-primitives/http-date.wat`:

- IMF-fixdate parse: `Sun, 06 Nov 1994 08:49:37 GMT`
- obsolete RFC850 parse: `Sunday, 06-Nov-94 08:49:37 GMT`
- obsolete asctime parse: `Sun Nov  6 08:49:37 1994`
- RFC850 two-digit year mapping: `<70 => 2000+`, otherwise `1900+`
- weekday/date validation against Unix-day arithmetic
- Unix seconds conversion for years `1970..=9999`
- canonical 29-byte IMF-fixdate emission

The removed Rust value was `SystemTime`, `Display`, `FromStr`, `std::error`,
and comparison API scaffolding around that behavior. Do not restore the crate to
preserve those API shapes.

Verification:

```bash
node standards/runners/http-date-smoke.js
```

The smoke runner covers the original Rust examples, epoch formatting, the 2016
format example, invalid year, bad separators, wrong weekday/date combination,
and output-short formatting.
