#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-channel-handshake"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-channel-handshake.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_channel_handshake.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha256 = crypto.hash.sha2.Sha256;
const Ed25519 = crypto.sign.Ed25519;

const STATUS_OK: i32 = 0;
const STATUS_NOT_FOUND: i32 = 1;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;
const CMD_VERSIONS: u8 = 7;
const CMD_NETINFO: u8 = 8;
const CMD_CERTS: u8 = 129;
const CMD_AUTH_CHALLENGE: u8 = 130;
const CMD_AUTHENTICATE: u8 = 131;
const CERT_IDENTITY_V_SIGNING: u8 = 4;
const CERT_SIGNING_V_TLS: u8 = 5;
const CERT_SIGNING_V_LINK_AUTH: u8 = 6;
const CERT_KEY_ED25519: u8 = 1;
const CERT_KEY_SHA256_DER: u8 = 3;
const CERT_EXT_SIGNED_WITH_ED25519: u8 = 4;
const AUTH_ED25519_SHA256_RFC5705: u16 = 3;
const AUTH0003_LEN: usize = 296;

fn get16be(s: []const u8) u16 {
    return (@as(u16, s[0]) << 8) | @as(u16, s[1]);
}

fn get32be(s: []const u8) u32 {
    return (@as(u32, s[0]) << 24) | (@as(u32, s[1]) << 16) | (@as(u32, s[2]) << 8) | @as(u32, s[3]);
}

fn put16be(out: []u8, v: u16) void {
    out[0] = @intCast((v >> 8) & 0xff);
    out[1] = @intCast(v & 0xff);
}

fn put32be(out: []u8, v: u32) void {
    out[0] = @intCast((v >> 24) & 0xff);
    out[1] = @intCast((v >> 16) & 0xff);
    out[2] = @intCast((v >> 8) & 0xff);
    out[3] = @intCast(v & 0xff);
}

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn sha256(data: []const u8, out32: []u8) void {
    var digest: [32]u8 = undefined;
    Sha256.hash(data, &digest, .{});
    @memcpy(out32[0..32], &digest);
}

fn certBodyLen(cert: []const u8) i32 {
    if (cert.len < 40 + 64) return STATUS_BOUNDS;
    if (cert[0] != 1) return STATUS_UNSUPPORTED;
    var pos: usize = 40;
    var n: usize = cert[39];
    while (n > 0) : (n -= 1) {
        if (pos + 4 > cert.len) return STATUS_BOUNDS;
        const ext_len: usize = get16be(cert[pos..][0..2]);
        pos += 4;
        if (pos + ext_len > cert.len) return STATUS_BOUNDS;
        pos += ext_len;
    }
    if (pos + 64 > cert.len) return STATUS_BOUNDS;
    return @intCast(pos);
}

fn signedWithKey(cert: []const u8, out32: []u8) i32 {
    const body_len_i = certBodyLen(cert);
    if (body_len_i < 0) return body_len_i;
    const body_len: usize = @intCast(body_len_i);
    var pos: usize = 40;
    var n: usize = cert[39];
    while (n > 0) : (n -= 1) {
        const ext_len: usize = get16be(cert[pos..][0..2]);
        const typ = cert[pos + 2];
        const flags = cert[pos + 3];
        pos += 4;
        if (typ == CERT_EXT_SIGNED_WITH_ED25519) {
            if (ext_len != 32) return STATUS_INVALID;
            @memcpy(out32[0..32], cert[pos..][0..32]);
            return STATUS_OK;
        }
        if ((flags & 1) != 0) return STATUS_UNSUPPORTED;
        pos += ext_len;
        if (pos > body_len) return STATUS_BOUNDS;
    }
    return STATUS_NOT_FOUND;
}

fn verifyCert(cert: []const u8, expected_type: u8, expected_key_type: u8, signer32: []const u8, now_hour: u32, out_subject32: []u8) i32 {
    const body_len_i = certBodyLen(cert);
    if (body_len_i < 0) return body_len_i;
    const body_len: usize = @intCast(body_len_i);
    if (cert.len < body_len + 64) return STATUS_BOUNDS;
    if (cert[1] != expected_type) return STATUS_INVALID;
    if (cert[6] != expected_key_type) return STATUS_INVALID;
    if (get32be(cert[2..6]) < now_hour) return STATUS_AUTH;
    @memcpy(out_subject32[0..32], cert[7..39]);
    const pk = Ed25519.PublicKey.fromBytes(signer32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.Signature.fromBytes(cert[body_len..][0..64].*);
    Ed25519.Signature.verify(sig, cert[0..body_len], pk) catch return STATUS_AUTH;
    return STATUS_OK;
}

fn findCert(certs: []const u8, typ: u8, out_view: []u32) i32 {
    if (certs.len < 1) return STATUS_BOUNDS;
    var seen: [256]u8 = [_]u8{0} ** 256;
    var pos: usize = 1;
    var n: usize = certs[0];
    while (n > 0) : (n -= 1) {
        if (pos + 3 > certs.len) return STATUS_BOUNDS;
        const rec_type = certs[pos];
        const len: usize = get16be(certs[pos + 1 ..][0..2]);
        pos += 3;
        if (seen[rec_type] != 0) return STATUS_INVALID;
        seen[rec_type] = 1;
        if (pos + len > certs.len) return STATUS_BOUNDS;
        if (rec_type == typ) {
            out_view[0] = rec_type;
            out_view[1] = @intCast(pos);
            out_view[2] = @intCast(len);
            out_view[3] = @intCast(pos + len);
            return STATUS_OK;
        }
        pos += len;
    }
    return STATUS_NOT_FOUND;
}

export fn proto_standard_id() u32 {
    return 300221;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_link_versions_negotiate(local: [*]const u8, local_len: u32, remote: [*]const u8, remote_len: u32) i32 {
    if ((local_len & 1) != 0 or (remote_len & 1) != 0) return STATUS_INVALID;
    var best: u16 = 0;
    var i: usize = 0;
    while (i < local_len) : (i += 2) {
        const lv = get16be(local[i..][0..2]);
        if (lv < 3 or lv > 5) continue;
        var j: usize = 0;
        while (j < remote_len) : (j += 2) {
            const rv = get16be(remote[j..][0..2]);
            if (lv == rv and lv > best) best = lv;
        }
    }
    if (best == 0) return STATUS_UNSUPPORTED;
    return best;
}

export fn tor_channel_ed25519_public(seed32: [*]const u8, out_pub32: [*]u8) i32 {
    const kp = Ed25519.KeyPair.generateDeterministic(seed32[0..32].*) catch return STATUS_AUTH;
    const public_bytes = kp.public_key.toBytes();
    @memcpy(out_pub32[0..32], public_bytes[0..32]);
    return STATUS_OK;
}

export fn tor_channel_ed25519_sign_seeded(msg: [*]const u8, msg_len: u32, seed32: [*]const u8, out_sig64: [*]u8, out_pub32: [*]u8) i32 {
    if (msg_len > 65536) return STATUS_BOUNDS;
    const kp = Ed25519.KeyPair.generateDeterministic(seed32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.KeyPair.sign(kp, msg[0..msg_len], null) catch return STATUS_AUTH;
    Ed25519.Signature.verify(sig, msg[0..msg_len], kp.public_key) catch return STATUS_AUTH;
    const sig_bytes = sig.toBytes();
    @memcpy(out_sig64[0..64], sig_bytes[0..64]);
    const public_bytes = kp.public_key.toBytes();
    @memcpy(out_pub32[0..32], public_bytes[0..32]);
    return STATUS_OK;
}

export fn tor_channel_ed25519_verify_raw(msg: [*]const u8, msg_len: u32, sig64: [*]const u8, pub32: [*]const u8) i32 {
    if (msg_len > 65536) return STATUS_BOUNDS;
    const pk = Ed25519.PublicKey.fromBytes(pub32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.Signature.fromBytes(sig64[0..64].*);
    Ed25519.Signature.verify(sig, msg[0..msg_len], pk) catch return STATUS_AUTH;
    return STATUS_OK;
}

export fn tor_certs_find(certs: [*]const u8, certs_len: u32, typ: u32, out_view_u32: [*]u32) i32 {
    if (typ > 255) return STATUS_INVALID;
    return findCert(certs[0..certs_len], @intCast(typ), out_view_u32[0..4]);
}

export fn tor_ed25519_cert_signed_with(cert: [*]const u8, cert_len: u32, out_signer32: [*]u8) i32 {
    return signedWithKey(cert[0..cert_len], out_signer32[0..32]);
}

export fn tor_ed25519_cert_verify(cert: [*]const u8, cert_len: u32, expected_type: u32, expected_key_type: u32, signer32: [*]const u8, now_hour: u32, out_subject32: [*]u8) i32 {
    if (expected_type > 255 or expected_key_type > 255) return STATUS_INVALID;
    return verifyCert(cert[0..cert_len], @intCast(expected_type), @intCast(expected_key_type), signer32[0..32], now_hour, out_subject32[0..32]);
}

export fn tor_certs_validate_responder(certs: [*]const u8, certs_len: u32, tls_cert_sha256_32: [*]const u8, now_hour: u32, out_relay_id32: [*]u8, out_signing_key32: [*]u8) i32 {
    var view: [4]u32 = undefined;
    var signer: [32]u8 = undefined;
    var subject: [32]u8 = undefined;
    var rc = findCert(certs[0..certs_len], CERT_IDENTITY_V_SIGNING, &view);
    if (rc != STATUS_OK) return rc;
    const id_cert = certs[view[1] .. view[1] + view[2]];
    rc = signedWithKey(id_cert, &signer);
    if (rc != STATUS_OK) return rc;
    rc = verifyCert(id_cert, CERT_IDENTITY_V_SIGNING, CERT_KEY_ED25519, &signer, now_hour, &subject);
    if (rc != STATUS_OK) return rc;
    @memcpy(out_relay_id32[0..32], &signer);
    @memcpy(out_signing_key32[0..32], &subject);

    rc = findCert(certs[0..certs_len], CERT_SIGNING_V_TLS, &view);
    if (rc != STATUS_OK) return rc;
    rc = verifyCert(certs[view[1] .. view[1] + view[2]], CERT_SIGNING_V_TLS, CERT_KEY_SHA256_DER, &subject, now_hour, &signer);
    if (rc != STATUS_OK) return rc;
    if (!ctEq(&signer, tls_cert_sha256_32[0..32])) return STATUS_AUTH;
    return STATUS_OK;
}

export fn tor_auth_challenge_build(out_body: [*]u8, challenge32: [*]const u8) i32 {
    @memcpy(out_body[0..32], challenge32[0..32]);
    put16be(out_body[32..34], 1);
    put16be(out_body[34..36], AUTH_ED25519_SHA256_RFC5705);
    return 36;
}

export fn tor_auth_challenge_parse(body: [*]const u8, body_len: u32, out_challenge32: [*]u8) i32 {
    if (body_len < 34) return STATUS_BOUNDS;
    @memcpy(out_challenge32[0..32], body[0..32]);
    const n = get16be(body[32..34]);
    if (body_len < 34 + @as(u32, n) * 2) return STATUS_BOUNDS;
    var i: usize = 0;
    while (i < n) : (i += 1) {
        if (get16be(body[34 + i * 2 ..][0..2]) == AUTH_ED25519_SHA256_RFC5705) return STATUS_OK;
    }
    return STATUS_UNSUPPORTED;
}

export fn tor_authenticate_auth0003_build(out_body: [*]u8, cid32: [*]const u8, sid32: [*]const u8, cid_ed32: [*]const u8, sid_ed32: [*]const u8, slog32: [*]const u8, clog32: [*]const u8, scert32: [*]const u8, tlssecrets32: [*]const u8, rand24: [*]const u8, link_sign_seed32: [*]const u8) i32 {
    put16be(out_body[0..2], AUTH_ED25519_SHA256_RFC5705);
    put16be(out_body[2..4], AUTH0003_LEN);
    @memcpy(out_body[4..12], "AUTH0003");
    @memcpy(out_body[12..44], cid32[0..32]);
    @memcpy(out_body[44..76], sid32[0..32]);
    @memcpy(out_body[76..108], cid_ed32[0..32]);
    @memcpy(out_body[108..140], sid_ed32[0..32]);
    @memcpy(out_body[140..172], slog32[0..32]);
    @memcpy(out_body[172..204], clog32[0..32]);
    @memcpy(out_body[204..236], scert32[0..32]);
    @memcpy(out_body[236..268], tlssecrets32[0..32]);
    @memcpy(out_body[268..292], rand24[0..24]);
    const kp = Ed25519.KeyPair.generateDeterministic(link_sign_seed32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.KeyPair.sign(kp, out_body[4..296], null) catch return STATUS_AUTH;
    const sig_bytes = sig.toBytes();
    @memcpy(out_body[296..360], sig_bytes[0..64]);
    return 360;
}

export fn tor_authenticate_auth0003_verify(body: [*]const u8, body_len: u32, cid32: [*]const u8, sid32: [*]const u8, cid_ed32: [*]const u8, sid_ed32: [*]const u8, slog32: [*]const u8, clog32: [*]const u8, scert32: [*]const u8, tlssecrets32: [*]const u8, link_pub32: [*]const u8) i32 {
    if (body_len < 360) return STATUS_BOUNDS;
    if (get16be(body[0..2]) != AUTH_ED25519_SHA256_RFC5705) return STATUS_UNSUPPORTED;
    if (get16be(body[2..4]) != AUTH0003_LEN) return STATUS_INVALID;
    if (!ctEq(body[4..12], "AUTH0003")) return STATUS_INVALID;
    if (!ctEq(body[12..44], cid32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[44..76], sid32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[76..108], cid_ed32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[108..140], sid_ed32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[140..172], slog32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[172..204], clog32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[204..236], scert32[0..32])) return STATUS_AUTH;
    if (!ctEq(body[236..268], tlssecrets32[0..32])) return STATUS_AUTH;
    const pk = Ed25519.PublicKey.fromBytes(link_pub32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.Signature.fromBytes(body[296..360].*);
    Ed25519.Signature.verify(sig, body[4..296], pk) catch return STATUS_AUTH;
    return STATUS_OK;
}

export fn tor_netinfo_build_ipv4(out_body: [*]u8, timestamp: u32, other_ipv4_be: u32, my_ipv4_be: u32, include_my: u32) i32 {
    put32be(out_body[0..4], timestamp);
    out_body[4] = 4;
    out_body[5] = 4;
    put32be(out_body[6..10], other_ipv4_be);
    if (include_my == 0) {
        out_body[10] = 0;
        return 11;
    }
    out_body[10] = 1;
    out_body[11] = 4;
    out_body[12] = 4;
    put32be(out_body[13..17], my_ipv4_be);
    return 17;
}
ZIG

zig build-exe "$WORK_DIR/tor_channel_handshake.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-channel-handshake.wasm"

wasm2wat "$WORK_DIR/tor-channel-handshake.wasm" -o "$OUT_DIR/tor-channel-handshake.wat"
wat2wasm "$OUT_DIR/tor-channel-handshake.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-channel-handshake.wat"
