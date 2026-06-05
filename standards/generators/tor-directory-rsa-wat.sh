#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-directory-rsa"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-directory-rsa.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_directory_rsa.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha1 = crypto.hash.Sha1;
const Sha256 = crypto.hash.sha2.Sha256;
const Rsa = crypto.Certificate.rsa;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;
const ALG_SHA1_RSA_PKCS1: u32 = 1;
const ALG_SHA256_RSA_PKCS1: u32 = 2;
const MAX_DOC_LEN: u32 = 1024 * 1024;

fn verifyFixed(comptime modulus_len: usize, document: []const u8, exponent: []const u8, modulus: []const u8, signature: []const u8, algorithm: u32) i32 {
    if (modulus.len != modulus_len or signature.len != modulus_len) return STATUS_BOUNDS;
    const pk = Rsa.PublicKey.fromBytes(exponent, modulus) catch return STATUS_INVALID;
    const sig = Rsa.PKCS1v1_5Signature.fromBytes(modulus_len, signature);
    switch (algorithm) {
        ALG_SHA1_RSA_PKCS1 => Rsa.PKCS1v1_5Signature.verify(modulus_len, sig, document, pk, Sha1) catch return STATUS_AUTH,
        ALG_SHA256_RSA_PKCS1 => Rsa.PKCS1v1_5Signature.verify(modulus_len, sig, document, pk, Sha256) catch return STATUS_AUTH,
        else => return STATUS_UNSUPPORTED,
    }
    return STATUS_OK;
}

export fn proto_standard_id() u32 {
    return 300223;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_dir_rsa_algorithm_sha1_pkcs1() u32 {
    return ALG_SHA1_RSA_PKCS1;
}

export fn tor_dir_rsa_algorithm_sha256_pkcs1() u32 {
    return ALG_SHA256_RSA_PKCS1;
}

export fn tor_dir_rsa_verify_pkcs1(document: [*]const u8, document_len: u32, exponent: [*]const u8, exponent_len: u32, modulus: [*]const u8, modulus_len: u32, signature: [*]const u8, signature_len: u32, algorithm: u32) i32 {
    if (document_len > MAX_DOC_LEN) return STATUS_BOUNDS;
    if (exponent_len == 0 or exponent_len > 4) return STATUS_BOUNDS;
    return switch (modulus_len) {
        64 => verifyFixed(64, document[0..document_len], exponent[0..exponent_len], modulus[0..64], signature[0..signature_len], algorithm),
        128 => verifyFixed(128, document[0..document_len], exponent[0..exponent_len], modulus[0..128], signature[0..signature_len], algorithm),
        256 => verifyFixed(256, document[0..document_len], exponent[0..exponent_len], modulus[0..256], signature[0..signature_len], algorithm),
        512 => verifyFixed(512, document[0..document_len], exponent[0..exponent_len], modulus[0..512], signature[0..signature_len], algorithm),
        else => STATUS_UNSUPPORTED,
    };
}
ZIG

zig build-exe "$WORK_DIR/tor_directory_rsa.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-directory-rsa.wasm"

wasm2wat "$WORK_DIR/tor-directory-rsa.wasm" -o "$OUT_DIR/tor-directory-rsa.wat"
wat2wasm "$OUT_DIR/tor-directory-rsa.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-directory-rsa.wat"
