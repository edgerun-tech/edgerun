# edgerun-json

`edgerun-json` is a small JSON value crate for Rust with owned, borrowed,
tape, and compiled-schema paths. It is designed for `no_std` targets with
`alloc`, while still offering host-only helpers behind the `std` feature.

## Quick Start

```toml
[dependencies]
edgerun-json = "1.0"
```

```rust
use edgerun_json::{from_str, json, to_string, Value};

let value: Value = from_str(r#"{"ok":true,"n":7}"#)?;
assert_eq!(value["ok"].as_bool(), Some(true));
assert_eq!(value["n"].as_i64(), Some(7));

let built = json!({"msg": "hello", "items": [1, 2, null]});
let encoded = to_string(&built)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Modes

- `JsonValue` / `Value`: owned JSON tree.
- `BorrowedJsonValue`: borrowed parse tree for low-allocation inspection.
- Tape parsing: token stream for fast structural scans and repeated lookups.
- Compiled schemas: efficient serialization for known object shapes.
- Native model helpers: `FromJson`, `ToJson`, `from_str_as`, `from_value_as`,
  and `impl_json_struct!` for explicit protocol models.

## Features

| Feature | Description |
|---------|-------------|
| `std` | Enables host I/O helpers. |
| `yaml` | Enables YAML value parsing and serialization. |
| `toml` | Enables TOML value parsing and serialization. |

## Testing

```bash
cargo test -p edgerun-json --lib
cargo test -p edgerun-json --test model_macro
cargo test -p edgerun-json --test unicode
```

For bare-target validation:

```bash
cargo +nightly build -p edgerun-json --release --target x86_64-unknown-none -Zbuild-std=core,alloc
```
