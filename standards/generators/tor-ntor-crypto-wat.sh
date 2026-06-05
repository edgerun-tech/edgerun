#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-ntor-crypto"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-ntor-crypto.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_ntor_crypto.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;

const X25519 = crypto.dh.X25519;
const HmacSha256 = crypto.auth.hmac.sha2.HmacSha256;
const Sha256 = crypto.hash.sha2.Sha256;

const PROTOID = "ntor-curve25519-sha256-1";
const T_MAC = PROTOID ++ ":mac";
const T_KEY = PROTOID ++ ":key_extract";
const T_VERIFY = PROTOID ++ ":verify";
const M_EXPAND = PROTOID ++ ":key_expand";

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_WEAK_KEY: i32 = -2;

fn slice(ptr: [*]const u8, len: usize) []const u8 {
    return ptr[0..len];
}

fn mut(ptr: [*]u8, len: usize) []u8 {
    return ptr[0..len];
}

fn copy(dst: [*]u8, src: []const u8) void {
    @memcpy(dst[0..src.len], src);
}

fn eq32(a: []const u8, b: []const u8) bool {
    var acc: u8 = 0;
    var i: usize = 0;
    while (i < 32) : (i += 1) acc |= a[i] ^ b[i];
    return acc == 0;
}

fn allZero(bytes: []const u8) bool {
    var acc: u8 = 0;
    for (bytes) |b| acc |= b;
    return acc == 0;
}

fn hmac(out: [*]u8, key: []const u8, msg: []const u8) void {
    var mac: [32]u8 = undefined;
    HmacSha256.create(&mac, msg, key);
    copy(out, &mac);
}

fn hkdfExpand(out: [*]u8, out_len: usize, prk: []const u8, info: []const u8) void {
    var t: [32]u8 = undefined;
    var generated: usize = 0;
    var counter: u8 = 1;
    var prev_len: usize = 0;
    while (generated < out_len) : (counter += 1) {
        var ctx = HmacSha256.init(prk);
        if (prev_len != 0) ctx.update(t[0..prev_len]);
        ctx.update(info);
        ctx.update((&counter)[0..1]);
        ctx.final(&t);
        prev_len = 32;
        const take = @min(32, out_len - generated);
        @memcpy(out[generated..][0..take], t[0..take]);
        generated += take;
    }
}

export fn proto_standard_id() u32 {
    return 300206;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_sha256(data: [*]const u8, len: u32, out32: [*]u8) i32 {
    var digest: [32]u8 = undefined;
    Sha256.hash(slice(data, len), &digest, .{});
    copy(out32, &digest);
    return STATUS_OK;
}

export fn tor_hmac_sha256(key: [*]const u8, key_len: u32, msg: [*]const u8, msg_len: u32, out32: [*]u8) i32 {
    hmac(out32, slice(key, key_len), slice(msg, msg_len));
    return STATUS_OK;
}

export fn tor_x25519_public(secret32: [*]const u8, out32: [*]u8) i32 {
    const sk = secret32[0..32].*;
    const pk = X25519.recoverPublicKey(sk) catch return STATUS_WEAK_KEY;
    copy(out32, &pk);
    return STATUS_OK;
}

export fn tor_x25519_shared(secret32: [*]const u8, public32: [*]const u8, out32: [*]u8) i32 {
    const sk = secret32[0..32].*;
    const pk = public32[0..32].*;
    const shared = X25519.scalarmult(sk, pk) catch return STATUS_WEAK_KEY;
    if (allZero(&shared)) return STATUS_WEAK_KEY;
    copy(out32, &shared);
    return STATUS_OK;
}

// ntor v2 server side. The caller supplies the relay identity digest, onion
// public/secret keypair, and an ephemeral y secret. This keeps entropy outside
// the primitive while all cryptographic computation remains in WAT.
export fn tor_ntor_server_handshake_seeded(
    handshake84: [*]const u8,
    node_id20: [*]const u8,
    onion_pub32: [*]const u8,
    onion_secret32: [*]const u8,
    y_secret32: [*]const u8,
    out_reply64: [*]u8,
    out_key_material92: [*]u8,
) i32 {
    if (!std.mem.eql(u8, handshake84[0..20], node_id20[0..20])) return STATUS_INVALID;
    if (!std.mem.eql(u8, handshake84[20..52], onion_pub32[0..32])) return STATUS_INVALID;

    const client_x = handshake84[52..84];

    var exp_xy: [32]u8 = undefined;
    var exp_xb: [32]u8 = undefined;
    var y_pub: [32]u8 = undefined;
    if (tor_x25519_shared(y_secret32, client_x.ptr, &exp_xy) != STATUS_OK) return STATUS_WEAK_KEY;
    if (tor_x25519_shared(onion_secret32, client_x.ptr, &exp_xb) != STATUS_OK) return STATUS_WEAK_KEY;
    if (tor_x25519_public(y_secret32, &y_pub) != STATUS_OK) return STATUS_WEAK_KEY;

    var secret_input: [204]u8 = undefined;
    var pos: usize = 0;
    @memcpy(secret_input[pos..][0..32], &exp_xy); pos += 32;
    @memcpy(secret_input[pos..][0..32], &exp_xb); pos += 32;
    @memcpy(secret_input[pos..][0..20], node_id20[0..20]); pos += 20;
    @memcpy(secret_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(secret_input[pos..][0..32], client_x); pos += 32;
    @memcpy(secret_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(secret_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    if (pos != secret_input.len) return STATUS_INVALID;

    var key_seed: [32]u8 = undefined;
    var verify: [32]u8 = undefined;
    HmacSha256.create(&key_seed, &secret_input, T_KEY);
    HmacSha256.create(&verify, &secret_input, T_VERIFY);

    var auth_input: [178]u8 = undefined;
    pos = 0;
    @memcpy(auth_input[pos..][0..32], &verify); pos += 32;
    @memcpy(auth_input[pos..][0..20], node_id20[0..20]); pos += 20;
    @memcpy(auth_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(auth_input[pos..][0..32], client_x); pos += 32;
    @memcpy(auth_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(auth_input[pos..][0.."Server".len], "Server"); pos += "Server".len;
    if (pos != auth_input.len) return STATUS_INVALID;

    var auth: [32]u8 = undefined;
    HmacSha256.create(&auth, &auth_input, T_MAC);
    copy(out_reply64, &y_pub);
    copy(out_reply64 + 32, &auth);

    hkdfExpand(out_key_material92, 92, &key_seed, M_EXPAND);
    return STATUS_OK;
}

export fn tor_ntor_client_handshake_seeded(
    node_id20: [*]const u8,
    onion_pub32: [*]const u8,
    x_secret32: [*]const u8,
    out_handshake84: [*]u8,
    out_x_public32: [*]u8,
) i32 {
    if (tor_x25519_public(x_secret32, out_x_public32) != STATUS_OK) return STATUS_WEAK_KEY;
    copy(out_handshake84, node_id20[0..20]);
    copy(out_handshake84 + 20, onion_pub32[0..32]);
    copy(out_handshake84 + 52, out_x_public32[0..32]);
    return STATUS_OK;
}

export fn tor_ntor_client_process(
    reply64: [*]const u8,
    node_id20: [*]const u8,
    onion_pub32: [*]const u8,
    x_secret32: [*]const u8,
    x_public32: [*]const u8,
    out_key_material92: [*]u8,
) i32 {
    const server_y = reply64[0..32];
    const server_auth = reply64[32..64];

    var exp_xy: [32]u8 = undefined;
    var exp_xb: [32]u8 = undefined;
    if (tor_x25519_shared(x_secret32, server_y.ptr, &exp_xy) != STATUS_OK) return STATUS_WEAK_KEY;
    if (tor_x25519_shared(x_secret32, onion_pub32, &exp_xb) != STATUS_OK) return STATUS_WEAK_KEY;

    var secret_input: [204]u8 = undefined;
    var pos: usize = 0;
    @memcpy(secret_input[pos..][0..32], &exp_xy); pos += 32;
    @memcpy(secret_input[pos..][0..32], &exp_xb); pos += 32;
    @memcpy(secret_input[pos..][0..20], node_id20[0..20]); pos += 20;
    @memcpy(secret_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(secret_input[pos..][0..32], x_public32[0..32]); pos += 32;
    @memcpy(secret_input[pos..][0..32], server_y); pos += 32;
    @memcpy(secret_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    if (pos != secret_input.len) return STATUS_INVALID;

    var key_seed: [32]u8 = undefined;
    var verify: [32]u8 = undefined;
    HmacSha256.create(&key_seed, &secret_input, T_KEY);
    HmacSha256.create(&verify, &secret_input, T_VERIFY);

    var auth_input: [178]u8 = undefined;
    pos = 0;
    @memcpy(auth_input[pos..][0..32], &verify); pos += 32;
    @memcpy(auth_input[pos..][0..20], node_id20[0..20]); pos += 20;
    @memcpy(auth_input[pos..][0..32], onion_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], server_y); pos += 32;
    @memcpy(auth_input[pos..][0..32], x_public32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(auth_input[pos..][0.."Server".len], "Server"); pos += "Server".len;
    if (pos != auth_input.len) return STATUS_INVALID;

    var expected_auth: [32]u8 = undefined;
    HmacSha256.create(&expected_auth, &auth_input, T_MAC);
    if (!std.mem.eql(u8, &expected_auth, server_auth)) return STATUS_INVALID;

    hkdfExpand(out_key_material92, 92, &key_seed, M_EXPAND);
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_ntor_crypto.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-ntor-crypto.wasm"

wasm2wat "$WORK_DIR/tor-ntor-crypto.wasm" -o "$OUT_DIR/tor-ntor-crypto.wat"
wat2wasm "$OUT_DIR/tor-ntor-crypto.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
