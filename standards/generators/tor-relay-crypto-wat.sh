#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-relay-crypto"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-relay-crypto.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_relay_crypto.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha1 = crypto.hash.Sha1;
const Sha256 = crypto.hash.sha2.Sha256;
const Aes128 = crypto.core.aes.Aes128;
const modes = crypto.core.modes;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;

fn incCtrBe(iv: []u8, blocks: usize) void {
    var n = blocks;
    while (n > 0) : (n -= 1) {
        var i: usize = 16;
        while (i > 0) {
            i -= 1;
            iv[i] +%= 1;
            if (iv[i] != 0) break;
        }
    }
}

export fn proto_standard_id() u32 {
    return 300218;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_relay_digest_state_len() u32 {
    return 20;
}

export fn tor_relay_digest_update20_sha1(state20: [*]const u8, payload: [*]const u8, payload_len: u32, out20: [*]u8) i32 {
    if (payload_len > 509) return STATUS_BOUNDS;
    var h = Sha1.init(.{});
    h.update(state20[0..20]);
    h.update(payload[0..payload_len]);
    var digest: [20]u8 = undefined;
    h.final(&digest);
    @memcpy(out20[0..20], &digest);
    return STATUS_OK;
}

export fn tor_relay_digest4_le_sha1(state20: [*]const u8, payload: [*]const u8, payload_len: u32) u32 {
    var digest: [20]u8 = undefined;
    const rc = tor_relay_digest_update20_sha1(state20, payload, payload_len, &digest);
    if (rc != STATUS_OK) return 0;
    return @as(u32, digest[0]) |
        (@as(u32, digest[1]) << 8) |
        (@as(u32, digest[2]) << 16) |
        (@as(u32, digest[3]) << 24);
}

export fn tor_relay_digest_update32(state32: [*]const u8, payload: [*]const u8, payload_len: u32, out32: [*]u8) i32 {
    if (payload_len > 509) return STATUS_BOUNDS;
    var h = Sha256.init(.{});
    h.update(state32[0..32]);
    h.update(payload[0..payload_len]);
    var digest: [32]u8 = undefined;
    h.final(&digest);
    @memcpy(out32[0..32], &digest);
    return STATUS_OK;
}

export fn tor_relay_digest4_le(state32: [*]const u8, payload: [*]const u8, payload_len: u32) u32 {
    var digest: [32]u8 = undefined;
    const rc = tor_relay_digest_update32(state32, payload, payload_len, &digest);
    if (rc != STATUS_OK) return 0;
    return @as(u32, digest[0]) |
        (@as(u32, digest[1]) << 8) |
        (@as(u32, digest[2]) << 16) |
        (@as(u32, digest[3]) << 24);
}

export fn tor_relay_aes128_ctr_crypt(key16: [*]const u8, iv16: [*]u8, input: [*]const u8, len: u32, out: [*]u8) i32 {
    if (len > 65536) return STATUS_BOUNDS;
    const key = key16[0..16].*;
    const ctx = Aes128.initEnc(key);
    const iv = iv16[0..16].*;
    modes.ctr(@TypeOf(ctx), ctx, out[0..len], input[0..len], iv, .big);
    incCtrBe(iv16[0..16], (@as(usize, len) + 15) / 16);
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_relay_crypto.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-relay-crypto.wasm"

wasm2wat "$WORK_DIR/tor-relay-crypto.wasm" -o "$OUT_DIR/tor-relay-crypto.wat"
echo "wrote $OUT_DIR/tor-relay-crypto.wat"
