#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
out_dir="${1:-$repo_root/out/nodeos}"
target="x86_64-unknown-uefi"

mkdir -p "$out_dir"

cd "$repo_root"
rustup target add "$target"

cargo build -p edgerun-bootloader-efi --release --features efi-app --target "$target"

target_dir="${CARGO_TARGET_DIR:-$repo_root/out/target}"
src="$target_dir/$target/release/edgerun-bootloader-efi.efi"
dst="$out_dir/edgerun-bootloader.unsigned.efi"
cp "$src" "$dst"

echo "bootloader_efi=$dst"
sha256sum "$dst"
