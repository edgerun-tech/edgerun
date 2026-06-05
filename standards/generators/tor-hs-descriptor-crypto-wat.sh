#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-descriptor-crypto"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-descriptor-crypto.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_hs_descriptor_crypto.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha3_256 = crypto.hash.sha3.Sha3_256;
const Shake256 = crypto.hash.sha3.Shake256;
const X25519 = crypto.dh.X25519;
const Aes256 = crypto.core.aes.Aes256;
const modes = crypto.core.modes;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_WEAK_KEY: i32 = -4;

const MAX_TEXT: usize = 50000;
const S_KEY_LEN: usize = 32;
const S_IV_LEN: usize = 16;
const MAC_KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const MAC_LEN: usize = 32;
const FIRST = "hsdir-superencrypted-data";
const SECOND = "hsdir-encrypted-data";

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn put64be(out: []u8, v: usize) void {
    var n = v;
    var i: usize = 8;
    while (i > 0) {
        i -= 1;
        out[i] = @as(u8, @intCast(n & 0xff));
        n >>= 8;
    }
}

fn aes256Ctr(key32: []const u8, iv16: []const u8, input: []const u8, out: []u8) void {
    const key = key32[0..32].*;
    const iv = iv16[0..16].*;
    const ctx = Aes256.initEnc(key);
    modes.ctr(@TypeOf(ctx), ctx, out, input, iv, .big);
}

fn derive(secret: []const u8, subcred32: []const u8, revision: u64, salt16: []const u8, constant: []const u8, out80: []u8) i32 {
    if (secret.len > 96) return STATUS_BOUNDS;
    var input: [96 + 32 + 8 + 16 + 32]u8 = undefined;
    var pos: usize = 0;
    @memcpy(input[pos..][0..secret.len], secret); pos += secret.len;
    @memcpy(input[pos..][0..32], subcred32[0..32]); pos += 32;
    put64be(input[pos..][0..8], @intCast(revision)); pos += 8;
    @memcpy(input[pos..][0..16], salt16[0..16]); pos += 16;
    @memcpy(input[pos..][0..constant.len], constant); pos += constant.len;
    Shake256.hash(input[0..pos], out80[0..80], .{});
    return STATUS_OK;
}

fn mac(mac_key32: []const u8, salt16: []const u8, encrypted: []const u8, out32: []u8) void {
    var prefix: [8 + 32 + 8 + 16]u8 = undefined;
    put64be(prefix[0..8], MAC_KEY_LEN);
    @memcpy(prefix[8..40], mac_key32[0..32]);
    put64be(prefix[40..48], SALT_LEN);
    @memcpy(prefix[48..64], salt16[0..16]);
    var h = Sha3_256.init(.{});
    h.update(&prefix);
    h.update(encrypted);
    var digest: [32]u8 = undefined;
    h.final(&digest);
    @memcpy(out32[0..32], &digest);
}

fn crypt(secret: []const u8, subcred32: []const u8, revision: u64, salt16: []const u8, constant: []const u8, input: []const u8, out_payload: []u8) i32 {
    if (input.len > MAX_TEXT) return STATUS_BOUNDS;
    var keys: [80]u8 = undefined;
    const rc = derive(secret, subcred32, revision, salt16, constant, &keys);
    if (rc != STATUS_OK) return rc;
    @memcpy(out_payload[0..16], salt16[0..16]);
    aes256Ctr(keys[0..32], keys[32..48], input, out_payload[16..][0..input.len]);
    mac(keys[48..80], salt16, out_payload[16..][0..input.len], out_payload[16 + input.len ..][0..32]);
    return STATUS_OK;
}

fn decrypt(secret: []const u8, subcred32: []const u8, revision: u64, constant: []const u8, blob: []const u8, out_plain: []u8) i32 {
    if (blob.len < 48) return STATUS_INVALID;
    const enc_len = blob.len - 48;
    if (enc_len > MAX_TEXT) return STATUS_BOUNDS;
    var keys: [80]u8 = undefined;
    const rc = derive(secret, subcred32, revision, blob[0..16], constant, &keys);
    if (rc != STATUS_OK) return rc;
    var expected: [32]u8 = undefined;
    mac(keys[48..80], blob[0..16], blob[16..][0..enc_len], &expected);
    if (!ctEq(&expected, blob[16 + enc_len ..][0..32])) return STATUS_AUTH;
    aes256Ctr(keys[0..32], keys[32..48], blob[16..][0..enc_len], out_plain[0..enc_len]);
    return @intCast(enc_len);
}

export fn proto_standard_id() u32 {
    return 300212;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_hs_desc_salt_len() u32 {
    return SALT_LEN;
}

export fn tor_hs_desc_mac_len() u32 {
    return MAC_LEN;
}

export fn tor_hs_desc_key_material_len() u32 {
    return 80;
}

export fn tor_hs_desc_derive_first(blinded_pub32: [*]const u8, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, salt16: [*]const u8, out80: [*]u8) i32 {
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    return derive(blinded_pub32[0..32], subcred32[0..32], rev, salt16[0..16], FIRST, out80[0..80]);
}

export fn tor_hs_desc_derive_second(blinded_pub32: [*]const u8, cookie32: [*]const u8, cookie_len: u32, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, salt16: [*]const u8, out80: [*]u8) i32 {
    if (cookie_len != 0 and cookie_len != 32) return STATUS_INVALID;
    var secret: [64]u8 = undefined;
    @memcpy(secret[0..32], blinded_pub32[0..32]);
    if (cookie_len == 32) @memcpy(secret[32..64], cookie32[0..32]);
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    return derive(secret[0 .. 32 + cookie_len], subcred32[0..32], rev, salt16[0..16], SECOND, out80[0..80]);
}

export fn tor_hs_desc_encrypt_first(blinded_pub32: [*]const u8, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, salt16: [*]const u8, plain: [*]const u8, plain_len: u32, out_blob: [*]u8) i32 {
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    const rc = crypt(blinded_pub32[0..32], subcred32[0..32], rev, salt16[0..16], FIRST, plain[0..plain_len], out_blob[0 .. 16 + plain_len + 32]);
    if (rc != STATUS_OK) return rc;
    return @intCast(16 + plain_len + 32);
}

export fn tor_hs_desc_decrypt_first(blinded_pub32: [*]const u8, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, blob: [*]const u8, blob_len: u32, out_plain: [*]u8) i32 {
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    return decrypt(blinded_pub32[0..32], subcred32[0..32], rev, FIRST, blob[0..blob_len], out_plain[0..MAX_TEXT]);
}

export fn tor_hs_desc_encrypt_second(blinded_pub32: [*]const u8, cookie32: [*]const u8, cookie_len: u32, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, salt16: [*]const u8, plain: [*]const u8, plain_len: u32, out_blob: [*]u8) i32 {
    if (cookie_len != 0 and cookie_len != 32) return STATUS_INVALID;
    var secret: [64]u8 = undefined;
    @memcpy(secret[0..32], blinded_pub32[0..32]);
    if (cookie_len == 32) @memcpy(secret[32..64], cookie32[0..32]);
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    const rc = crypt(secret[0 .. 32 + cookie_len], subcred32[0..32], rev, salt16[0..16], SECOND, plain[0..plain_len], out_blob[0 .. 16 + plain_len + 32]);
    if (rc != STATUS_OK) return rc;
    return @intCast(16 + plain_len + 32);
}

export fn tor_hs_desc_decrypt_second(blinded_pub32: [*]const u8, cookie32: [*]const u8, cookie_len: u32, subcred32: [*]const u8, revision_hi: u32, revision_lo: u32, blob: [*]const u8, blob_len: u32, out_plain: [*]u8) i32 {
    if (cookie_len != 0 and cookie_len != 32) return STATUS_INVALID;
    var secret: [64]u8 = undefined;
    @memcpy(secret[0..32], blinded_pub32[0..32]);
    if (cookie_len == 32) @memcpy(secret[32..64], cookie32[0..32]);
    const rev = (@as(u64, revision_hi) << 32) | revision_lo;
    return decrypt(secret[0 .. 32 + cookie_len], subcred32[0..32], rev, SECOND, blob[0..blob_len], out_plain[0..MAX_TEXT]);
}

export fn tor_hs_desc_cookie_keys(subcred32: [*]const u8, secret_seed32: [*]const u8, out_client_id8: [*]u8, out_cookie_key32: [*]u8) i32 {
    var input: [64]u8 = undefined;
    @memcpy(input[0..32], subcred32[0..32]);
    @memcpy(input[32..64], secret_seed32[0..32]);
    var keys: [40]u8 = undefined;
    Shake256.hash(&input, &keys, .{});
    @memcpy(out_client_id8[0..8], keys[0..8]);
    @memcpy(out_cookie_key32[0..32], keys[8..40]);
    return STATUS_OK;
}

export fn tor_hs_desc_wrap_cookie(cookie_key32: [*]const u8, iv16: [*]const u8, cookie32: [*]const u8, out32: [*]u8) i32 {
    aes256Ctr(cookie_key32[0..32], iv16[0..16], cookie32[0..32], out32[0..32]);
    return STATUS_OK;
}

export fn tor_hs_desc_client_cookie_keys(subcred32: [*]const u8, client_secret32: [*]const u8, service_ephemeral_pub32: [*]const u8, out_client_id8: [*]u8, out_cookie_key32: [*]u8) i32 {
    const sk = client_secret32[0..32].*;
    const pk = service_ephemeral_pub32[0..32].*;
    const shared = X25519.scalarmult(sk, pk) catch return STATUS_WEAK_KEY;
    return tor_hs_desc_cookie_keys(subcred32, &shared, out_client_id8, out_cookie_key32);
}
ZIG

zig build-exe "$WORK_DIR/tor_hs_descriptor_crypto.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-hs-descriptor-crypto.wasm"

wasm2wat "$WORK_DIR/tor-hs-descriptor-crypto.wasm" -o "$OUT_DIR/tor-hs-descriptor-crypto.wat"
wat2wasm "$OUT_DIR/tor-hs-descriptor-crypto.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
