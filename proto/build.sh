#!/bin/bash
set -e

# Check if prost plugin exists
if ! which protoc-gen-prost >/dev/null 2>&1; then
    echo "protoc-gen-prost not found, trying cargo..."

    # Create temp project to build prost
    TMPDIR=$(mktemp -d)
    cd "$TMPDIR"

    cat > Cargo.toml << 'EOF'
[package]
name = "prost-gen"
version = "0.1.0"
edition = "2021"

[dependencies]
prost = "0.14"

[build-dependencies]
prost-build = "0.14"
EOF

    mkdir -p src
    cat > src/main.rs << 'EOF'
fn main() {
    prost_build::build::configure()
        .out_dir(std::env::var_os("OUT_DIR").unwrap())
        .run()
        .unwrap();
}
EOF

    cat > build.rs << 'EOF'
fn main() {
    prost_build::build::build().exit(
        std::io::success(),
        "prost-gen",
        &["--help"],
    );
}
EOF

    cargo build --release 2>&1 | tail -5
    cp target/release/prost-gen ~/.cargo/bin/protoc-gen-prost 2>/dev/null || true
    cd -
    rm -rf "$TMPDIR"
fi

echo "Done"