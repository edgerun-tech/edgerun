#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
out_dir="${1:-$repo_root/out/nodeos}"
spec_path="${2:-$out_dir/initramfs.list}"
cargo_target_dir="${CARGO_TARGET_DIR:-$repo_root/out/target}"
rust_target="x86_64-unknown-linux-musl"
profile="release"
bin_name="edgerun-node-manager"
built_bin="$cargo_target_dir/$rust_target/$profile/$bin_name"
staged_bin="$out_dir/$bin_name"

for cmd in cargo rustup strip ldd awk sort uniq dirname mkdir cp file sha256sum; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

mkdir -p "$out_dir"

if ! rustup target list --installed | grep -q "^${rust_target}$"; then
  rustup target add "$rust_target"
fi

# musl target is expected to produce static binaries; enforce it explicitly.
CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-musl-gcc}" \
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
CARGO_TARGET_DIR="$cargo_target_dir" \
cargo build -p "$bin_name" --profile "$profile" --target "$rust_target"

cp -f "$built_bin" "$staged_bin"
strip --strip-unneeded "$staged_bin"

if ldd "$staged_bin" 2>&1 | grep -Eq "not a dynamic executable|statically linked"; then
  :
else
  echo "binary is not fully static: $staged_bin" >&2
  ldd "$staged_bin" >&2 || true
  exit 1
fi

tmp_spec="$(mktemp "$out_dir/.initramfs.XXXXXX.list")"
tmp_libs="$(mktemp "$out_dir/.initramfs.libs.XXXXXX")"
trap 'rm -f "$tmp_spec" "$tmp_libs"' EXIT

cat >"$tmp_spec" <<EOF_SPEC
# SPDX-License-Identifier: Apache-2.0
# Kernel initramfs source list. Use:
#   CONFIG_INITRAMFS_SOURCE="$spec_path"
dir /dev 755 0 0
nod /dev/console 600 0 0 c 5 1
nod /dev/null 666 0 0 c 1 3
dir /proc 555 0 0
dir /sys 555 0 0
dir /run 755 0 0
dir /tmp 1777 0 0
dir /etc 755 0 0
dir /etc/ssl 755 0 0
dir /etc/ssl/certs 755 0 0
dir /usr 755 0 0
dir /usr/bin 755 0 0
file /init $staged_bin 755 0 0
EOF_SPEC

add_file_line() {
  local dst="$1"
  local src="$2"
  local mode="$3"
  if [[ -f "$src" ]]; then
    echo "file $dst $src $mode 0 0" >>"$tmp_spec"
  fi
}

add_tool_with_libs() {
  local tool="$1"
  local src
  src="$(command -v "$tool" || true)"
  if [[ -z "$src" ]]; then
    return 0
  fi
  add_file_line "/usr/bin/$tool" "$src" 755
  ldd "$src" \
    | awk '/=> \// { print $3 } /^[[:space:]]*\/[^[:space:]]+[[:space:]]+\(/ { print $1 }' \
    >>"$tmp_libs" || true
}

add_tool_with_libs "tpm2_getcap"
add_tool_with_libs "tpm2_nvreadpublic"
add_tool_with_libs "tpm2_nvdefine"
add_tool_with_libs "tpm2_nvread"
add_tool_with_libs "tpm2_nvwrite"
add_tool_with_libs "agave-validator"
add_tool_with_libs "solana-keygen"

if [[ -s "$tmp_libs" ]]; then
  sort -u "$tmp_libs" | while IFS= read -r lib; do
    [[ -z "$lib" ]] && continue
    [[ -f "$lib" ]] || continue
    add_file_line "$lib" "$lib" 755
  done
fi

if [[ -f /etc/ssl/certs/ca-certificates.crt ]]; then
  add_file_line "/etc/ssl/certs/ca-certificates.crt" "/etc/ssl/certs/ca-certificates.crt" 644
fi

mv "$tmp_spec" "$spec_path"

echo "initramfs_spec=$spec_path"
echo "manager_binary=$staged_bin"
echo "manager_binary_file_type=$(file -b "$staged_bin")"
echo "manager_sha256=$(sha256sum "$staged_bin" | awk '{print $1}')"
