#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-ntor"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-ntor.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_hs_ntor.zig" <<'ZIG'
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

const PROTOID = "tor-hs-ntor-curve25519-sha3-256-1";
const T_HSENC = "tor-hs-ntor-curve25519-sha3-256-1:hs_key_extract";
const T_HSVERIFY = "tor-hs-ntor-curve25519-sha3-256-1:hs_verify";
const T_HSMAC = "tor-hs-ntor-curve25519-sha3-256-1:hs_mac";
const M_HSEXPAND = "tor-hs-ntor-curve25519-sha3-256-1:hs_key_expand";
const SERVER = "Server";

const KEY_LEN: usize = 32;
const COOKIE_LEN: usize = 20;
const MAC_LEN: usize = 32;
const MAX_BODY: usize = 490;
const MAX_PLAIN: usize = 426;
const INTRO_PREFIX_LEN: usize = 56;
const INTRO_AUTH_KEY_LEN: usize = 32;
const INTRO_ENCRYPTED_OVERHEAD: usize = 64;
const INTRO_PLAINTEXT_BASE_LEN: usize = 57;

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

fn aes256Ctr(key32: []const u8, input: []const u8, out: []u8) void {
    const key = key32[0..32].*;
    const iv = [_]u8{0} ** 16;
    const ctx = Aes256.initEnc(key);
    modes.ctr(@TypeOf(ctx), ctx, out, input, iv, .big);
}

fn macVar(key: []const u8, msg: []const u8, out32: []u8) void {
    var h = Sha3_256.init(.{});
    var encap: [8]u8 = undefined;
    put64be(&encap, key.len);
    h.update(&encap);
    h.update(key);
    h.update(msg);
    var digest: [32]u8 = undefined;
    h.final(&digest);
    @memcpy(out32[0..32], &digest);
}

fn deriveIntroKeys(exp32: []const u8, auth_key32: []const u8, client_pub32: []const u8, service_pub32: []const u8, subcred32: []const u8, out64: []u8) void {
    var input: [32 + 32 + 32 + 32 + PROTOID.len + T_HSENC.len + M_HSEXPAND.len + 32]u8 = undefined;
    var pos: usize = 0;
    @memcpy(input[pos..][0..32], exp32[0..32]); pos += 32;
    @memcpy(input[pos..][0..32], auth_key32[0..32]); pos += 32;
    @memcpy(input[pos..][0..32], client_pub32[0..32]); pos += 32;
    @memcpy(input[pos..][0..32], service_pub32[0..32]); pos += 32;
    @memcpy(input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(input[pos..][0..T_HSENC.len], T_HSENC); pos += T_HSENC.len;
    @memcpy(input[pos..][0..M_HSEXPAND.len], M_HSEXPAND); pos += M_HSEXPAND.len;
    @memcpy(input[pos..][0..32], subcred32[0..32]); pos += 32;
    Shake256.hash(input[0..pos], out64[0..64], .{});
}

fn deriveIntroKeysClient(client_secret32: []const u8, auth_key32: []const u8, service_pub32: []const u8, subcred32: []const u8, client_pub_out: ?[]u8, out64: []u8) i32 {
    const sk = client_secret32[0..32].*;
    const service_pub = service_pub32[0..32].*;
    const client_pub = X25519.recoverPublicKey(sk) catch return STATUS_WEAK_KEY;
    const shared = X25519.scalarmult(sk, service_pub) catch return STATUS_WEAK_KEY;
    if (client_pub_out) |out| @memcpy(out[0..32], &client_pub);
    deriveIntroKeys(&shared, auth_key32, &client_pub, service_pub32, subcred32, out64);
    return STATUS_OK;
}

fn deriveIntroKeysService(service_secret32: []const u8, auth_key32: []const u8, service_pub32: []const u8, client_pub32: []const u8, subcred32: []const u8, out64: []u8) i32 {
    const sk = service_secret32[0..32].*;
    const client_pub = client_pub32[0..32].*;
    const shared = X25519.scalarmult(sk, client_pub) catch return STATUS_WEAK_KEY;
    deriveIntroKeys(&shared, auth_key32, client_pub32, service_pub32, subcred32, out64);
    return STATUS_OK;
}

fn introMacInput(auth_key32: []const u8, client_pub32: []const u8, encrypted_data: []const u8, out: []u8) []u8 {
    @memset(out[0..20], 0);
    out[20] = 2;
    out[21] = 0;
    out[22] = 32;
    @memcpy(out[23..55], auth_key32[0..32]);
    out[55] = 0;
    @memcpy(out[56..88], client_pub32[0..32]);
    @memcpy(out[88..][0..encrypted_data.len], encrypted_data);
    return out[0 .. 88 + encrypted_data.len];
}

fn serviceRendezvous(client_pub32: []const u8, service_secret32: []const u8, service_pub32: []const u8, auth_key32: []const u8, y_secret32: []const u8, out_handshake64: []u8, out_seed32: []u8) i32 {
    const x = client_pub32[0..32].*;
    const b = service_secret32[0..32].*;
    const y = y_secret32[0..32].*;
    const xb = X25519.scalarmult(b, x) catch return STATUS_WEAK_KEY;
    const xy = X25519.scalarmult(y, x) catch return STATUS_WEAK_KEY;
    const y_pub = X25519.recoverPublicKey(y) catch return STATUS_WEAK_KEY;

    var rend_input: [32 + 32 + 32 + 32 + 32 + 32 + PROTOID.len]u8 = undefined;
    var pos: usize = 0;
    @memcpy(rend_input[pos..][0..32], &xy); pos += 32;
    @memcpy(rend_input[pos..][0..32], &xb); pos += 32;
    @memcpy(rend_input[pos..][0..32], auth_key32[0..32]); pos += 32;
    @memcpy(rend_input[pos..][0..32], service_pub32[0..32]); pos += 32;
    @memcpy(rend_input[pos..][0..32], client_pub32[0..32]); pos += 32;
    @memcpy(rend_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(rend_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    const rin = rend_input[0..pos];

    macVar(rin, T_HSENC, out_seed32[0..32]);
    var verify: [32]u8 = undefined;
    macVar(rin, T_HSVERIFY, &verify);

    var auth_input: [32 + 32 + 32 + 32 + 32 + PROTOID.len + SERVER.len]u8 = undefined;
    pos = 0;
    @memcpy(auth_input[pos..][0..32], &verify); pos += 32;
    @memcpy(auth_input[pos..][0..32], auth_key32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], service_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(auth_input[pos..][0..32], client_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(auth_input[pos..][0..SERVER.len], SERVER); pos += SERVER.len;

    @memcpy(out_handshake64[0..32], &y_pub);
    macVar(auth_input[0..pos], T_HSMAC, out_handshake64[32..64]);
    return STATUS_OK;
}

export fn proto_standard_id() u32 {
    return 300214;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_hs_ntor_intro_prefix_len() u32 {
    return INTRO_PREFIX_LEN;
}

export fn tor_hs_ntor_intro_plaintext_base_len() u32 {
    return INTRO_PLAINTEXT_BASE_LEN;
}

export fn tor_hs_ntor_max_body_len() u32 {
    return MAX_BODY;
}

export fn tor_hs_ntor_derive_intro_keys_client(client_secret32: [*]const u8, auth_key32: [*]const u8, service_pub32: [*]const u8, subcred32: [*]const u8, out_client_pub32: [*]u8, out64: [*]u8) i32 {
    return deriveIntroKeysClient(client_secret32[0..32], auth_key32[0..32], service_pub32[0..32], subcred32[0..32], out_client_pub32[0..32], out64[0..64]);
}

export fn tor_hs_ntor_derive_intro_keys_service(service_secret32: [*]const u8, auth_key32: [*]const u8, service_pub32: [*]const u8, client_pub32: [*]const u8, subcred32: [*]const u8, out64: [*]u8) i32 {
    return deriveIntroKeysService(service_secret32[0..32], auth_key32[0..32], service_pub32[0..32], client_pub32[0..32], subcred32[0..32], out64[0..64]);
}

export fn tor_hs_ntor_build_intro_plain(out: [*]u8, cookie20: [*]const u8, onion_key32: [*]const u8, nspec: u32, linkspecs: [*]const u8, linkspecs_len: u32, pad_len: u32) i32 {
    if (nspec > 255) return STATUS_INVALID;
    const total = INTRO_PLAINTEXT_BASE_LEN + linkspecs_len + pad_len;
    if (total > MAX_PLAIN) return STATUS_BOUNDS;
    @memcpy(out[0..20], cookie20[0..20]);
    out[20] = 0;
    out[21] = 1;
    out[22] = 0;
    out[23] = 32;
    @memcpy(out[24..56], onion_key32[0..32]);
    out[56] = @intCast(nspec);
    if (linkspecs_len != 0) @memcpy(out[57..][0..linkspecs_len], linkspecs[0..linkspecs_len]);
    if (pad_len != 0) @memset(out[57 + linkspecs_len ..][0..pad_len], 0);
    return @intCast(total);
}

export fn tor_hs_ntor_parse_intro_plain(plain: [*]const u8, plain_len: u32, out_cookie20: [*]u8, out_onion_key32: [*]u8, out_linkspecs: [*]u8, linkspecs_cap: u32, out_linkspecs_len: [*]u32) i32 {
    if (plain_len < INTRO_PLAINTEXT_BASE_LEN) return STATUS_INVALID;
    if (plain[20] != 0 or plain[21] != 1 or plain[22] != 0 or plain[23] != 32) return STATUS_INVALID;
    const nspec = plain[56];
    if (nspec == 0) return STATUS_INVALID;
    const rem = plain_len - INTRO_PLAINTEXT_BASE_LEN;
    if (rem > linkspecs_cap) return STATUS_BOUNDS;
    @memcpy(out_cookie20[0..20], plain[0..20]);
    @memcpy(out_onion_key32[0..32], plain[24..56]);
    out_linkspecs_len[0] = rem;
    if (rem != 0) @memcpy(out_linkspecs[0..rem], plain[57..][0..rem]);
    return STATUS_OK;
}

export fn tor_hs_ntor_build_introduce1_prefix(out: [*]u8, auth_key32: [*]const u8, encrypted_field: [*]const u8, encrypted_len: u32) i32 {
    if (encrypted_len > MAX_BODY - INTRO_PREFIX_LEN) return STATUS_BOUNDS;
    @memset(out[0..20], 0);
    out[20] = 2;
    out[21] = 0;
    out[22] = 32;
    @memcpy(out[23..55], auth_key32[0..32]);
    out[55] = 0;
    if (encrypted_len != 0) @memcpy(out[56..][0..encrypted_len], encrypted_field[0..encrypted_len]);
    return @intCast(INTRO_PREFIX_LEN + encrypted_len);
}

export fn tor_hs_ntor_build_introduce1_encrypted(out: [*]u8, auth_key32: [*]const u8, service_pub32: [*]const u8, client_secret32: [*]const u8, subcred32: [*]const u8, plaintext: [*]const u8, plaintext_len: u32) i32 {
    if (plaintext_len > MAX_PLAIN) return STATUS_BOUNDS;
    var keys: [64]u8 = undefined;
    var client_pub: [32]u8 = undefined;
    const rc = deriveIntroKeysClient(client_secret32[0..32], auth_key32[0..32], service_pub32[0..32], subcred32[0..32], &client_pub, &keys);
    if (rc != STATUS_OK) return rc;
    @memcpy(out[0..32], &client_pub);
    aes256Ctr(keys[0..32], plaintext[0..plaintext_len], out[32..][0..plaintext_len]);
    var mac_input: [INTRO_PREFIX_LEN + 32 + MAX_PLAIN]u8 = undefined;
    const msg = introMacInput(auth_key32[0..32], &client_pub, out[32..][0..plaintext_len], &mac_input);
    macVar(keys[32..64], msg, out[32 + plaintext_len ..][0..32]);
    return @intCast(INTRO_ENCRYPTED_OVERHEAD + plaintext_len);
}

export fn tor_hs_ntor_open_introduce2(out_plain: [*]u8, out_plain_len: [*]u32, auth_key32: [*]const u8, service_secret32: [*]const u8, service_pub32: [*]const u8, subcred32: [*]const u8, encrypted_field: [*]const u8, encrypted_len: u32) i32 {
    if (encrypted_len < INTRO_ENCRYPTED_OVERHEAD or encrypted_len > MAX_BODY - INTRO_PREFIX_LEN) return STATUS_INVALID;
    const data_len = encrypted_len - INTRO_ENCRYPTED_OVERHEAD;
    var keys: [64]u8 = undefined;
    const rc = deriveIntroKeysService(service_secret32[0..32], auth_key32[0..32], service_pub32[0..32], encrypted_field[0..32], subcred32[0..32], &keys);
    if (rc != STATUS_OK) return rc;
    var mac_input: [INTRO_PREFIX_LEN + 32 + MAX_PLAIN]u8 = undefined;
    const msg = introMacInput(auth_key32[0..32], encrypted_field[0..32], encrypted_field[32..][0..data_len], &mac_input);
    var expected: [32]u8 = undefined;
    macVar(keys[32..64], msg, &expected);
    if (!ctEq(&expected, encrypted_field[32 + data_len ..][0..32])) return STATUS_AUTH;
    if (data_len != 0) aes256Ctr(keys[0..32], encrypted_field[32..][0..data_len], out_plain[0..data_len]);
    out_plain_len[0] = data_len;
    return STATUS_OK;
}

export fn tor_hs_ntor_service_rendezvous(encrypted_field: [*]const u8, encrypted_len: u32, auth_key32: [*]const u8, service_secret32: [*]const u8, service_pub32: [*]const u8, y_secret32: [*]const u8, out_handshake64: [*]u8, out_seed32: [*]u8) i32 {
    if (encrypted_len < INTRO_ENCRYPTED_OVERHEAD) return STATUS_INVALID;
    return serviceRendezvous(encrypted_field[0..32], service_secret32[0..32], service_pub32[0..32], auth_key32[0..32], y_secret32[0..32], out_handshake64[0..64], out_seed32[0..32]);
}

export fn tor_hs_ntor_client_verify_rendezvous(client_secret32: [*]const u8, auth_key32: [*]const u8, service_pub32: [*]const u8, handshake64: [*]const u8, out_seed32: [*]u8) i32 {
    const x = client_secret32[0..32].*;
    const b_pub = service_pub32[0..32].*;
    const y_pub = handshake64[0..32].*;
    const client_pub = X25519.recoverPublicKey(x) catch return STATUS_WEAK_KEY;
    const xb = X25519.scalarmult(x, b_pub) catch return STATUS_WEAK_KEY;
    const xy = X25519.scalarmult(x, y_pub) catch return STATUS_WEAK_KEY;

    var rend_input: [32 + 32 + 32 + 32 + 32 + 32 + PROTOID.len]u8 = undefined;
    var pos: usize = 0;
    @memcpy(rend_input[pos..][0..32], &xy); pos += 32;
    @memcpy(rend_input[pos..][0..32], &xb); pos += 32;
    @memcpy(rend_input[pos..][0..32], auth_key32[0..32]); pos += 32;
    @memcpy(rend_input[pos..][0..32], service_pub32[0..32]); pos += 32;
    @memcpy(rend_input[pos..][0..32], &client_pub); pos += 32;
    @memcpy(rend_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(rend_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    const rin = rend_input[0..pos];

    macVar(rin, T_HSENC, out_seed32[0..32]);
    var verify: [32]u8 = undefined;
    macVar(rin, T_HSVERIFY, &verify);

    var auth_input: [32 + 32 + 32 + 32 + 32 + PROTOID.len + SERVER.len]u8 = undefined;
    pos = 0;
    @memcpy(auth_input[pos..][0..32], &verify); pos += 32;
    @memcpy(auth_input[pos..][0..32], auth_key32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], service_pub32[0..32]); pos += 32;
    @memcpy(auth_input[pos..][0..32], &y_pub); pos += 32;
    @memcpy(auth_input[pos..][0..32], &client_pub); pos += 32;
    @memcpy(auth_input[pos..][0..PROTOID.len], PROTOID); pos += PROTOID.len;
    @memcpy(auth_input[pos..][0..SERVER.len], SERVER); pos += SERVER.len;
    var expected: [32]u8 = undefined;
    macVar(auth_input[0..pos], T_HSMAC, &expected);
    if (!ctEq(&expected, handshake64[32..64])) return STATUS_AUTH;
    return STATUS_OK;
}

export fn tor_hs_ntor_build_rendezvous1(out: [*]u8, cookie20: [*]const u8, handshake64: [*]const u8) i32 {
    @memcpy(out[0..20], cookie20[0..20]);
    @memcpy(out[20..84], handshake64[0..64]);
    return 84;
}
ZIG

zig build-exe "$WORK_DIR/tor_hs_ntor.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-hs-ntor.wasm"

wasm2wat "$WORK_DIR/tor-hs-ntor.wasm" -o "$OUT_DIR/tor-hs-ntor.wat"
wat2wasm "$OUT_DIR/tor-hs-ntor.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
