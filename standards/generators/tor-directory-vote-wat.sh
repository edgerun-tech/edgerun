#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-directory-vote"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-directory-vote.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_directory_vote.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Sha256 = crypto.hash.sha2.Sha256;
const Ed25519 = crypto.sign.Ed25519;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;
const SIG_RECORD_LEN: u32 = 164;
const ALG_SHA256_ED25519: u32 = 2;
const MAX_DOC_LEN: u32 = 1024 * 1024;
const HEX = "0123456789abcdef";
const B64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn put32be(out: []u8, v: u32) void {
    out[0] = @intCast((v >> 24) & 0xff);
    out[1] = @intCast((v >> 16) & 0xff);
    out[2] = @intCast((v >> 8) & 0xff);
    out[3] = @intCast(v & 0xff);
}

fn get32be(s: []const u8) u32 {
    return (@as(u32, s[0]) << 24) | (@as(u32, s[1]) << 16) | (@as(u32, s[2]) << 8) | @as(u32, s[3]);
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

fn hexEncode(src: []const u8, out: []u8) usize {
    var i: usize = 0;
    while (i < src.len) : (i += 1) {
        out[i * 2] = HEX[src[i] >> 4];
        out[i * 2 + 1] = HEX[src[i] & 15];
    }
    return src.len * 2;
}

fn b64Encode(src: []const u8, out: []u8) usize {
    var i: usize = 0;
    var j: usize = 0;
    while (i + 3 <= src.len) : (i += 3) {
        const n = (@as(u32, src[i]) << 16) | (@as(u32, src[i + 1]) << 8) | @as(u32, src[i + 2]);
        out[j] = B64[(n >> 18) & 63];
        out[j + 1] = B64[(n >> 12) & 63];
        out[j + 2] = B64[(n >> 6) & 63];
        out[j + 3] = B64[n & 63];
        j += 4;
    }
    const rem = src.len - i;
    if (rem == 1) {
        const n = @as(u32, src[i]) << 16;
        out[j] = B64[(n >> 18) & 63];
        out[j + 1] = B64[(n >> 12) & 63];
        out[j + 2] = '=';
        out[j + 3] = '=';
        j += 4;
    } else if (rem == 2) {
        const n = (@as(u32, src[i]) << 16) | (@as(u32, src[i + 1]) << 8);
        out[j] = B64[(n >> 18) & 63];
        out[j + 1] = B64[(n >> 12) & 63];
        out[j + 2] = B64[(n >> 6) & 63];
        out[j + 3] = '=';
        j += 4;
    }
    return j;
}

fn write(out: [*]u8, pos: *usize, data: []const u8) void {
    @memcpy(out[pos.* .. pos.* + data.len], data);
    pos.* += data.len;
}

fn writeHex32(out: [*]u8, pos: *usize, src32: []const u8) void {
    pos.* += hexEncode(src32[0..32], out[pos.* .. pos.* + 64]);
}

fn median3(a: u32, b: u32, c: u32) u32 {
    if ((a >= b and a <= c) or (a >= c and a <= b)) return a;
    if ((b >= a and b <= c) or (b >= c and b <= a)) return b;
    return c;
}

fn lowMedian3(a: u32, b: u32, c: u32) u32 {
    return median3(a, b, c);
}

fn threshold(count: u32) u32 {
    if (count == 0) return 0;
    return (count * 2) / 3 + 1;
}

fn sortU32(values: []u32) void {
    var i: usize = 1;
    while (i < values.len) : (i += 1) {
        const v = values[i];
        var j = i;
        while (j > 0 and values[j - 1] > v) : (j -= 1) {
            values[j] = values[j - 1];
        }
        values[j] = v;
    }
}

export fn proto_standard_id() u32 {
    return 300222;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_dir_vote_signature_record_len() u32 {
    return SIG_RECORD_LEN;
}

export fn tor_dir_vote_algorithm_sha256_ed25519() u32 {
    return ALG_SHA256_ED25519;
}

export fn tor_dir_vote_public_from_seed(signing_seed32: [*]const u8, out_signing_key32: [*]u8) i32 {
    const kp = Ed25519.KeyPair.generateDeterministic(signing_seed32[0..32].*) catch return STATUS_AUTH;
    const public_bytes = kp.public_key.toBytes();
    @memcpy(out_signing_key32[0..32], public_bytes[0..32]);
    return STATUS_OK;
}

export fn tor_dir_vote_doc_digest(document: [*]const u8, document_len: u32, out_digest32: [*]u8) i32 {
    if (document_len > MAX_DOC_LEN) return STATUS_BOUNDS;
    sha256(document[0..document_len], out_digest32[0..32]);
    return STATUS_OK;
}

export fn tor_dir_vote_hex32(input32: [*]const u8, out_hex64: [*]u8) i32 {
    return @intCast(hexEncode(input32[0..32], out_hex64[0..64]));
}

export fn tor_dir_vote_signature_object(record: [*]const u8, record_len: u32, out_text: [*]u8) i32 {
    if (record_len < SIG_RECORD_LEN) return STATUS_BOUNDS;
    var pos: usize = 0;
    write(out_text, &pos, "-----BEGIN SIGNATURE-----\n");
    pos += b64Encode(record[100..164], out_text[pos .. pos + 88]);
    write(out_text, &pos, "\n-----END SIGNATURE-----\n");
    return @intCast(pos);
}

export fn tor_dir_vote_directory_signature_block(record: [*]const u8, record_len: u32, out_text: [*]u8) i32 {
    if (record_len < SIG_RECORD_LEN) return STATUS_BOUNDS;
    if (get32be(record[0..4]) != ALG_SHA256_ED25519) return STATUS_UNSUPPORTED;
    var pos: usize = 0;
    write(out_text, &pos, "directory-signature sha256 ");
    writeHex32(out_text, &pos, record[4..36]);
    write(out_text, &pos, " ");
    writeHex32(out_text, &pos, record[36..68]);
    write(out_text, &pos, "\n");
    const obj_len = tor_dir_vote_signature_object(record, record_len, out_text + pos);
    if (obj_len < 0) return obj_len;
    pos += @intCast(obj_len);
    return @intCast(pos);
}

export fn tor_dir_vote_consensus_digest_line(digest32: [*]const u8, out_text: [*]u8) i32 {
    var pos: usize = 0;
    write(out_text, &pos, "consensus-digest ");
    writeHex32(out_text, &pos, digest32[0..32]);
    write(out_text, &pos, "\n");
    return @intCast(pos);
}

export fn tor_dir_vote_additional_digest_line(flavor: [*]const u8, flavor_len: u32, digest32: [*]const u8, out_text: [*]u8) i32 {
    if (flavor_len == 0 or flavor_len > 32) return STATUS_BOUNDS;
    var pos: usize = 0;
    write(out_text, &pos, "additional-digest ");
    write(out_text, &pos, flavor[0..flavor_len]);
    write(out_text, &pos, " sha256 ");
    writeHex32(out_text, &pos, digest32[0..32]);
    write(out_text, &pos, "\n");
    return @intCast(pos);
}

export fn tor_dir_vote_additional_signature_line(flavor: [*]const u8, flavor_len: u32, record: [*]const u8, record_len: u32, out_text: [*]u8) i32 {
    if (flavor_len == 0 or flavor_len > 32) return STATUS_BOUNDS;
    if (record_len < SIG_RECORD_LEN) return STATUS_BOUNDS;
    if (get32be(record[0..4]) != ALG_SHA256_ED25519) return STATUS_UNSUPPORTED;
    var pos: usize = 0;
    write(out_text, &pos, "additional-signature ");
    write(out_text, &pos, flavor[0..flavor_len]);
    write(out_text, &pos, " sha256 ");
    writeHex32(out_text, &pos, record[4..36]);
    write(out_text, &pos, " ");
    writeHex32(out_text, &pos, record[36..68]);
    write(out_text, &pos, "\n");
    const obj_len = tor_dir_vote_signature_object(record, record_len, out_text + pos);
    if (obj_len < 0) return obj_len;
    pos += @intCast(obj_len);
    return @intCast(pos);
}

export fn tor_dir_vote_detached_timing_block(valid_after: [*]const u8, valid_after_len: u32, fresh_until: [*]const u8, fresh_until_len: u32, valid_until: [*]const u8, valid_until_len: u32, out_text: [*]u8) i32 {
    if (valid_after_len > 32 or fresh_until_len > 32 or valid_until_len > 32) return STATUS_BOUNDS;
    var pos: usize = 0;
    write(out_text, &pos, "valid-after ");
    write(out_text, &pos, valid_after[0..valid_after_len]);
    write(out_text, &pos, "\nfresh-until ");
    write(out_text, &pos, fresh_until[0..fresh_until_len]);
    write(out_text, &pos, "\nvalid-until ");
    write(out_text, &pos, valid_until[0..valid_until_len]);
    write(out_text, &pos, "\n");
    return @intCast(pos);
}

export fn tor_dir_vote_build_signature_record(out_record: [*]u8, identity32: [*]const u8, signing_key32: [*]const u8, digest32: [*]const u8, sig64: [*]const u8, algorithm: u32) i32 {
    if (algorithm != ALG_SHA256_ED25519) return STATUS_UNSUPPORTED;
    put32be(out_record[0..4], algorithm);
    @memcpy(out_record[4..36], identity32[0..32]);
    @memcpy(out_record[36..68], signing_key32[0..32]);
    @memcpy(out_record[68..100], digest32[0..32]);
    @memcpy(out_record[100..164], sig64[0..64]);
    return SIG_RECORD_LEN;
}

export fn tor_dir_vote_parse_signature_record(record: [*]const u8, record_len: u32, out_view_u32: [*]u32) i32 {
    if (record_len < SIG_RECORD_LEN) return STATUS_BOUNDS;
    const algorithm = get32be(record[0..4]);
    if (algorithm != ALG_SHA256_ED25519) return STATUS_UNSUPPORTED;
    out_view_u32[0] = algorithm;
    out_view_u32[1] = 4;
    out_view_u32[2] = 36;
    out_view_u32[3] = 68;
    out_view_u32[4] = 100;
    out_view_u32[5] = SIG_RECORD_LEN;
    return STATUS_OK;
}

export fn tor_dir_vote_sign(document: [*]const u8, document_len: u32, identity32: [*]const u8, signing_seed32: [*]const u8, out_record: [*]u8) i32 {
    if (document_len > MAX_DOC_LEN) return STATUS_BOUNDS;
    var digest: [32]u8 = undefined;
    sha256(document[0..document_len], &digest);
    const kp = Ed25519.KeyPair.generateDeterministic(signing_seed32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.KeyPair.sign(kp, &digest, null) catch return STATUS_AUTH;
    Ed25519.Signature.verify(sig, &digest, kp.public_key) catch return STATUS_AUTH;
    const public_bytes = kp.public_key.toBytes();
    const sig_bytes = sig.toBytes();
    return tor_dir_vote_build_signature_record(out_record, identity32, &public_bytes, &digest, &sig_bytes, ALG_SHA256_ED25519);
}

export fn tor_dir_vote_verify(document: [*]const u8, document_len: u32, identity32: [*]const u8, signing_key32: [*]const u8, record: [*]const u8, record_len: u32) i32 {
    if (document_len > MAX_DOC_LEN) return STATUS_BOUNDS;
    if (record_len < SIG_RECORD_LEN) return STATUS_BOUNDS;
    if (get32be(record[0..4]) != ALG_SHA256_ED25519) return STATUS_UNSUPPORTED;
    if (!ctEq(record[4..36], identity32[0..32])) return STATUS_AUTH;
    if (!ctEq(record[36..68], signing_key32[0..32])) return STATUS_AUTH;
    var digest: [32]u8 = undefined;
    sha256(document[0..document_len], &digest);
    if (!ctEq(record[68..100], &digest)) return STATUS_AUTH;
    const pk = Ed25519.PublicKey.fromBytes(signing_key32[0..32].*) catch return STATUS_AUTH;
    const sig = Ed25519.Signature.fromBytes(record[100..164].*);
    Ed25519.Signature.verify(sig, &digest, pk) catch return STATUS_AUTH;
    return STATUS_OK;
}

export fn tor_dir_vote_median3(a: u32, b: u32, c: u32) u32 {
    return median3(a, b, c);
}

export fn tor_dir_vote_low_median3(a: u32, b: u32, c: u32) u32 {
    return lowMedian3(a, b, c);
}

export fn tor_dir_vote_timing_median3(valid_after_a: u32, valid_after_b: u32, valid_after_c: u32, fresh_until_a: u32, fresh_until_b: u32, fresh_until_c: u32, valid_until_a: u32, valid_until_b: u32, valid_until_c: u32, out_three_u32: [*]u32) i32 {
    out_three_u32[0] = median3(valid_after_a, valid_after_b, valid_after_c);
    out_three_u32[1] = median3(fresh_until_a, fresh_until_b, fresh_until_c);
    out_three_u32[2] = median3(valid_until_a, valid_until_b, valid_until_c);
    return STATUS_OK;
}

export fn tor_dir_vote_params_low_median3(a: u32, b: u32, c: u32) u32 {
    return lowMedian3(a, b, c);
}

export fn tor_dir_vote_known_flags_union(flags_a: u32, flags_b: u32, flags_c: u32) u32 {
    return flags_a | flags_b | flags_c;
}

export fn tor_dir_vote_threshold(authority_count: u32) u32 {
    return threshold(authority_count);
}

export fn tor_dir_vote_method_mask(methods: [*]const u8, method_count: u32, out_two_u32: [*]u32) i32 {
    if (method_count > 256) return STATUS_BOUNDS;
    out_two_u32[0] = 0;
    out_two_u32[1] = 0;
    var i: usize = 0;
    while (i < method_count) : (i += 1) {
        const method = methods[i];
        if (method == 0 or method > 64) return STATUS_INVALID;
        const bit: u5 = @intCast((method - 1) & 31);
        if (method <= 32) {
            out_two_u32[0] |= @as(u32, 1) << bit;
        } else {
            out_two_u32[1] |= @as(u32, 1) << bit;
        }
    }
    return STATUS_OK;
}

export fn tor_dir_vote_choose_method_masks(masks: [*]const u32, authority_count: u32) i32 {
    if (authority_count == 0 or authority_count > 256) return STATUS_BOUNDS;
    const q = threshold(authority_count);
    var method: i32 = 64;
    while (method >= 1) : (method -= 1) {
        const m: u32 = @intCast(method);
        const word: usize = if (m <= 32) 0 else 1;
        const bit: u5 = @intCast((m - 1) & 31);
        var votes: u32 = 0;
        var i: usize = 0;
        while (i < authority_count) : (i += 1) {
            if ((masks[i * 2 + word] & (@as(u32, 1) << bit)) != 0) votes += 1;
        }
        if (votes >= q) return method;
    }
    return STATUS_UNSUPPORTED;
}

export fn tor_dir_vote_flag_consensus(flags: [*]const u32, authority_count: u32) u32 {
    if (authority_count == 0 or authority_count > 256) return 0;
    const q = threshold(authority_count);
    var out: u32 = 0;
    var bit_index: u32 = 0;
    while (bit_index < 32) : (bit_index += 1) {
        const shift: u5 = @intCast(bit_index);
        const bit = @as(u32, 1) << shift;
        var votes: u32 = 0;
        var i: usize = 0;
        while (i < authority_count) : (i += 1) {
            if ((flags[i] & bit) != 0) votes += 1;
        }
        if (votes >= q) out |= bit;
    }
    return out;
}

export fn tor_dir_vote_median_u32(values: [*]const u32, count: u32) i32 {
    if (count == 0 or count > 64) return STATUS_BOUNDS;
    var tmp: [64]u32 = undefined;
    var i: usize = 0;
    while (i < count) : (i += 1) tmp[i] = values[i];
    sortU32(tmp[0..count]);
    return @intCast(tmp[count / 2]);
}

export fn tor_dir_vote_low_median_u32(values: [*]const u32, count: u32) i32 {
    if (count == 0 or count > 64) return STATUS_BOUNDS;
    var tmp: [64]u32 = undefined;
    var i: usize = 0;
    while (i < count) : (i += 1) tmp[i] = values[i];
    sortU32(tmp[0..count]);
    return @intCast(tmp[(count - 1) / 2]);
}
ZIG

zig build-exe "$WORK_DIR/tor_directory_vote.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-directory-vote.wasm"

wasm2wat "$WORK_DIR/tor-directory-vote.wasm" -o "$OUT_DIR/tor-directory-vote.wat"
wat2wasm "$OUT_DIR/tor-directory-vote.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-directory-vote.wat"
