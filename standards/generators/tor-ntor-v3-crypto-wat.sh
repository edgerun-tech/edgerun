#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-ntor-v3-crypto"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-ntor-v3-crypto.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_ntor_v3_crypto.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;

const X25519 = crypto.dh.X25519;
const Sha3_256 = crypto.hash.sha3.Sha3_256;
const Shake256 = crypto.hash.sha3.Shake256;
const Aes256 = crypto.core.aes.Aes256;
const modes = crypto.core.modes;

const PROTOID = "ntor3-curve25519-sha3_256-1";
const T_MSGKDF = PROTOID ++ ":kdf_phase1";
const T_MSGMAC = PROTOID ++ ":msg_mac";
const T_KEY_SEED = PROTOID ++ ":key_seed";
const T_VERIFY = PROTOID ++ ":verify";
const T_FINAL = PROTOID ++ ":kdf_final";
const T_AUTH = PROTOID ++ ":auth_final";
const SERVER = "Server";

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_WEAK_KEY: i32 = -2;
const STATUS_BOUNDS: i32 = -3;
const MAX_MSG: usize = 512;
const MAX_VER: usize = 128;
const MAX_KEYSTREAM: usize = 512;

fn slice(ptr: [*]const u8, len: usize) []const u8 {
    return ptr[0..len];
}

fn copy(dst: [*]u8, src: []const u8) void {
    @memcpy(dst[0..src.len], src);
}

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

fn put64be(out: []u8, v: usize) void {
    var n = v;
    var i: usize = 8;
    while (i > 0) {
        i -= 1;
        out[i] = @as(u8, @intCast(n & 0xff));
        n >>= 8;
    }
}

fn encapInto(out: []u8, s: []const u8) usize {
    put64be(out[0..8], s.len);
    @memcpy(out[8..][0..s.len], s);
    return 8 + s.len;
}

fn hashTag(out: *[32]u8, s: []const u8, tag: []const u8) void {
    var buf: [8 + 64 + 1024]u8 = undefined;
    var pos = encapInto(buf[0..], tag);
    @memcpy(buf[pos..][0..s.len], s);
    pos += s.len;
    Sha3_256.hash(buf[0..pos], out, .{});
}

fn macTag(out: *[32]u8, key: []const u8, msg: []const u8, tag: []const u8) void {
    var buf: [8 + 64 + 8 + 64 + 1024]u8 = undefined;
    var pos = encapInto(buf[0..], tag);
    pos += encapInto(buf[pos..], key);
    @memcpy(buf[pos..][0..msg.len], msg);
    pos += msg.len;
    Sha3_256.hash(buf[0..pos], out, .{});
}

fn kdfTag(out: []u8, s: []const u8, tag: []const u8) void {
    var buf: [8 + 64 + 1024]u8 = undefined;
    var pos = encapInto(buf[0..], tag);
    @memcpy(buf[pos..][0..s.len], s);
    pos += s.len;
    Shake256.hash(buf[0..pos], out, .{});
}

fn aes256Ctr(key: []const u8, input: []const u8, out: []u8) void {
    const k = key[0..32].*;
    const ctx = Aes256.initEnc(k);
    const iv = [_]u8{0} ** 16;
    modes.ctr(@TypeOf(ctx), ctx, out, input, iv, .big);
}

fn validateExtensions(msg: []const u8) i32 {
    if (msg.len == 0) return STATUS_OK;
    const n = msg[0];
    var pos: usize = 1;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        if (pos + 2 > msg.len) return STATUS_INVALID;
        const l = msg[pos + 1];
        pos += 2;
        if (pos + l > msg.len) return STATUS_INVALID;
        pos += l;
    }
    if (pos != msg.len) return STATUS_INVALID;
    return STATUS_OK;
}

export fn proto_standard_id() u32 {
    return 300209;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_ntor_v3_protoid_len() u32 {
    return PROTOID.len;
}

export fn tor_sha3_256(data: [*]const u8, len: u32, out32: [*]u8) i32 {
    var digest: [32]u8 = undefined;
    Sha3_256.hash(slice(data, len), &digest, .{});
    copy(out32, &digest);
    return STATUS_OK;
}

export fn tor_shake256(data: [*]const u8, len: u32, out: [*]u8, out_len: u32) i32 {
    if (out_len > MAX_KEYSTREAM) return STATUS_BOUNDS;
    Shake256.hash(slice(data, len), out[0..out_len], .{});
    return STATUS_OK;
}

export fn tor_aes256_ctr(key32: [*]const u8, input: [*]const u8, len: u32, out: [*]u8) i32 {
    if (len > MAX_MSG) return STATUS_BOUNDS;
    aes256Ctr(key32[0..32], input[0..len], out[0..len]);
    return STATUS_OK;
}

export fn tor_ntor_v3_extensions_validate(msg: [*]const u8, len: u32) i32 {
    if (len > MAX_MSG) return STATUS_BOUNDS;
    return validateExtensions(msg[0..len]);
}

export fn tor_ntor_v3_server_handshake_seeded(
    handshake: [*]const u8,
    handshake_len: u32,
    node_id32: [*]const u8,
    onion_pub32: [*]const u8,
    onion_secret32: [*]const u8,
    y_secret32: [*]const u8,
    ver: [*]const u8,
    ver_len: u32,
    server_msg: [*]const u8,
    server_msg_len: u32,
    out_reply: [*]u8,
    out_reply_len: [*]u32,
    out_keystream: [*]u8,
    out_keystream_len: u32,
    out_client_msg: [*]u8,
    out_client_msg_len: [*]u32,
) i32 {
    if (handshake_len < 128) return STATUS_INVALID;
    if (handshake_len > 128 + MAX_MSG) return STATUS_BOUNDS;
    if (ver_len > MAX_VER or server_msg_len > MAX_MSG or out_keystream_len > MAX_KEYSTREAM) return STATUS_BOUNDS;
    if (!ctEq(handshake[0..32], node_id32[0..32])) return STATUS_INVALID;
    if (!ctEq(handshake[32..64], onion_pub32[0..32])) return STATUS_INVALID;

    const client_x = handshake[64..96];
    const encrypted_client_msg = handshake[96 .. handshake_len - 32];
    const client_mac = handshake[handshake_len - 32 .. handshake_len];

    var xb: [32]u8 = undefined;
    const sk_b = onion_secret32[0..32].*;
    const pk_x = client_x[0..32].*;
    xb = X25519.scalarmult(sk_b, pk_x) catch return STATUS_WEAK_KEY;
    if (allZero(&xb)) return STATUS_WEAK_KEY;

    var secret_phase1: [32 + 32 + 32 + 32 + PROTOID.len + 8 + MAX_VER]u8 = undefined;
    var pos: usize = 0;
    @memcpy(secret_phase1[pos..][0..32], &xb); pos += 32;
    @memcpy(secret_phase1[pos..][0..32], node_id32[0..32]); pos += 32;
    @memcpy(secret_phase1[pos..][0..32], client_x); pos += 32;
    @memcpy(secret_phase1[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(secret_phase1[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    pos += encapInto(secret_phase1[pos..], ver[0..ver_len]);

    var phase1_keys: [64]u8 = undefined;
    kdfTag(&phase1_keys, secret_phase1[0..pos], T_MSGKDF);
    const enc_k1 = phase1_keys[0..32];
    const mac_k1 = phase1_keys[32..64];

    var mac_input: [32 + 32 + 32 + MAX_MSG]u8 = undefined;
    var mi: usize = 0;
    @memcpy(mac_input[mi..][0..32], node_id32[0..32]); mi += 32;
    @memcpy(mac_input[mi..][0..32], onion_pub32[0..32]); mi += 32;
    @memcpy(mac_input[mi..][0..32], client_x); mi += 32;
    @memcpy(mac_input[mi..][0..encrypted_client_msg.len], encrypted_client_msg); mi += encrypted_client_msg.len;
    var expected_mac: [32]u8 = undefined;
    macTag(&expected_mac, mac_k1, mac_input[0..mi], T_MSGMAC);
    if (!ctEq(&expected_mac, client_mac)) return STATUS_INVALID;

    var client_msg: [MAX_MSG]u8 = undefined;
    aes256Ctr(enc_k1, encrypted_client_msg, client_msg[0..encrypted_client_msg.len]);
    if (validateExtensions(client_msg[0..encrypted_client_msg.len]) != STATUS_OK) return STATUS_INVALID;
    copy(out_client_msg, client_msg[0..encrypted_client_msg.len]);
    out_client_msg_len[0] = @intCast(encrypted_client_msg.len);

    var y_pub: [32]u8 = undefined;
    const y_sk = y_secret32[0..32].*;
    y_pub = X25519.recoverPublicKey(y_sk) catch return STATUS_WEAK_KEY;
    var xy: [32]u8 = undefined;
    xy = X25519.scalarmult(y_sk, pk_x) catch return STATUS_WEAK_KEY;
    if (allZero(&xy)) return STATUS_WEAK_KEY;

    var secret_input: [32 + 32 + 32 + 32 + 32 + 32 + PROTOID.len + 8 + MAX_VER]u8 = undefined;
    pos = 0;
    @memcpy(secret_input[pos..][0..32], &xy); pos += 32;
    @memcpy(secret_input[pos..][0..32], &xb); pos += 32;
    @memcpy(secret_input[pos..][0..32], node_id32[0..32]); pos += 32;
    @memcpy(secret_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(secret_input[pos..][0..32], client_x); pos += 32;
    @memcpy(secret_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(secret_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    pos += encapInto(secret_input[pos..], ver[0..ver_len]);

    var ntor_key_seed: [32]u8 = undefined;
    var verify: [32]u8 = undefined;
    hashTag(&ntor_key_seed, secret_input[0..pos], T_KEY_SEED);
    hashTag(&verify, secret_input[0..pos], T_VERIFY);

    var raw: [32 + MAX_KEYSTREAM]u8 = undefined;
    kdfTag(raw[0 .. 32 + out_keystream_len], &ntor_key_seed, T_FINAL);
    const enc_key = raw[0..32];
    copy(out_keystream, raw[32 .. 32 + out_keystream_len]);

    var encrypted_server_msg: [MAX_MSG]u8 = undefined;
    aes256Ctr(enc_key, server_msg[0..server_msg_len], encrypted_server_msg[0..server_msg_len]);

    var auth_input: [32 + 32 + 32 + 32 + 32 + 32 + 8 + MAX_MSG + PROTOID.len + SERVER.len]u8 = undefined;
    pos = 0;
    @memcpy(auth_input[pos..][0..32], &verify); pos += 32;
    @memcpy(auth_input[pos..][0..32], node_id32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(auth_input[pos..][0..32], client_x); pos += 32;
    @memcpy(auth_input[pos..][0..32], client_mac); pos += 32;
    pos += encapInto(auth_input[pos..], encrypted_server_msg[0..server_msg_len]);
    @memcpy(auth_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(auth_input[pos..][0..SERVER.len], SERVER); pos += SERVER.len;
    var auth: [32]u8 = undefined;
    hashTag(&auth, auth_input[0..pos], T_AUTH);

    copy(out_reply, &y_pub);
    copy(out_reply + 32, &auth);
    copy(out_reply + 64, encrypted_server_msg[0..server_msg_len]);
    out_reply_len[0] = @intCast(64 + server_msg_len);
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_ntor_v3_crypto.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-ntor-v3-crypto.wasm"

wasm2wat "$WORK_DIR/tor-ntor-v3-crypto.wasm" -o "$OUT_DIR/tor-ntor-v3-crypto.wat"
wat2wasm "$OUT_DIR/tor-ntor-v3-crypto.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
