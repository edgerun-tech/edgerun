#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-intro-auth"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-intro-auth.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_hs_intro_auth.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha3_256 = crypto.hash.sha3.Sha3_256;
const Ed25519 = crypto.sign.Ed25519;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_CRYPTO: i32 = -4;

const DOMAIN = "Tor establish-intro cell v1";
const AUTH_KEY_TYPE_ED25519: u8 = 2;
const AUTH_KEY_LEN: usize = 32;
const HANDSHAKE_AUTH_LEN: usize = 32;
const SIG_LEN: usize = 64;
const MAX_BODY: usize = 509;
const NO_EXT: [1]u8 = .{0};

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn put16be(out: []u8, v: usize) void {
    out[0] = @intCast((v >> 8) & 0xff);
    out[1] = @intCast(v & 0xff);
}

fn put64be(out: []u8, v: u64) void {
    var n = v;
    var i: usize = 8;
    while (i > 0) {
        i -= 1;
        out[i] = @intCast(n & 0xff);
        n >>= 8;
    }
}

fn parseExtensions(ext: []const u8) i32 {
    if (ext.len < 1) return STATUS_INVALID;
    var pos: usize = 1;
    var n: usize = ext[0];
    while (n > 0) {
        if (pos + 2 > ext.len) return STATUS_INVALID;
        const ext_len: usize = ext[pos + 1];
        pos += 2;
        if (pos + ext_len > ext.len) return STATUS_INVALID;
        pos += ext_len;
        n -= 1;
    }
    if (pos != ext.len) return STATUS_INVALID;
    return STATUS_OK;
}

fn handshakeAuth(kh32: []const u8, prior: []const u8, out32: []u8) void {
    var h = Sha3_256.init(.{});
    h.update(kh32[0..32]);
    h.update(prior);
    var digest: [32]u8 = undefined;
    h.final(&digest);
    @memcpy(out32[0..32], &digest);
}

fn writePrior(out: []u8, auth_pub32: []const u8, extensions: []const u8) usize {
    out[0] = AUTH_KEY_TYPE_ED25519;
    out[1] = 0;
    out[2] = 32;
    @memcpy(out[3..35], auth_pub32[0..32]);
    @memcpy(out[35..][0..extensions.len], extensions);
    return 35 + extensions.len;
}

fn sigMessage(body_prefix: []const u8, out: []u8) []u8 {
    @memcpy(out[0..DOMAIN.len], DOMAIN);
    @memcpy(out[DOMAIN.len..][0..body_prefix.len], body_prefix);
    return out[0 .. DOMAIN.len + body_prefix.len];
}

export fn proto_standard_id() u32 {
    return 300215;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_hs_intro_auth_noext_len() u32 {
    return 134;
}

export fn tor_hs_intro_auth_domain_len() u32 {
    return DOMAIN.len;
}

export fn tor_hs_intro_auth_public(seed32: [*]const u8, out_pub32: [*]u8) i32 {
    const seed = seed32[0..32].*;
    const kp = Ed25519.KeyPair.generateDeterministic(seed) catch return STATUS_CRYPTO;
    const public_bytes = kp.public_key.toBytes();
    @memcpy(out_pub32[0..32], &public_bytes);
    return STATUS_OK;
}

export fn tor_hs_intro_auth_build_dos_extension(out_ext: [*]u8, rate_per_sec: u32, burst_per_sec: u32) i32 {
    out_ext[0] = 1;
    out_ext[1] = 1;
    out_ext[2] = 19;
    out_ext[3] = 2;
    out_ext[4] = 1;
    put64be(out_ext[5..13], rate_per_sec);
    out_ext[13] = 2;
    put64be(out_ext[14..22], burst_per_sec);
    return 22;
}

export fn tor_hs_intro_auth_derive(kh32: [*]const u8, auth_pub32: [*]const u8, extensions: [*]const u8, extensions_len: u32, out32: [*]u8) i32 {
    const ext = if (extensions_len == 0) NO_EXT[0..] else extensions[0..extensions_len];
    if (ext.len > MAX_BODY) return STATUS_BOUNDS;
    const parse_rc = parseExtensions(ext);
    if (parse_rc != STATUS_OK) return parse_rc;
    var prior: [MAX_BODY]u8 = undefined;
    const prior_len = writePrior(&prior, auth_pub32[0..32], ext);
    handshakeAuth(kh32[0..32], prior[0..prior_len], out32[0..32]);
    return STATUS_OK;
}

export fn tor_hs_intro_auth_sign(seed32: [*]const u8, kh32: [*]const u8, extensions: [*]const u8, extensions_len: u32, out_body: [*]u8) i32 {
    const ext = if (extensions_len == 0) NO_EXT[0..] else extensions[0..extensions_len];
    if (35 + ext.len + HANDSHAKE_AUTH_LEN + 2 + SIG_LEN > MAX_BODY) return STATUS_BOUNDS;
    const parse_rc = parseExtensions(ext);
    if (parse_rc != STATUS_OK) return parse_rc;

    const seed = seed32[0..32].*;
    const kp = Ed25519.KeyPair.generateDeterministic(seed) catch return STATUS_CRYPTO;
    const public_bytes = kp.public_key.toBytes();
    var body_prefix: [MAX_BODY]u8 = undefined;
    var pos = writePrior(&body_prefix, &public_bytes, ext);
    handshakeAuth(kh32[0..32], body_prefix[0..pos], body_prefix[pos..][0..32]);
    pos += 32;

    var msg: [DOMAIN.len + MAX_BODY]u8 = undefined;
    const signed_msg = sigMessage(body_prefix[0..pos], &msg);
    const sig = Ed25519.KeyPair.sign(kp, signed_msg, null) catch return STATUS_CRYPTO;
    const sig_bytes = sig.toBytes();

    @memcpy(out_body[0..pos], body_prefix[0..pos]);
    put16be(out_body[pos..][0..2], SIG_LEN);
    pos += 2;
    @memcpy(out_body[pos..][0..64], &sig_bytes);
    pos += 64;
    return @intCast(pos);
}

export fn tor_hs_intro_auth_verify(body: [*]const u8, body_len: u32, kh32: [*]const u8, out_auth_pub32: [*]u8) i32 {
    if (body_len < 35 + 1 + 32 + 2 + 64 or body_len > MAX_BODY) return STATUS_BOUNDS;
    if (body[0] != AUTH_KEY_TYPE_ED25519 or body[1] != 0 or body[2] != 32) return STATUS_INVALID;
    var pos: usize = 35;
    const n_ext = body[pos];
    pos += 1;
    var n: usize = n_ext;
    while (n > 0) {
        if (pos + 2 > body_len) return STATUS_INVALID;
        const ext_len: usize = body[pos + 1];
        pos += 2;
        if (pos + ext_len > body_len) return STATUS_INVALID;
        pos += ext_len;
        n -= 1;
    }
    const handshake_off = pos;
    if (handshake_off + 32 + 2 + 64 != body_len) return STATUS_INVALID;
    var expected: [32]u8 = undefined;
    handshakeAuth(kh32[0..32], body[0..handshake_off], &expected);
    if (!ctEq(&expected, body[handshake_off..][0..32])) return STATUS_AUTH;
    const sig_len_off = handshake_off + 32;
    if (body[sig_len_off] != 0 or body[sig_len_off + 1] != 64) return STATUS_INVALID;
    const pk = Ed25519.PublicKey.fromBytes(body[3..35].*) catch return STATUS_CRYPTO;
    const sig = Ed25519.Signature.fromBytes(body[sig_len_off + 2 ..][0..64].*);
    var msg: [DOMAIN.len + MAX_BODY]u8 = undefined;
    const signed_msg = sigMessage(body[0..sig_len_off], &msg);
    Ed25519.Signature.verify(sig, signed_msg, pk) catch return STATUS_AUTH;
    @memcpy(out_auth_pub32[0..32], body[3..35]);
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_hs_intro_auth.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-hs-intro-auth.wasm"

wasm2wat "$WORK_DIR/tor-hs-intro-auth.wasm" -o "$OUT_DIR/tor-hs-intro-auth.wat"
wat2wasm "$OUT_DIR/tor-hs-intro-auth.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
