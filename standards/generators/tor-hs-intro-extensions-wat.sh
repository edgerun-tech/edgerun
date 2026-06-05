#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-hs-intro-extensions"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-hs-intro-extensions.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_hs_intro_extensions.zig" <<'ZIG'
const std = @import("std");
const Blake2b32 = std.crypto.hash.blake2.Blake2b(32);

const STATUS_OK: i32 = 0;
const STATUS_NOT_FOUND: i32 = 1;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;

const EXT_CC: u8 = 1;
const EXT_POW: u8 = 2;
const POW_SCHEME_V1: u8 = 1;
const POW_EXT_LEN: usize = 41;
const POW_NONCE_LEN: usize = 16;
const POW_SEED_PREFIX_LEN: usize = 4;
const POW_SOLUTION_LEN: usize = 16;
const POW_PERSONALIZATION = "Tor hs intro v1\x00";
const POW_CHALLENGE_LEN: usize = POW_PERSONALIZATION.len + 32 + 32 + 16 + 4;
const MAX_EXT_BLOCK: usize = 426;

fn get32be(s: []const u8) u32 {
    return (@as(u32, s[0]) << 24) | (@as(u32, s[1]) << 16) | (@as(u32, s[2]) << 8) | @as(u32, s[3]);
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

fn validateBlock(block: []const u8) i32 {
    if (block.len < 1 or block.len > MAX_EXT_BLOCK) return STATUS_BOUNDS;
    var pos: usize = 1;
    var n: usize = block[0];
    while (n > 0) {
        if (pos + 2 > block.len) return STATUS_INVALID;
        const typ = block[pos];
        const len: usize = block[pos + 1];
        pos += 2;
        if (pos + len > block.len) return STATUS_INVALID;
        if (typ == EXT_CC and len != 0) return STATUS_INVALID;
        if (typ == EXT_POW) {
            if (len != POW_EXT_LEN) return STATUS_INVALID;
            if (block[pos] != POW_SCHEME_V1) return STATUS_UNSUPPORTED;
        }
        pos += len;
        n -= 1;
    }
    if (pos != block.len) return STATUS_INVALID;
    return STATUS_OK;
}

fn nthRecord(block: []const u8, index: usize, out: []u32) i32 {
    const rc = validateBlock(block);
    if (rc != STATUS_OK) return rc;
    if (index >= block[0]) return STATUS_NOT_FOUND;
    var pos: usize = 1;
    var i: usize = 0;
    while (i < index) : (i += 1) {
        pos += 2 + @as(usize, block[pos + 1]);
    }
    out[0] = block[pos];
    out[1] = @intCast(pos + 2);
    out[2] = block[pos + 1];
    out[3] = @intCast(pos + 2 + @as(usize, block[pos + 1]));
    return STATUS_OK;
}

fn findRecord(block: []const u8, typ: u8, out: []u32) i32 {
    const rc = validateBlock(block);
    if (rc != STATUS_OK) return rc;
    var pos: usize = 1;
    var n: usize = block[0];
    while (n > 0) {
        const rec_type = block[pos];
        const len: usize = block[pos + 1];
        if (rec_type == typ) {
            out[0] = rec_type;
            out[1] = @intCast(pos + 2);
            out[2] = @intCast(len);
            out[3] = @intCast(pos + 2 + len);
            return STATUS_OK;
        }
        pos += 2 + len;
        n -= 1;
    }
    return STATUS_NOT_FOUND;
}

fn challenge(identity32: []const u8, seed32: []const u8, nonce16: []const u8, effort: u32, out: []u8) void {
    var pos: usize = 0;
    @memcpy(out[pos..][0..POW_PERSONALIZATION.len], POW_PERSONALIZATION); pos += POW_PERSONALIZATION.len;
    @memcpy(out[pos..][0..32], identity32[0..32]); pos += 32;
    @memcpy(out[pos..][0..32], seed32[0..32]); pos += 32;
    @memcpy(out[pos..][0..16], nonce16[0..16]); pos += 16;
    put32be(out[pos..][0..4], effort);
}

fn blake2b32be(input: []const u8) u32 {
    var out: [4]u8 = undefined;
    Blake2b32.hash(input, &out, .{});
    return get32be(&out);
}

export fn proto_standard_id() u32 {
    return 300216;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_hs_intro_ext_pow_len() u32 {
    return POW_EXT_LEN;
}

export fn tor_hs_intro_ext_pow_challenge_len() u32 {
    return POW_CHALLENGE_LEN;
}

export fn tor_hs_intro_ext_empty(out_block: [*]u8) i32 {
    out_block[0] = 0;
    return 1;
}

export fn tor_hs_intro_ext_build_cc(out_block: [*]u8) i32 {
    out_block[0] = 1;
    out_block[1] = EXT_CC;
    out_block[2] = 0;
    return 3;
}

export fn tor_hs_intro_ext_build_pow(out_block: [*]u8, nonce16: [*]const u8, effort: u32, seed_prefix4: [*]const u8, solution16: [*]const u8) i32 {
    out_block[0] = 1;
    out_block[1] = EXT_POW;
    out_block[2] = POW_EXT_LEN;
    out_block[3] = POW_SCHEME_V1;
    @memcpy(out_block[4..20], nonce16[0..16]);
    put32be(out_block[20..24], effort);
    @memcpy(out_block[24..28], seed_prefix4[0..4]);
    @memcpy(out_block[28..44], solution16[0..16]);
    return 44;
}

export fn tor_hs_intro_ext_build_cc_pow(out_block: [*]u8, nonce16: [*]const u8, effort: u32, seed_prefix4: [*]const u8, solution16: [*]const u8) i32 {
    out_block[0] = 2;
    out_block[1] = EXT_CC;
    out_block[2] = 0;
    out_block[3] = EXT_POW;
    out_block[4] = POW_EXT_LEN;
    out_block[5] = POW_SCHEME_V1;
    @memcpy(out_block[6..22], nonce16[0..16]);
    put32be(out_block[22..26], effort);
    @memcpy(out_block[26..30], seed_prefix4[0..4]);
    @memcpy(out_block[30..46], solution16[0..16]);
    return 46;
}

export fn tor_hs_intro_ext_validate(block: [*]const u8, block_len: u32) i32 {
    return validateBlock(block[0..block_len]);
}

export fn tor_hs_intro_ext_record(block: [*]const u8, block_len: u32, index: u32, out_view_u32: [*]u32) i32 {
    return nthRecord(block[0..block_len], index, out_view_u32[0..4]);
}

export fn tor_hs_intro_ext_find(block: [*]const u8, block_len: u32, typ: u32, out_view_u32: [*]u32) i32 {
    if (typ > 255) return STATUS_INVALID;
    return findRecord(block[0..block_len], @intCast(typ), out_view_u32[0..4]);
}

export fn tor_hs_intro_ext_parse_pow(ext_data: [*]const u8, ext_len: u32, out_nonce16: [*]u8, out_effort: [*]u32, out_seed_prefix4: [*]u8, out_solution16: [*]u8) i32 {
    if (ext_len != POW_EXT_LEN) return STATUS_INVALID;
    if (ext_data[0] != POW_SCHEME_V1) return STATUS_UNSUPPORTED;
    @memcpy(out_nonce16[0..16], ext_data[1..17]);
    out_effort[0] = get32be(ext_data[17..21]);
    @memcpy(out_seed_prefix4[0..4], ext_data[21..25]);
    @memcpy(out_solution16[0..16], ext_data[25..41]);
    return STATUS_OK;
}

export fn tor_hs_intro_pow_seed_prefix_ok(seed32: [*]const u8, seed_prefix4: [*]const u8) i32 {
    return if (ctEq(seed32[0..4], seed_prefix4[0..4])) STATUS_OK else STATUS_AUTH;
}

export fn tor_hs_intro_pow_challenge(identity32: [*]const u8, seed32: [*]const u8, nonce16: [*]const u8, effort: u32, out_challenge: [*]u8) i32 {
    challenge(identity32[0..32], seed32[0..32], nonce16[0..16], effort, out_challenge[0..POW_CHALLENGE_LEN]);
    return POW_CHALLENGE_LEN;
}

export fn tor_hs_intro_pow_blake2b_result(identity32: [*]const u8, seed32: [*]const u8, nonce16: [*]const u8, effort: u32, solution16: [*]const u8) u32 {
    var input: [POW_CHALLENGE_LEN + POW_SOLUTION_LEN]u8 = undefined;
    challenge(identity32[0..32], seed32[0..32], nonce16[0..16], effort, input[0..POW_CHALLENGE_LEN]);
    @memcpy(input[POW_CHALLENGE_LEN..][0..16], solution16[0..16]);
    return blake2b32be(&input);
}

export fn tor_hs_intro_pow_effort_gate(identity32: [*]const u8, seed32: [*]const u8, nonce16: [*]const u8, effort: u32, seed_prefix4: [*]const u8, solution16: [*]const u8, out_r: [*]u32) i32 {
    if (tor_hs_intro_pow_seed_prefix_ok(seed32, seed_prefix4) != STATUS_OK) return STATUS_AUTH;
    const r = tor_hs_intro_pow_blake2b_result(identity32, seed32, nonce16, effort, solution16);
    out_r[0] = r;
    if (@as(u64, r) * @as(u64, effort) > 0xffff_ffff) return STATUS_AUTH;
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_hs_intro_extensions.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-hs-intro-extensions.wasm"

wasm2wat "$WORK_DIR/tor-hs-intro-extensions.wasm" -o "$OUT_DIR/tor-hs-intro-extensions.wat"
wat2wasm "$OUT_DIR/tor-hs-intro-extensions.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
