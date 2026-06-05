#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-identity"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-identity.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_hs_identity.zig" <<'ZIG'
const std = @import("std");
const Sha3_256 = std.crypto.hash.sha3.Sha3_256;
const Ed25519 = std.crypto.sign.Ed25519;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_TORSION: i32 = -3;
const VERSION: u8 = 3;
const CHECKSUM_PREFIX = ".onion checksum";
const CRED_PREFIX = "credential";
const SUBCRED_PREFIX = "subcredential";
const ALPHABET = "abcdefghijklmnopqrstuvwxyz234567";

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn b32Value(c: u8) i32 {
    if (c >= 'a' and c <= 'z') return c - 'a';
    if (c >= 'A' and c <= 'Z') return c - 'A';
    if (c >= '2' and c <= '7') return 26 + c - '2';
    return -1;
}

fn checksum(pubkey: []const u8, out2: []u8) void {
    var buf: [CHECKSUM_PREFIX.len + 32 + 1]u8 = undefined;
    @memcpy(buf[0..CHECKSUM_PREFIX.len], CHECKSUM_PREFIX);
    @memcpy(buf[CHECKSUM_PREFIX.len..][0..32], pubkey[0..32]);
    buf[CHECKSUM_PREFIX.len + 32] = VERSION;
    var digest: [32]u8 = undefined;
    Sha3_256.hash(&buf, &digest, .{});
    out2[0] = digest[0];
    out2[1] = digest[1];
}

fn base32Encode35(raw: []const u8, out56: []u8) void {
    var bitbuf: u32 = 0;
    var bits: u5 = 0;
    var out_i: usize = 0;
    for (raw) |b| {
        bitbuf = (bitbuf << 8) | b;
        bits += 8;
        while (bits >= 5) {
            bits -= 5;
            out56[out_i] = ALPHABET[@as(usize, @intCast((bitbuf >> bits) & 31))];
            out_i += 1;
        }
    }
}

fn base32Decode56(input: []const u8, out35: []u8) i32 {
    var bitbuf: u32 = 0;
    var bits: u5 = 0;
    var out_i: usize = 0;
    for (input) |c| {
        const v = b32Value(c);
        if (v < 0) return STATUS_INVALID;
        bitbuf = (bitbuf << 5) | @as(u32, @intCast(v));
        bits += 5;
        if (bits >= 8) {
            bits -= 8;
            if (out_i >= 35) return STATUS_INVALID;
            out35[out_i] = @truncate(bitbuf >> bits);
            out_i += 1;
        }
    }
    if (out_i != 35 or bits != 0) return STATUS_INVALID;
    return STATUS_OK;
}

fn validSuffix(addr: [*]const u8) bool {
    return addr[56] == '.' and addr[57] == 'o' and addr[58] == 'n' and addr[59] == 'i' and addr[60] == 'o' and addr[61] == 'n';
}

export fn proto_standard_id() u32 {
    return 300211;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_hs_identity_onion_len() u32 {
    return 62;
}

export fn tor_hs_identity_raw_len() u32 {
    return 35;
}

export fn tor_hs_identity_build_onion(pubkey32: [*]const u8, out62: [*]u8) i32 {
    var raw: [35]u8 = undefined;
    @memcpy(raw[0..32], pubkey32[0..32]);
    checksum(pubkey32[0..32], raw[32..34]);
    raw[34] = VERSION;
    base32Encode35(&raw, out62[0..56]);
    @memcpy(out62[56..62], ".onion");
    return STATUS_OK;
}

export fn tor_hs_identity_validate_onion(addr: [*]const u8, len: u32, out_pubkey32: [*]u8) i32 {
    if (len != 62) return STATUS_INVALID;
    if (!validSuffix(addr)) return STATUS_INVALID;
    var raw: [35]u8 = undefined;
    const rc = base32Decode56(addr[0..56], &raw);
    if (rc != STATUS_OK) return rc;
    if (raw[34] != VERSION) return STATUS_INVALID;
    var expected: [2]u8 = undefined;
    checksum(raw[0..32], &expected);
    if (!ctEq(raw[32..34], &expected)) return STATUS_INVALID;
    _ = Ed25519.PublicKey.fromBytes(raw[0..32].*) catch return STATUS_TORSION;
    @memcpy(out_pubkey32[0..32], raw[0..32]);
    return STATUS_OK;
}

export fn tor_hs_identity_credential(pubkey32: [*]const u8, out32: [*]u8) i32 {
    var buf: [CRED_PREFIX.len + 32]u8 = undefined;
    @memcpy(buf[0..CRED_PREFIX.len], CRED_PREFIX);
    @memcpy(buf[CRED_PREFIX.len..][0..32], pubkey32[0..32]);
    var digest: [32]u8 = undefined;
    Sha3_256.hash(&buf, &digest, .{});
    @memcpy(out32[0..32], &digest);
    return STATUS_OK;
}

export fn tor_hs_identity_subcredential(pubkey32: [*]const u8, blinded_pubkey32: [*]const u8, out32: [*]u8) i32 {
    var cred: [32]u8 = undefined;
    _ = tor_hs_identity_credential(pubkey32, &cred);
    var buf: [SUBCRED_PREFIX.len + 32 + 32]u8 = undefined;
    @memcpy(buf[0..SUBCRED_PREFIX.len], SUBCRED_PREFIX);
    @memcpy(buf[SUBCRED_PREFIX.len..][0..32], &cred);
    @memcpy(buf[SUBCRED_PREFIX.len + 32..][0..32], blinded_pubkey32[0..32]);
    var digest: [32]u8 = undefined;
    Sha3_256.hash(&buf, &digest, .{});
    @memcpy(out32[0..32], &digest);
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_hs_identity.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-hs-identity.wasm"

wasm2wat "$WORK_DIR/tor-hs-identity.wasm" -o "$OUT_DIR/tor-hs-identity.wat"
wat2wasm "$OUT_DIR/tor-hs-identity.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
