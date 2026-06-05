# edgerun-url deletion map

`edgerun-url` is a Rust API shell around a small absolute URL subset. The useful
portable behavior is now owned by `url-scan.wat`.

## Extracted behavior

- `standards/build/wasm/codec-primitives/url-scan.wat`
- `standards/runners/url-scan-smoke.js`
- `standards/components/manifests/codec-primitives/url-scan.toml`

`url_scan(ptr, len, out) -> i32` scans:

- scheme span;
- authority span;
- host span;
- optional decimal `u16` port after the last colon;
- path span;
- query span;
- fragment span.

`url_scheme_validate(ptr, len) -> i32` validates the source scheme character
set without lowercasing. Lowercasing was Rust API policy, not wire behavior.

The scanner requires `://`, rejects empty authority, rejects empty host before a
port, and rejects non-decimal or overflowing ports. Userinfo and bracketed IPv6
are explicitly unsupported; the source crate did not implement those URL forms
as useful behavior.

## Composed behavior

`Url::append_query_pair` duplicates percent-encoding behavior. Do not port it as
URL behavior. Compose query mutation through:

- `standards/build/wasm/codec-primitives/percent-url-form.wat`

`Url::to_string` is API reconstruction over scanned spans plus optional query
composition. It is not a separate standard unless a caller needs canonical URL
emission as behavior.

## Deletion status

`edgerun-url` is deleted. Do not keep or restore it to preserve Rust object
methods, JSON traits, or string wrappers. No remaining useful behavior beyond
`url-scan.wat` and `percent-url-form.wat` was identified in the source.

Caller demolition should route consumers to URL scan records, not to a restored
`Url` struct.
