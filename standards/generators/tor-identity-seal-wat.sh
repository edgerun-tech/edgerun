#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-identity-seal"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-identity-seal.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_identity_seal.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const X25519 = crypto.dh.X25519;
const HmacSha256 = crypto.auth.hmac.sha2.HmacSha256;
const Sha256 = crypto.hash.sha2.Sha256;
const Aes256 = crypto.core.aes.Aes256;
const modes = crypto.core.modes;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_WEAK_KEY: i32 = -4;
const MAX_MSG: usize = 65536;
const HEADER_LEN: usize = 48; // ephemeral pubkey32 || iv16
const MAC_LEN: usize = 32;
const OVERHEAD: usize = HEADER_LEN + MAC_LEN;
const LABEL_EXTRACT = "edgerun:tor:identity-seal:v1:extract";
const LABEL_EXPAND = "edgerun:tor:identity-seal:v1:expand";
const LABEL_MAC = "edgerun:tor:identity-seal:v1:mac";

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn allZero(bytes: []const u8) bool {
    var acc: u8 = 0;
    for (bytes) |b| acc |= b;
    return acc == 0;
}

fn hmac(out: []u8, key: []const u8, msg: []const u8) void {
    HmacSha256.create(out[0..32], msg, key);
}

fn derive(shared32: []const u8, sender_id32: []const u8, recipient_id32: []const u8, eph_pub32: []const u8, out_key64: []u8) void {
    var extract_input: [32 + 32 + 32 + 32]u8 = undefined;
    @memcpy(extract_input[0..32], shared32[0..32]);
    @memcpy(extract_input[32..64], sender_id32[0..32]);
    @memcpy(extract_input[64..96], recipient_id32[0..32]);
    @memcpy(extract_input[96..128], eph_pub32[0..32]);
    var prk: [32]u8 = undefined;
    hmac(&prk, LABEL_EXTRACT, &extract_input);

    var block1_input: [LABEL_EXPAND.len + 1]u8 = undefined;
    @memcpy(block1_input[0..LABEL_EXPAND.len], LABEL_EXPAND);
    block1_input[LABEL_EXPAND.len] = 1;
    var t1: [32]u8 = undefined;
    hmac(&t1, &prk, &block1_input);

    var block2_input: [32 + LABEL_EXPAND.len + 1]u8 = undefined;
    @memcpy(block2_input[0..32], &t1);
    @memcpy(block2_input[32..][0..LABEL_EXPAND.len], LABEL_EXPAND);
    block2_input[32 + LABEL_EXPAND.len] = 2;
    var t2: [32]u8 = undefined;
    hmac(&t2, &prk, &block2_input);
    @memcpy(out_key64[0..32], &t1);
    @memcpy(out_key64[32..64], &t2);
}

fn aes256Ctr(key32: []const u8, iv16: []const u8, input: []const u8, out: []u8) void {
    const key = key32[0..32].*;
    const iv = iv16[0..16].*;
    const ctx = Aes256.initEnc(key);
    modes.ctr(@TypeOf(ctx), ctx, out, input, iv, .big);
}

fn macSeal(mac_key32: []const u8, sender_id32: []const u8, recipient_id32: []const u8, header48: []const u8, ciphertext: []const u8, out32: []u8) void {
    var h = HmacSha256.init(mac_key32[0..32]);
    h.update(LABEL_MAC);
    h.update(sender_id32[0..32]);
    h.update(recipient_id32[0..32]);
    h.update(header48[0..48]);
    h.update(ciphertext);
    h.final(out32[0..32]);
}

export fn proto_standard_id() u32 {
    return 300220;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_identity_seal_overhead() u32 {
    return OVERHEAD;
}

export fn tor_identity_seal_public(secret32: [*]const u8, out_pub32: [*]u8) i32 {
    const sk = secret32[0..32].*;
    const pk = X25519.recoverPublicKey(sk) catch return STATUS_WEAK_KEY;
    @memcpy(out_pub32[0..32], &pk);
    return STATUS_OK;
}

export fn tor_identity_seal_key_material(sender_id32: [*]const u8, recipient_pub32: [*]const u8, eph_secret32: [*]const u8, out_eph_pub32: [*]u8, out_key64: [*]u8) i32 {
    const sk = eph_secret32[0..32].*;
    const recipient = recipient_pub32[0..32].*;
    const eph_pub = X25519.recoverPublicKey(sk) catch return STATUS_WEAK_KEY;
    const shared = X25519.scalarmult(sk, recipient) catch return STATUS_WEAK_KEY;
    if (allZero(&shared)) return STATUS_WEAK_KEY;
    @memcpy(out_eph_pub32[0..32], &eph_pub);
    derive(&shared, sender_id32[0..32], recipient_pub32[0..32], &eph_pub, out_key64[0..64]);
    return STATUS_OK;
}

export fn tor_identity_seal(sender_id32: [*]const u8, recipient_pub32: [*]const u8, eph_secret32: [*]const u8, iv16: [*]const u8, plain: [*]const u8, plain_len: u32, out_box: [*]u8) i32 {
    if (plain_len > MAX_MSG) return STATUS_BOUNDS;
    var eph_pub: [32]u8 = undefined;
    var keys: [64]u8 = undefined;
    const rc = tor_identity_seal_key_material(sender_id32, recipient_pub32, eph_secret32, &eph_pub, &keys);
    if (rc != STATUS_OK) return rc;
    @memcpy(out_box[0..32], &eph_pub);
    @memcpy(out_box[32..48], iv16[0..16]);
    aes256Ctr(keys[0..32], iv16[0..16], plain[0..plain_len], out_box[48..][0..plain_len]);
    macSeal(keys[32..64], sender_id32[0..32], recipient_pub32[0..32], out_box[0..48], out_box[48..][0..plain_len], out_box[48 + plain_len ..][0..32]);
    return @intCast(OVERHEAD + plain_len);
}

export fn tor_identity_open(sender_id32: [*]const u8, recipient_secret32: [*]const u8, recipient_pub32: [*]const u8, sealed: [*]const u8, sealed_len: u32, out_plain: [*]u8) i32 {
    if (sealed_len < OVERHEAD) return STATUS_INVALID;
    const cipher_len = sealed_len - OVERHEAD;
    if (cipher_len > MAX_MSG) return STATUS_BOUNDS;
    const sk = recipient_secret32[0..32].*;
    const eph_pub = sealed[0..32].*;
    const shared = X25519.scalarmult(sk, eph_pub) catch return STATUS_WEAK_KEY;
    if (allZero(&shared)) return STATUS_WEAK_KEY;
    var keys: [64]u8 = undefined;
    derive(&shared, sender_id32[0..32], recipient_pub32[0..32], sealed[0..32], &keys);
    var expected: [32]u8 = undefined;
    macSeal(keys[32..64], sender_id32[0..32], recipient_pub32[0..32], sealed[0..48], sealed[48..][0..cipher_len], &expected);
    if (!ctEq(&expected, sealed[48 + cipher_len ..][0..32])) return STATUS_AUTH;
    aes256Ctr(keys[0..32], sealed[32..48], sealed[48..][0..cipher_len], out_plain[0..cipher_len]);
    return @intCast(cipher_len);
}
ZIG

zig build-exe "$WORK_DIR/tor_identity_seal.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-identity-seal.wasm"

wasm2wat "$WORK_DIR/tor-identity-seal.wasm" -o "$OUT_DIR/tor-identity-seal.wat"
wat2wasm "$OUT_DIR/tor-identity-seal.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-identity-seal.wat"
