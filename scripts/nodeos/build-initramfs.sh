#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
out_dir="${1:-$repo_root/out/nodeos}"
spec_path="${2:-$out_dir/initramfs.list}"
initrd_path="${3:-$out_dir/initramfs-edgerun.cpio.gz}"
cargo_target_dir="${CARGO_TARGET_DIR:-$repo_root/out/target}"
rust_target="x86_64-unknown-linux-musl"
profile="release"
manager_bin_name="edgerun-node-manager"
manager_built="$cargo_target_dir/$rust_target/$profile/$manager_bin_name"
manager_staged="$out_dir/$manager_bin_name"
stage_root="$out_dir/initrd-root"

for cmd in cargo rustup strip ldd awk sort uniq dirname mkdir cp file sha256sum cpio gzip chmod find rm; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "required command not found: $cmd" >&2
    exit 1
  fi
done

mkdir -p "$out_dir"
rm -rf "$stage_root"
mkdir -p "$stage_root"

if ! rustup target list --installed | grep -q "^${rust_target}$"; then
  rustup target add "$rust_target"
fi

# musl target is expected to produce static binaries; enforce it explicitly.
CC_x86_64_unknown_linux_musl="${CC_x86_64_unknown_linux_musl:-musl-gcc}" \
RUSTFLAGS="${RUSTFLAGS:-} -C target-feature=+crt-static" \
CARGO_TARGET_DIR="$cargo_target_dir" \
cargo build -p "$manager_bin_name" --profile "$profile" --target "$rust_target"

cp -f "$manager_built" "$manager_staged"
strip --strip-unneeded "$manager_staged"

check_static() {
  local bin="$1"
  if ldd "$bin" 2>&1 | grep -Eq "not a dynamic executable|statically linked"; then
    return 0
  fi
  echo "binary is not fully static: $bin" >&2
  ldd "$bin" >&2 || true
  exit 1
}

check_static "$manager_staged"

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
dir /etc/edgerun 755 0 0
dir /etc/edgerun/secureboot 755 0 0
dir /usr 755 0 0
dir /usr/bin 755 0 0
file /init $manager_staged 755 0 0
EOF_SPEC

stage_dir() {
  local path="$1"
  mkdir -p "$stage_root$path"
}

stage_dir "/dev"
stage_dir "/proc"
stage_dir "/sys"
stage_dir "/run"
stage_dir "/tmp"
stage_dir "/etc/ssl/certs"
stage_dir "/etc/edgerun/secureboot"
stage_dir "/bin"
stage_dir "/usr/bin"
ln -sfn usr/lib "$stage_root/lib"
ln -sfn usr/lib64 "$stage_root/lib64"
ln -sfn ../usr/bin/bash "$stage_root/bin/sh"
cp -f "$manager_staged" "$stage_root/init"
chmod 755 "$stage_root/init"

add_file_line() {
  local dst="$1"
  local src="$2"
  local mode="$3"
  if [[ -f "$src" ]]; then
    echo "file $dst $src $mode 0 0" >>"$tmp_spec"
    mkdir -p "$(dirname "$stage_root$dst")"
    cp -f "$src" "$stage_root$dst"
    chmod "$mode" "$stage_root$dst" || true
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
add_tool_with_libs "efi-updatevar"
add_tool_with_libs "mount"
add_tool_with_libs "bash"
add_tool_with_libs "busybox"
add_tool_with_libs "crun"

if [[ -s "$tmp_libs" ]]; then
  sort -u "$tmp_libs" | while IFS= read -r lib; do
    [[ -z "$lib" ]] && continue
    [[ -f "$lib" ]] || continue
    add_file_line "$lib" "$lib" 755
  done
fi

# tpm2-tools loads TCTI backends via dlopen; these do not appear in ldd output.
for tcti_lib in /usr/lib/libtss2-tcti-*.so*; do
  [[ -e "$tcti_lib" ]] || continue
  add_file_line "$tcti_lib" "$tcti_lib" 755
done

if [[ -f /etc/ssl/certs/ca-certificates.crt ]]; then
  add_file_line "/etc/ssl/certs/ca-certificates.crt" "/etc/ssl/certs/ca-certificates.crt" 644
fi

if [[ -f "$out_dir/secureboot/edgerun-secureboot-db-cert.pem" ]]; then
  add_file_line \
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.pem" \
    "$out_dir/secureboot/edgerun-secureboot-db-cert.pem" \
    644
fi

if [[ -f "$out_dir/secureboot/edgerun-secureboot-db-cert.der" ]]; then
  add_file_line \
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.der" \
    "$out_dir/secureboot/edgerun-secureboot-db-cert.der" \
    644
fi

if command -v cert-to-efi-sig-list >/dev/null 2>&1 && [[ -f "$out_dir/secureboot/edgerun-secureboot-db-cert.pem" ]]; then
  sb_tmp_dir="$(mktemp -d "$out_dir/.sb-esl.XXXXXX")"
  trap 'rm -rf "$sb_tmp_dir"' EXIT
  sb_owner_guid="f3e4a490-8f2d-4f2a-8a0e-5b77c2b8b401"
  for var in PK KEK db; do
    esl_path="$sb_tmp_dir/${var}.esl"
    cert-to-efi-sig-list -g "$sb_owner_guid" "$out_dir/secureboot/edgerun-secureboot-db-cert.pem" "$esl_path"
    add_file_line "/etc/edgerun/secureboot/${var}.esl" "$esl_path" 644
  done
fi

mv "$tmp_spec" "$spec_path"

(
  cd "$stage_root"
  find . -print0 | cpio --null -ov --format=newc | gzip -9 > "$initrd_path"
)

echo "initramfs_spec=$spec_path"
echo "initramfs_image=$initrd_path"
echo "manager_binary=$manager_staged"
echo "manager_binary_file_type=$(file -b "$manager_staged")"
echo "manager_sha256=$(sha256sum "$manager_staged" | awk '{print $1}')"
echo "initrd_sha256=$(sha256sum "$initrd_path" | awk '{print $1}')"
