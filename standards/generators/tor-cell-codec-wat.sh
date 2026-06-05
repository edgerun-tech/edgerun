#!/usr/bin/env sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT_DIR="$ROOT/build/wasm/app-primitives/tor-cell-codec"
WORK_DIR="${TMPDIR:-/tmp}/edgerun-tor-cell-codec.$$"
mkdir -p "$OUT_DIR" "$WORK_DIR"
trap 'rm -rf "$WORK_DIR"' EXIT

cat > "$WORK_DIR/tor_cell_codec.zig" <<'ZIG'
const std = @import("std");

const STATUS_OK: i32 = 0;
const STATUS_INVALID: i32 = -1;
const STATUS_BOUNDS: i32 = -2;
const STATUS_AUTH: i32 = -3;
const STATUS_UNSUPPORTED: i32 = -4;
const CELL_BODY_LEN: u32 = 509;
const RELAY_HEADER_LEN: u32 = 11;
const RELAY_DATA_MAX: u32 = 498;
const CMD_PADDING: u8 = 0;
const CMD_CREATE: u8 = 1;
const CMD_CREATED: u8 = 2;
const CMD_RELAY: u8 = 3;
const CMD_DESTROY: u8 = 4;
const CMD_CREATE_FAST: u8 = 5;
const CMD_CREATED_FAST: u8 = 6;
const CMD_VERSIONS: u8 = 7;
const CMD_NETINFO: u8 = 8;
const CMD_RELAY_EARLY: u8 = 9;
const CMD_CREATE2: u8 = 10;
const CMD_CREATED2: u8 = 11;
const CMD_PADDING_NEGOTIATE: u8 = 12;
const CMD_VPADDING: u8 = 128;
const CMD_CERTS: u8 = 129;
const CMD_AUTH_CHALLENGE: u8 = 130;
const CMD_AUTHENTICATE: u8 = 131;
const DESTROY_NONE: u8 = 0;
const DESTROY_MISC: u8 = 1;
const DESTROY_PROTOCOL: u8 = 2;
const DESTROY_INTERNAL: u8 = 3;
const DESTROY_REQUESTED: u8 = 4;
const DESTROY_HIBERNATING: u8 = 5;
const DESTROY_RESOURCELIMIT: u8 = 6;
const DESTROY_CONNECTFAILED: u8 = 7;
const DESTROY_OR_IDENTITY: u8 = 8;
const DESTROY_CHANNEL_CLOSED: u8 = 9;
const DESTROY_FINISHED: u8 = 10;
const DESTROY_DESTROYED: u8 = 11;
const DESTROY_NOSUCHSERVICE: u8 = 12;
const PADDING_NEGOTIATE_VERSION: u8 = 0;
const PADDING_NEGOTIATE_STOP: u8 = 1;
const PADDING_NEGOTIATE_START: u8 = 2;
const SENDME_V0: u8 = 0;
const SENDME_V1: u8 = 1;

fn relayCommandStreamRequirement(cmd: u8) i32 {
    return switch (cmd) {
        1, 2, 3, 4, 11, 12, 13, 43, 44 => 1,
        6, 7, 8, 9, 10, 14, 15, 19, 20, 21, 22, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42 => 0,
        5 => -2,
        16, 17, 18 => STATUS_UNSUPPORTED,
        else => STATUS_UNSUPPORTED,
    };
}

fn put16be(out: []u8, v: u16) void {
    out[0] = @intCast((v >> 8) & 0xff);
    out[1] = @intCast(v & 0xff);
}

fn get16be(s: []const u8) u16 {
    return (@as(u16, s[0]) << 8) | @as(u16, s[1]);
}

fn put32be(out: []u8, v: u32) void {
    out[0] = @intCast((v >> 24) & 0xff);
    out[1] = @intCast((v >> 16) & 0xff);
    out[2] = @intCast((v >> 8) & 0xff);
    out[3] = @intCast(v & 0xff);
}

fn get32be(s: []const u8) u32 {
    return (@as(u32, s[0]) << 24) | (@as(u32, s[1]) << 16) | (@as(u32, s[2]) << 8) | @as(u32, s[3]);
}

fn circLen(link_version: u32) i32 {
    if (link_version < 3 or link_version > 5) return STATUS_UNSUPPORTED;
    return if (link_version < 4) 2 else 4;
}

fn isVariable(cmd: u8) bool {
    return cmd == CMD_VERSIONS or cmd >= 128;
}

fn commandCircRequirement(cmd: u8) i32 {
    return switch (cmd) {
        CMD_PADDING, CMD_VERSIONS, CMD_VPADDING, CMD_CERTS, CMD_AUTH_CHALLENGE, CMD_AUTHENTICATE => 0,
        CMD_CREATE, CMD_CREATED, CMD_RELAY, CMD_DESTROY, CMD_CREATE_FAST, CMD_CREATED_FAST, CMD_NETINFO, CMD_RELAY_EARLY, CMD_CREATE2, CMD_CREATED2, CMD_PADDING_NEGOTIATE => 1,
        else => -1,
    };
}

fn checkCirc(cmd: u8, circ_id: u32) i32 {
    const req = commandCircRequirement(cmd);
    if (req < 0) return STATUS_UNSUPPORTED;
    if (req == 0 and circ_id != 0) return STATUS_INVALID;
    if (req == 1 and circ_id == 0) return STATUS_INVALID;
    return STATUS_OK;
}

fn writeCirc(out: []u8, link_version: u32, circ_id: u32) i32 {
    const clen = circLen(link_version);
    if (clen < 0) return clen;
    if (clen == 2) {
        if (circ_id > 0xffff) return STATUS_BOUNDS;
        put16be(out[0..2], @intCast(circ_id));
    } else {
        put32be(out[0..4], circ_id);
    }
    return clen;
}

fn readCirc(cell: []const u8, link_version: u32, out_view: []u32) i32 {
    const clen = circLen(link_version);
    if (clen < 0) return clen;
    if (cell.len < @as(usize, @intCast(clen)) + 1) return STATUS_BOUNDS;
    out_view[0] = if (clen == 2) get16be(cell[0..2]) else get32be(cell[0..4]);
    out_view[1] = @intCast(clen);
    return clen;
}

export fn proto_standard_id() u32 {
    return 300224;
}

export fn proto_abi_version() u32 {
    return 1;
}

export fn tor_cell_body_len() u32 {
    return CELL_BODY_LEN;
}

export fn tor_cell_relay_header_len() u32 {
    return RELAY_HEADER_LEN;
}

export fn tor_cell_relay_data_max() u32 {
    return RELAY_DATA_MAX;
}

export fn tor_cell_circ_id_len(link_version: u32) i32 {
    return circLen(link_version);
}

export fn tor_cell_fixed_len(link_version: u32) i32 {
    const clen = circLen(link_version);
    if (clen < 0) return clen;
    return clen + 1 + @as(i32, @intCast(CELL_BODY_LEN));
}

export fn tor_cell_var_header_len(link_version: u32) i32 {
    const clen = circLen(link_version);
    if (clen < 0) return clen;
    return clen + 3;
}

export fn tor_cell_is_variable_command(cmd: u32) i32 {
    if (cmd > 255) return STATUS_INVALID;
    return if (isVariable(@intCast(cmd))) 1 else 0;
}

export fn tor_cell_command_circ_requirement(cmd: u32) i32 {
    if (cmd > 255) return STATUS_INVALID;
    return commandCircRequirement(@intCast(cmd));
}

export fn tor_cell_destroy_reason_none() u32 {
    return DESTROY_NONE;
}

export fn tor_cell_destroy_reason_protocol() u32 {
    return DESTROY_PROTOCOL;
}

export fn tor_cell_padding_command_stop() u32 {
    return PADDING_NEGOTIATE_STOP;
}

export fn tor_cell_padding_command_start() u32 {
    return PADDING_NEGOTIATE_START;
}

export fn tor_cell_sendme_version_0() u32 {
    return SENDME_V0;
}

export fn tor_cell_sendme_version_1() u32 {
    return SENDME_V1;
}

export fn tor_cell_relay_stream_requirement(relay_cmd: u32) i32 {
    if (relay_cmd > 255) return STATUS_INVALID;
    return relayCommandStreamRequirement(@intCast(relay_cmd));
}

export fn tor_cell_validate_relay_stream_id(relay_cmd: u32, stream_id: u32) i32 {
    if (relay_cmd > 255 or stream_id > 65535) return STATUS_INVALID;
    const req = relayCommandStreamRequirement(@intCast(relay_cmd));
    if (req == STATUS_UNSUPPORTED) return STATUS_UNSUPPORTED;
    if (req == 0 and stream_id != 0) return STATUS_INVALID;
    if (req == 1 and stream_id == 0) return STATUS_INVALID;
    return STATUS_OK;
}

export fn tor_cell_validate_command(link_version: u32, cmd: u32, circ_id: u32, body_len: u32) i32 {
    if (cmd > 255) return STATUS_INVALID;
    const clen = circLen(link_version);
    if (clen < 0) return clen;
    if (clen == 2 and circ_id > 0xffff) return STATUS_BOUNDS;
    const c: u8 = @intCast(cmd);
    const rc = checkCirc(c, circ_id);
    if (rc != STATUS_OK) return rc;
    if (isVariable(c)) {
        if (body_len > 65535) return STATUS_BOUNDS;
    } else if (body_len > CELL_BODY_LEN) {
        return STATUS_BOUNDS;
    }
    return STATUS_OK;
}

export fn tor_cell_build_fixed(link_version: u32, out: [*]u8, circ_id: u32, cmd: u32, body: [*]const u8, body_len: u32) i32 {
    if (cmd > 255) return STATUS_INVALID;
    const c: u8 = @intCast(cmd);
    if (isVariable(c)) return STATUS_INVALID;
    if (body_len > CELL_BODY_LEN) return STATUS_BOUNDS;
    var rc = checkCirc(c, circ_id);
    if (rc != STATUS_OK) return rc;
    rc = writeCirc(out[0..4], link_version, circ_id);
    if (rc < 0) return rc;
    const clen: usize = @intCast(rc);
    out[clen] = c;
    @memset(out[clen + 1 .. clen + 1 + CELL_BODY_LEN], 0);
    if (body_len != 0) @memcpy(out[clen + 1 .. clen + 1 + body_len], body[0..body_len]);
    return @intCast(clen + 1 + CELL_BODY_LEN);
}

export fn tor_cell_parse_fixed(link_version: u32, cell: [*]const u8, cell_len: u32, out_view_u32: [*]u32) i32 {
    var view: [2]u32 = undefined;
    const clen_i = readCirc(cell[0..cell_len], link_version, &view);
    if (clen_i < 0) return clen_i;
    const clen: usize = @intCast(clen_i);
    const need = clen + 1 + CELL_BODY_LEN;
    if (cell_len < need) return STATUS_BOUNDS;
    const cmd = cell[clen];
    if (isVariable(cmd)) return STATUS_INVALID;
    const rc = checkCirc(cmd, view[0]);
    if (rc != STATUS_OK) return rc;
    out_view_u32[0] = view[0];
    out_view_u32[1] = cmd;
    out_view_u32[2] = @intCast(clen + 1);
    out_view_u32[3] = CELL_BODY_LEN;
    out_view_u32[4] = @intCast(need);
    return STATUS_OK;
}

export fn tor_cell_build_var(link_version: u32, out: [*]u8, circ_id: u32, cmd: u32, body: [*]const u8, body_len: u32) i32 {
    if (cmd > 255) return STATUS_INVALID;
    const c: u8 = @intCast(cmd);
    if (!isVariable(c)) return STATUS_INVALID;
    if (body_len > 65535) return STATUS_BOUNDS;
    var rc = checkCirc(c, circ_id);
    if (rc != STATUS_OK) return rc;
    rc = writeCirc(out[0..4], link_version, circ_id);
    if (rc < 0) return rc;
    const clen: usize = @intCast(rc);
    out[clen] = c;
    put16be(out[clen + 1 .. clen + 3], @intCast(body_len));
    if (body_len != 0) @memcpy(out[clen + 3 .. clen + 3 + body_len], body[0..body_len]);
    return @intCast(clen + 3 + body_len);
}

export fn tor_cell_parse_var(link_version: u32, cell: [*]const u8, cell_len: u32, out_view_u32: [*]u32) i32 {
    var view: [2]u32 = undefined;
    const clen_i = readCirc(cell[0..cell_len], link_version, &view);
    if (clen_i < 0) return clen_i;
    const clen: usize = @intCast(clen_i);
    if (cell_len < clen + 3) return STATUS_BOUNDS;
    const cmd = cell[clen];
    if (!isVariable(cmd)) return STATUS_INVALID;
    const rc = checkCirc(cmd, view[0]);
    if (rc != STATUS_OK) return rc;
    const body_len = get16be(cell[clen + 1 .. clen + 3]);
    if (cell_len < clen + 3 + body_len) return STATUS_BOUNDS;
    out_view_u32[0] = view[0];
    out_view_u32[1] = cmd;
    out_view_u32[2] = @intCast(clen + 3);
    out_view_u32[3] = body_len;
    out_view_u32[4] = @intCast(clen + 3 + body_len);
    return STATUS_OK;
}

export fn tor_cell_build_versions_body(out: [*]u8, versions: [*]const u16, version_count: u32) i32 {
    if (version_count == 0 or version_count > 32) return STATUS_BOUNDS;
    var i: usize = 0;
    while (i < version_count) : (i += 1) {
        const v = versions[i];
        if (v == 1 or v == 2) return STATUS_UNSUPPORTED;
        put16be(out[i * 2 .. i * 2 + 2], v);
    }
    return @intCast(version_count * 2);
}

export fn tor_cell_parse_versions_body(body: [*]const u8, body_len: u32, out_versions_u16: [*]u16, max_versions: u32) i32 {
    if (body_len == 0 or (body_len & 1) != 0) return STATUS_INVALID;
    const count = body_len / 2;
    if (count > max_versions or count > 32) return STATUS_BOUNDS;
    var i: usize = 0;
    while (i < count) : (i += 1) {
        const v = get16be(body[i * 2 .. i * 2 + 2]);
        if (v == 1 or v == 2) return STATUS_UNSUPPORTED;
        out_versions_u16[i] = v;
    }
    return @intCast(count);
}

export fn tor_cell_versions_negotiate(local_body: [*]const u8, local_len: u32, remote_body: [*]const u8, remote_len: u32) i32 {
    if (local_len == 0 or remote_len == 0 or (local_len & 1) != 0 or (remote_len & 1) != 0) return STATUS_INVALID;
    var best: u16 = 0;
    var i: usize = 0;
    while (i < local_len) : (i += 2) {
        const lv = get16be(local_body[i .. i + 2]);
        if (lv < 3 or lv > 5) continue;
        var j: usize = 0;
        while (j < remote_len) : (j += 2) {
            const rv = get16be(remote_body[j .. j + 2]);
            if (lv == rv and lv > best) best = lv;
        }
    }
    if (best == 0) return STATUS_UNSUPPORTED;
    return best;
}

export fn tor_cell_build_relay_payload(out: [*]u8, relay_cmd: u32, recognized: u32, stream_id: u32, digest4_le: u32, data: [*]const u8, data_len: u32) i32 {
    if (relay_cmd > 255 or recognized > 65535 or stream_id > 65535) return STATUS_INVALID;
    if (data_len > RELAY_DATA_MAX) return STATUS_BOUNDS;
    const rc = tor_cell_validate_relay_stream_id(relay_cmd, stream_id);
    if (rc != STATUS_OK) return rc;
    @memset(out[0..CELL_BODY_LEN], 0);
    out[0] = @intCast(relay_cmd);
    put16be(out[1..3], @intCast(recognized));
    put16be(out[3..5], @intCast(stream_id));
    std.mem.writeInt(u32, out[5..9], digest4_le, .little);
    put16be(out[9..11], @intCast(data_len));
    if (data_len != 0) @memcpy(out[11 .. 11 + data_len], data[0..data_len]);
    return @intCast(RELAY_HEADER_LEN + data_len);
}

export fn tor_cell_parse_relay_payload(payload: [*]const u8, payload_len: u32, out_view_u32: [*]u32) i32 {
    if (payload_len < RELAY_HEADER_LEN) return STATUS_BOUNDS;
    const data_len = get16be(payload[9..11]);
    if (data_len > RELAY_DATA_MAX or RELAY_HEADER_LEN + @as(u32, data_len) > payload_len) return STATUS_BOUNDS;
    out_view_u32[0] = payload[0];
    out_view_u32[1] = get16be(payload[1..3]);
    out_view_u32[2] = get16be(payload[3..5]);
    out_view_u32[3] = std.mem.readInt(u32, payload[5..9], .little);
    out_view_u32[4] = data_len;
    out_view_u32[5] = RELAY_HEADER_LEN;
    return tor_cell_validate_relay_stream_id(payload[0], out_view_u32[2]);
}

export fn tor_cell_build_create2_body(out: [*]u8, htype: u32, hdata: [*]const u8, hlen: u32) i32 {
    if (htype > 65535 or hlen > CELL_BODY_LEN - 4) return STATUS_BOUNDS;
    @memset(out[0..CELL_BODY_LEN], 0);
    put16be(out[0..2], @intCast(htype));
    put16be(out[2..4], @intCast(hlen));
    if (hlen != 0) @memcpy(out[4 .. 4 + hlen], hdata[0..hlen]);
    return @intCast(4 + hlen);
}

export fn tor_cell_parse_create2_body(body: [*]const u8, body_len: u32, out_view_u32: [*]u32) i32 {
    if (body_len < 4) return STATUS_BOUNDS;
    const hlen = get16be(body[2..4]);
    if (hlen > CELL_BODY_LEN - 4 or 4 + @as(u32, hlen) > body_len) return STATUS_BOUNDS;
    out_view_u32[0] = get16be(body[0..2]);
    out_view_u32[1] = 4;
    out_view_u32[2] = hlen;
    out_view_u32[3] = 4 + @as(u32, hlen);
    return STATUS_OK;
}

export fn tor_cell_build_created2_body(out: [*]u8, hdata: [*]const u8, hlen: u32) i32 {
    if (hlen > CELL_BODY_LEN - 2) return STATUS_BOUNDS;
    @memset(out[0..CELL_BODY_LEN], 0);
    put16be(out[0..2], @intCast(hlen));
    if (hlen != 0) @memcpy(out[2 .. 2 + hlen], hdata[0..hlen]);
    return @intCast(2 + hlen);
}

export fn tor_cell_parse_created2_body(body: [*]const u8, body_len: u32, out_view_u32: [*]u32) i32 {
    if (body_len < 2) return STATUS_BOUNDS;
    const hlen = get16be(body[0..2]);
    if (hlen > CELL_BODY_LEN - 2 or 2 + @as(u32, hlen) > body_len) return STATUS_BOUNDS;
    out_view_u32[0] = 2;
    out_view_u32[1] = hlen;
    out_view_u32[2] = 2 + @as(u32, hlen);
    return STATUS_OK;
}

export fn tor_cell_build_destroy_body(out: [*]u8, reason: u32) i32 {
    if (reason > 255) return STATUS_INVALID;
    @memset(out[0..CELL_BODY_LEN], 0);
    out[0] = @intCast(reason);
    return 1;
}

export fn tor_cell_parse_destroy_body(body: [*]const u8, body_len: u32) i32 {
    if (body_len < 1) return STATUS_BOUNDS;
    return body[0];
}

export fn tor_cell_build_padding_negotiate_body(out: [*]u8, command: u32, ito_low_ms: u32, ito_high_ms: u32, nf_ito_low_ms: u32) i32 {
    if (command != PADDING_NEGOTIATE_STOP and command != PADDING_NEGOTIATE_START) return STATUS_UNSUPPORTED;
    out[0] = PADDING_NEGOTIATE_VERSION;
    out[1] = @intCast(command);
    if (command == PADDING_NEGOTIATE_STOP) {
        put16be(out[2..4], 0);
        put16be(out[4..6], 0);
        return 6;
    }
    var low = if (ito_low_ms < nf_ito_low_ms) nf_ito_low_ms else ito_low_ms;
    if (low > 65535) low = 65535;
    var high = if (ito_high_ms < low) low else ito_high_ms;
    if (high > 65535) high = 65535;
    put16be(out[2..4], @intCast(low));
    put16be(out[4..6], @intCast(high));
    return 6;
}

export fn tor_cell_parse_padding_negotiate_body(body: [*]const u8, body_len: u32, nf_ito_low_ms: u32, out_view_u32: [*]u32) i32 {
    if (body_len < 6) return STATUS_BOUNDS;
    if (body[0] != PADDING_NEGOTIATE_VERSION) return STATUS_UNSUPPORTED;
    const command = body[1];
    if (command != PADDING_NEGOTIATE_STOP and command != PADDING_NEGOTIATE_START) return STATUS_UNSUPPORTED;
    out_view_u32[0] = body[0];
    out_view_u32[1] = command;
    if (command == PADDING_NEGOTIATE_STOP) {
        out_view_u32[2] = 0;
        out_view_u32[3] = 0;
        out_view_u32[4] = 6;
        return STATUS_OK;
    }
    var low: u32 = get16be(body[2..4]);
    if (low < nf_ito_low_ms) low = nf_ito_low_ms;
    if (low > 65535) low = 65535;
    var high: u32 = get16be(body[4..6]);
    if (high < low) high = low;
    out_view_u32[2] = low;
    out_view_u32[3] = high;
    out_view_u32[4] = 6;
    return STATUS_OK;
}

export fn tor_cell_build_sendme_v0(out: [*]u8) i32 {
    out[0] = SENDME_V0;
    put16be(out[1..3], 0);
    return 3;
}

export fn tor_cell_build_sendme_v1(out: [*]u8, digest20: [*]const u8) i32 {
    out[0] = SENDME_V1;
    put16be(out[1..3], 20);
    @memcpy(out[3..23], digest20[0..20]);
    return 23;
}

export fn tor_cell_parse_sendme_body(body: [*]const u8, body_len: u32, min_accept_version: u32, out_view_u32: [*]u32) i32 {
    if (body_len < 3) return STATUS_BOUNDS;
    const version = body[0];
    const data_len = get16be(body[1..3]);
    if (version < min_accept_version) return STATUS_UNSUPPORTED;
    if (body_len < 3 + @as(u32, data_len)) return STATUS_BOUNDS;
    switch (version) {
        SENDME_V0 => {},
        SENDME_V1 => if (data_len < 20) return STATUS_BOUNDS,
        else => return STATUS_UNSUPPORTED,
    }
    out_view_u32[0] = version;
    out_view_u32[1] = 3;
    out_view_u32[2] = data_len;
    out_view_u32[3] = 3 + @as(u32, data_len);
    return STATUS_OK;
}

export fn tor_cell_sendme_v1_digest_matches(body: [*]const u8, body_len: u32, expected_digest20: [*]const u8) i32 {
    if (body_len < 23) return STATUS_BOUNDS;
    if (body[0] != SENDME_V1) return STATUS_UNSUPPORTED;
    if (get16be(body[1..3]) < 20) return STATUS_BOUNDS;
    var acc: u8 = 0;
    var i: usize = 0;
    while (i < 20) : (i += 1) acc |= body[3 + i] ^ expected_digest20[i];
    if (acc != 0) return STATUS_AUTH;
    return STATUS_OK;
}
ZIG

zig build-exe "$WORK_DIR/tor_cell_codec.zig" \
  -target wasm32-freestanding \
  -fno-entry \
  -rdynamic \
  -O ReleaseSmall \
  -femit-bin="$WORK_DIR/tor-cell-codec.wasm"

wasm2wat "$WORK_DIR/tor-cell-codec.wasm" -o "$OUT_DIR/tor-cell-codec.wat"
wat2wasm "$OUT_DIR/tor-cell-codec.wat" -o "$WORK_DIR/roundtrip.wasm"
wasm-validate "$WORK_DIR/roundtrip.wasm"
echo "wrote $OUT_DIR/tor-cell-codec.wat"
