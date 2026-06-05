#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-cgo-crypto"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-cgo-crypto.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_cgo_crypto.zig" <<'ZIG'
const std = @import("std");
const crypto = std.crypto;
const Aes128 = crypto.core.aes.Aes128;
const modes = crypto.core.modes;
const Polyval = crypto.onetimeauth.Polyval;

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;

const MSG_LEN: usize = 509;
const BLK_LEN: usize = 16;
const XR_LEN: usize = MSG_LEN - BLK_LEN;
const KLEN_UIV: usize = 64;
const STATE_LEN: usize = KLEN_UIV + BLK_LEN + BLK_LEN;
const UPDATE_LEN: usize = KLEN_UIV + BLK_LEN;
const PRF_C: u128 = 31;

fn xorInto(out: []u8, a: []const u8, b: []const u8) void {
    for (out, a, b) |*o, x, y| o.* = x ^ y;
}

fn ctEq(a: []const u8, b: []const u8) bool {
    if (a.len != b.len) return false;
    var acc: u8 = 0;
    for (a, b) |x, y| acc |= x ^ y;
    return acc == 0;
}

fn uh(key16: []const u8, msg: []const u8, out16: *[16]u8) void {
    const key = key16[0..16].*;
    var st = Polyval.init(&key);
    st.update(msg);
    st.final(out16);
}

fn aesEnc(key16: []const u8, block16: []const u8, out16: *[16]u8) void {
    const key = key16[0..16].*;
    const block = block16[0..16].*;
    const ctx = Aes128.initEnc(key);
    ctx.encrypt(out16, &block);
}

fn aesDec(key16: []const u8, block16: []const u8, out16: *[16]u8) void {
    const key = key16[0..16].*;
    const block = block16[0..16].*;
    const ctx = Aes128.initDec(key);
    ctx.decrypt(out16, &block);
}

fn encEt(k: []const u8, tweak: []const u8, block: []const u8, out16: *[16]u8) void {
    var h: [16]u8 = undefined;
    var x: [16]u8 = undefined;
    var y: [16]u8 = undefined;
    uh(k[16..32], tweak, &h);
    xorInto(&x, block[0..16], &h);
    aesEnc(k[0..16], &x, &y);
    xorInto(out16, &y, &h);
}

fn decEt(k: []const u8, tweak: []const u8, block: []const u8, out16: *[16]u8) void {
    var h: [16]u8 = undefined;
    var x: [16]u8 = undefined;
    var y: [16]u8 = undefined;
    uh(k[16..32], tweak, &h);
    xorInto(&x, block[0..16], &h);
    aesDec(k[0..16], &x, &y);
    xorInto(out16, &y, &h);
}

fn addCounter(c: *[16]u8, add: u128) void {
    var n = std.mem.readInt(u128, c, .big);
    n +%= add;
    std.mem.writeInt(u128, c, n, .big);
}

fn prf(s: []const u8, input: []const u8, t: u8, out: []u8) void {
    var ctr: [16]u8 = undefined;
    uh(s[16..32], input, &ctr);
    ctr[15] &= 0xc0;
    if (t != 0) addCounter(&ctr, PRF_C);
    const key = s[0..16].*;
    const ctx = Aes128.initEnc(key);
    modes.ctr(@TypeOf(ctx), ctx, out, out, ctr, .big);
}

fn updateUiv(k: []u8, nonce: []u8) void {
    var material: [UPDATE_LEN]u8 = @splat(0);
    prf(k[32..64], nonce[0..16], 1, &material);
    @memcpy(k[0..64], material[0..64]);
    @memcpy(nonce[0..16], material[64..80]);
}

fn encUiv(k: []const u8, h: []const u8, input: []const u8, out: []u8) void {
    var tweak: [BLK_LEN + 1 + XR_LEN]u8 = undefined;
    var left: [16]u8 = undefined;
    var stream: [XR_LEN]u8 = @splat(0);
    @memcpy(tweak[0..h.len], h);
    @memcpy(tweak[h.len..][0..XR_LEN], input[16..MSG_LEN]);
    encEt(k[0..32], tweak[0 .. h.len + XR_LEN], input[0..16], &left);
    prf(k[32..64], &left, 0, &stream);
    @memcpy(out[0..16], &left);
    var i: usize = 0;
    while (i < XR_LEN) : (i += 1) out[16 + i] = input[16 + i] ^ stream[i];
}

fn decUiv(k: []const u8, h: []const u8, input: []const u8, out: []u8) void {
    var stream: [XR_LEN]u8 = @splat(0);
    var xr: [XR_LEN]u8 = undefined;
    var tweak: [BLK_LEN + 1 + XR_LEN]u8 = undefined;
    var left: [16]u8 = undefined;
    prf(k[32..64], input[0..16], 0, &stream);
    var i: usize = 0;
    while (i < XR_LEN) : (i += 1) xr[i] = input[16 + i] ^ stream[i];
    @memcpy(tweak[0..h.len], h);
    @memcpy(tweak[h.len..][0..XR_LEN], &xr);
    decEt(k[0..32], tweak[0 .. h.len + XR_LEN], input[0..16], &left);
    @memcpy(out[0..16], &left);
    @memcpy(out[16..MSG_LEN], &xr);
}

fn stateK(state: [*]u8) []u8 {
    return state[0..64];
}

fn stateN(state: [*]u8) []u8 {
    return state[64..80];
}

fn stateTprev(state: [*]u8) []u8 {
    return state[80..96];
}

fn makeH(state: [*]u8, ad: u8, out: *[17]u8) void {
    @memcpy(out[0..16], stateTprev(state));
    out[16] = ad;
}

export fn proto_standard_id() u32 {
    return 300210;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_cgo_msg_len() u32 {
    return MSG_LEN;
}

export fn tor_cgo_body_len() u32 {
    return XR_LEN;
}

export fn tor_cgo_key_len() u32 {
    return UPDATE_LEN;
}

export fn tor_cgo_state_len() u32 {
    return STATE_LEN;
}

export fn tor_cgo_polyval(key16: [*]const u8, msg: [*]const u8, len: u32, out16: [*]u8) i32 {
    var out: [16]u8 = undefined;
    uh(key16[0..16], msg[0..len], &out);
    @memcpy(out16[0..16], &out);
    return STATUS_OK;
}

export fn tor_cgo_state_init(state: [*]u8, key80: [*]const u8) i32 {
    @memcpy(stateK(state), key80[0..64]);
    @memcpy(stateN(state), key80[64..80]);
    @memset(stateTprev(state), 0);
    return STATUS_OK;
}

export fn tor_cgo_update(state: [*]u8, nonce16: [*]const u8) i32 {
    var n: [16]u8 = nonce16[0..16].*;
    updateUiv(stateK(state), &n);
    @memcpy(stateN(state), &n);
    return STATUS_OK;
}

export fn tor_cgo_encrypt_op_dest(state: [*]u8, ad: u32, body493: [*]const u8, out509: [*]u8) i32 {
    var h: [17]u8 = undefined;
    var input: [MSG_LEN]u8 = undefined;
    makeH(state, @truncate(ad), &h);
    @memcpy(input[0..16], stateN(state));
    @memcpy(input[16..MSG_LEN], body493[0..XR_LEN]);
    @memcpy(stateTprev(state), input[0..16]);
    decUiv(stateK(state), &h, &input, out509[0..MSG_LEN]);
    updateUiv(stateK(state), stateN(state));
    return STATUS_OK;
}

export fn tor_cgo_decrypt_or(state: [*]u8, ad: u32, cell509: [*]const u8, out509: [*]u8) i32 {
    var h: [17]u8 = undefined;
    var transformed: [MSG_LEN]u8 = undefined;
    makeH(state, @truncate(ad), &h);
    encUiv(stateK(state), &h, cell509[0..MSG_LEN], &transformed);
    const recognized = ctEq(transformed[0..16], stateN(state));
    if (recognized) {
        updateUiv(stateK(state), stateN(state));
    }
    @memcpy(stateTprev(state), transformed[0..16]);
    @memcpy(out509[0..MSG_LEN], &transformed);
    return if (recognized) 1 else 0;
}

export fn tor_cgo_encrypt_or(state: [*]u8, ad: u32, body493: [*]const u8, out509: [*]u8) i32 {
    var h: [17]u8 = undefined;
    var input: [MSG_LEN]u8 = undefined;
    makeH(state, @truncate(ad), &h);
    @memcpy(input[0..16], stateN(state));
    @memcpy(input[16..MSG_LEN], body493[0..XR_LEN]);
    encUiv(stateK(state), &h, &input, out509[0..MSG_LEN]);
    updateUiv(stateK(state), out509[0..16]);
    @memcpy(stateTprev(state), out509[0..16]);
    return STATUS_OK;
}

export fn tor_cgo_proc_or(state: [*]u8, ad: u32, cell509: [*]const u8, out509: [*]u8) i32 {
    var h: [17]u8 = undefined;
    makeH(state, @truncate(ad), &h);
    encUiv(stateK(state), &h, cell509[0..MSG_LEN], out509[0..MSG_LEN]);
    @memcpy(stateTprev(state), out509[0..16]);
    return STATUS_OK;
}

export fn tor_cgo_build_body(out493: [*]u8, command: u32, stream_id: u32, data: [*]const u8, data_len: u32) i32 {
    const has_stream = command == 1 or command == 2 or command == 3 or command == 4 or command == 11 or command == 12 or command == 13 or command == 43 or command == 44;
    const header_len: usize = if (has_stream) 5 else 3;
    if (data_len > XR_LEN - header_len - 4) return STATUS_BOUNDS;
    @memset(out493[0..XR_LEN], 0);
    out493[0] = @truncate(command);
    out493[1] = @truncate(data_len >> 8);
    out493[2] = @truncate(data_len);
    var pos: usize = 3;
    if (has_stream) {
        out493[3] = @truncate(stream_id >> 8);
        out493[4] = @truncate(stream_id);
        pos = 5;
    }
    @memcpy(out493[pos..][0..data_len], data[0..data_len]);
    return STATUS_OK;
}

export fn tor_cgo_parse_body(body493: [*]const u8, out_view: [*]u32) i32 {
    const cmd = body493[0];
    const data_len: u32 = (@as(u32, body493[1]) << 8) | body493[2];
    const has_stream = cmd == 1 or cmd == 2 or cmd == 3 or cmd == 4 or cmd == 11 or cmd == 12 or cmd == 13 or cmd == 43 or cmd == 44;
    const header_len: u32 = if (has_stream) 5 else 3;
    if (data_len > XR_LEN - header_len) return STATUS_INVALID;
    out_view[0] = cmd;
    out_view[1] = data_len;
    out_view[2] = if (has_stream) ((@as(u32, body493[3]) << 8) | body493[4]) else 0;
    out_view[3] = header_len;
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_cgo_crypto.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-cgo-crypto.wasm"

wasm2wat "$WORK_DIR/tor-cgo-crypto.wasm" -o "$OUT_DIR/tor-cgo-crypto.wat"
wat2wasm "$OUT_DIR/tor-cgo-crypto.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
