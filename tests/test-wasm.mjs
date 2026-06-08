import { readFileSync } from 'fs';
import { WASI } from 'wasi';

const wasi = new WASI({ version: 'preview1' });
const wasm = readFileSync('/tmp/edgerun-fresh.wasm');

const linuxStubs = {
  read: () => -1, write: () => -1, open: () => -1, close: () => -1,
  poll: () => -1, mmap: () => -1, munmap: () => 0, socket: () => -1,
  connect: () => -1, sendmsg: () => -1, memfd_create: () => -1, ftruncate: () => -1,
};

const { instance } = await WebAssembly.instantiate(wasm, {
  wasi_snapshot_preview1: wasi.wasiImport,
  linux: linuxStubs,
});
const e = instance.exports;

let passed = 0;
let failed = 0;

function g(v) { return (typeof v === 'object' && v !== null && 'value' in v) ? v.value : v; }
function assert(cond, msg) { if (cond) passed++; else { failed++; console.error(`  FAIL: ${msg}`); } }
function str2mem(ptr, s) { const buf = new Uint8Array(e.memory.buffer); for (let i = 0; i < s.length; i++) buf[ptr + i] = s.charCodeAt(i); }
function mem2bytes(ptr, len) { return new Uint8Array(e.memory.buffer.slice(ptr, ptr + len)); }

const i64_lo = v => Number(BigInt(v) & 0xffffffffn);
const i64_hi = v => Number(BigInt(v) >> 32n);

// Use WASM hex_encode_lower instead of JS reimplementation
const HEX_BUF = 0x7F000;
function hex(ptr, len) {
  e.hex_encode_lower(ptr, len, HEX_BUF, len * 2 + 1);
  return new TextDecoder().decode(new Uint8Array(e.memory.buffer, HEX_BUF, len * 2));
}

// ── 1. Pack/byte helpers ──
console.log('=== pack/byte helpers ===');
assert(e.pack(3, 42) === ((3n << 32n) | 42n), 'pack(3, 42)');
assert(e.pack_u16(0xAB, 0xCD) === 0xABCD, 'pack_u16');
assert(e.byte(0x12345678, 0) === 0x78, 'byte(0) lsb');
assert(e.byte(0x12345678, 3) === 0x12, 'byte(3) msb');
assert(e.has(0xF0, 0x10) !== 0, 'has true');
assert(e.has(0xF0, 0x08) === 0, 'has false');

// ── 2. Memory ops ──
console.log('=== memory ops ===');
const view = new Uint8Array(e.memory.buffer);
e.memset(0x60000, 0xAB, 100);
assert(view[0x60000] === 0xAB, 'memset byte 0');
assert(view[0x60063] === 0xAB, 'memset byte 99');
e.memcpy(0x60100, 0x60000, 100);
assert(view[0x60100] === 0xAB, 'memcpy');
assert(view[0x60163] === 0xAB, 'memcpy end');
e.store8(0x60200, 0, 0x42);
assert(e.load8_u(0x60200, 0) === 0x42, 'store8/load8_u');
str2mem(0x60300, 'hello\0world');
assert(e.strlen(0x60300) === 5, 'strlen("hello")');
assert(e.strlen(0x60306) === 5, 'strlen("world")');
str2mem(0x60400, 'hello'); str2mem(0x60500, 'hello'); str2mem(0x60600, 'world');
assert(e.string_eq(0x60400, 5, 0x60500, 5) === 1, 'string_eq equal');
assert(e.string_eq(0x60400, 5, 0x60500, 4) === 0, 'string_eq diff len');
assert(e.string_eq(0x60400, 5, 0x60600, 5) === 0, 'string_eq diff content');
str2mem(0x60700, 'hello world');
assert(e.starts_with(0x60700, 11, 0x60500, 5) === 1, 'starts_with true');
assert(e.starts_with(0x60700, 11, 0x60600, 5) === 0, 'starts_with false');

// ── 3. Protocol/version ──
console.log('=== protocol ===');
assert(e.proto_abi_version() === 2, 'proto_abi_version');
assert(e.proto_standard_id() === 0, 'proto_standard_id');
assert(e.simd_capabilities() === 1, 'simd_capabilities');

// ── 4. Status codes ──
console.log('=== status codes ===');
assert(g(e.STATUS_OK) === 0, 'STATUS_OK');
assert(g(e.STATUS_INPUT_SHORT) === 1, 'STATUS_INPUT_SHORT');
assert(g(e.STATUS_OUTPUT_SHORT) === 2, 'STATUS_OUTPUT_SHORT');
assert(g(e.STATUS_INVALID) === 3, 'STATUS_INVALID');
assert(g(e.STATUS_OVERFLOW) === 4, 'STATUS_OVERFLOW');
assert(g(e.STATUS_TRUNCATED) === 5, 'STATUS_TRUNCATED');
assert(g(e.STATUS_TOO_LONG) === 6, 'STATUS_TOO_LONG');
assert(g(e.STATUS_MORE) === 7, 'STATUS_MORE');
assert(g(e.STATUS_TIMEOUT) === 8, 'STATUS_TIMEOUT');

// ── 5. Epoch ──
console.log('=== epoch ===');
assert(g(e.epoch) === 0, 'epoch initially 0');
e.epoch.value = 42;
assert(g(e.epoch) === 42, 'epoch set to 42');
e.epoch.value = 0;

// ── 6. Buffer constants ──
console.log('=== buffer constants ===');
assert(g(e.BUF_SIZE_8K) === 8192, 'BUF_SIZE_8K');
assert(g(e.BUF_SIZE_64K) === 64000, 'BUF_SIZE_64K');
assert(g(e.PAGE_SIZE) === 65536, 'PAGE_SIZE');
assert(g(e.SCRATCH_BUF) === 0x3000, 'SCRATCH_BUF');
assert(g(e.SHA256_OUT_BUF) === 0x5000, 'SHA256_OUT_BUF');
assert(g(e.CRYPTO_INPUT_BUF) === 0x6000, 'CRYPTO_INPUT_BUF');
assert(g(e.HEAP_START) === 0x40000, 'HEAP_START');
assert(g(e.ELF_OUT_BUF) === 0x400000, 'ELF_OUT_BUF');
assert(g(e.JIT_CACHE) === 0x100000, 'JIT_CACHE');
assert(g(e.WORK_BUF) === 0x4000, 'WORK_BUF');

// ── 7. LEB128 ──
console.log('=== LEB128 ===');
view[0x61000] = 0xE5; view[0x61001] = 0x8E; view[0x61002] = 0x26;
const r_u = e.read_leb128_u(0x61000, 10, 0);
assert(i64_hi(r_u) === 0, 'leb128_u status OK');
assert((i64_lo(r_u) >> 8) === 624485, 'leb128_u value');

view[0x61010] = 0x9B; view[0x61011] = 0xF1; view[0x61012] = 0x59;
const r_s = e.read_leb128_s(0x61010, 10, 0);
assert(i64_hi(r_s) === 0, 'leb128_s status OK');

// ── 8. SIMD operations ──
console.log('=== SIMD ===');
e.memset_simd(0x63000, 0xEF, 32);
assert(view[0x63000] === 0xEF, 'memset_simd');
assert(view[0x6301F] === 0xEF, 'memset_simd end');
e.memset(0x63100, 0xCD, 64);
e.memcpy_simd(0x63200, 0x63100, 64);
assert(view[0x63200] === 0xCD, 'memcpy_simd');
assert(view[0x6323F] === 0xCD, 'memcpy_simd end');
str2mem(0x63300, 'hello world!!!');
str2mem(0x63400, 'hello world!!!');
str2mem(0x63500, 'hello world??');
assert(e.string_eq_simd(0x63300, 14, 0x63400, 14) === 1, 'string_eq_simd eq');
assert(e.string_eq_simd(0x63300, 14, 0x63500, 14) === 0, 'string_eq_simd diff');

// ── 9. memchr ──
console.log('=== memchr ===');
str2mem(0x64000, 'abcdefghijklmnopqrstuvwxyz');
assert(e.simd_memchr(0x64000, 26, 0x6d) === 0x6400C, 'simd_memchr finds m');
assert(e.simd_memchr(0x64000, 26, 0x7b) === -1, 'simd_memchr missing');
str2mem(0x64100, 'abacadae');
assert(e.simd_memrchr(0x64100, 8, 0x61) === 0x64106, 'simd_memrchr last a');

// ── 10. fnv1a_lower ──
console.log('=== fnv1a ===');
str2mem(0x65000, 'Hello');
assert(e.fnv1a_lower(0x65000, 5) !== 0, 'fnv1a nonzero');

// ── 11. Math ──
console.log('=== math ===');
assert(e.min(10, 20) === 10, 'min');
assert(e.max(10, 20) === 20, 'max');
assert(e.min_u(10, 20) === 10, 'min_u');
assert(e.max_u(10, 20) === 20, 'max_u');
assert(e.clamp(5, 10, 20) === 10, 'clamp low');
assert(e.clamp(15, 10, 20) === 15, 'clamp mid');
assert(e.clamp(25, 10, 20) === 20, 'clamp high');
assert(e.sat_sub(10, 3) === 7, 'sat_sub');
assert(e.align_up(13, 8) === 16, 'align_up');
assert(e.is_power_of_two(16) === 1, 'is_power_of_two');
assert(e.round_up(13, 8) === 16, 'round_up');
assert(e.bounds_check(100, 50, 10) === 1, 'bounds_check ok');
assert(e.bounds_check(100, 95, 10) === 0, 'bounds_check over');
assert(e.bounds_check(10, 0, 20) === 0, 'bounds_check need>len');

// ── 12. Pipeline infrastructure ──
console.log('=== pipeline infrastructure ===');
const pcap = 4096;
const ip = e.pipe_create(pcap);
const op = e.pipe_create(pcap);
assert(ip > 0, 'pipe_create input');
assert(op > 0, 'pipe_create output');

const SB = g(e.SCRATCH_BUF);
str2mem(SB, 'hello pipeline');
assert(e.pipe_write(ip, SB, 14) === 0, 'pipe_write 14 bytes (status OK=0)');
assert(e.pipe_available(ip) === 14, 'pipe_available == 14');
e.pipe_read(ip, SB, 14);
assert(e.pipe_available(ip) === 0, 'pipe drained');

// pipeline_create / set_stage / run with passthrough (STAGE_PASSTHROUGH=0)
const pl = e.pipeline_create(0x70000, 1);
assert(pl > 0, 'pipeline created');
e.pipeline_set_stage(pl, 0, 0, 0, 0); // type=0 (passthrough), no config
assert(e.pipeline_get_stage_type(pl, 0) === 0, 'stage type = 0 (passthrough)');

// Test with simple pipeline: write data, run through passthrough, read back
str2mem(SB, 'passthrough test');
e.pipe_write(ip, SB, 16);
// scratch arg is a memory buffer pointer (SCRATCH_BUF), not a pipe
assert(e.pipeline_run(pl, ip, op, SB, g(e.BUF_SIZE_8K)) === 0, 'pipeline_run passthrough');
assert(e.pipe_available(op) === 16, 'output has 16 bytes');
const nread = e.pipe_read(op, 0x71000, 16);
assert(nread === 16, `pipe_read returned ${nread} bytes`);
const outStr = Buffer.from(mem2bytes(0x71000, 16)).toString('ascii');
assert(outStr === 'passthrough test', `passthrough: got "${outStr}"`);

e.pipe_close(ip); e.pipe_close(op);

// ── 13. Messaging over pipes (msg_write / msg_read) ──
console.log('=== msg_write/msg_read ===');
const mp = e.pipe_create(pcap);
str2mem(SB, 'hello msg');
assert(e.msg_write(mp, SB, 9) === 0, 'msg_write 9 bytes');
const msg_len = e.msg_read(mp, 0x73000, 100);
assert(msg_len === 9, 'msg_read returned 9');
const msgStr = Buffer.from(mem2bytes(0x73004, 9)).toString('ascii');
assert(msgStr === 'hello msg', `msg: got "${msgStr}"`);
e.pipe_close(mp);

// ── 14. Character classification ──
console.log('=== char classification ===');
assert(e.char_class(0x30) & 1, 'is_digit(0)');
assert(e.char_class(0x39) & 1, 'is_digit(9)');
assert(!(e.char_class(0x41) & 1), 'is_digit(A) is false');
assert(e.char_class(0x41) & 2, 'is_upper(A)');
assert(e.char_class(0x5A) & 2, 'is_upper(Z)');
assert(!(e.char_class(0x61) & 2), 'is_upper(a) is false');
assert(e.char_class(0x61) & 4, 'is_lower(a)');
assert(e.char_class(0x7A) & 4, 'is_lower(z)');
assert(!(e.char_class(0x41) & 4), 'is_lower(A) is false');
assert(e.char_class(0x61) & 6, 'is_alpha(a)');
assert(e.char_class(0x41) & 6, 'is_alpha(A)');
assert(!(e.char_class(0x30) & 6), 'is_alpha(0) is false');
assert(e.char_class(0x30) & 16, 'is_hex(0)');
assert(e.char_class(0x41) & 16, 'is_hex(A)');
assert(e.char_class(0x66) & 16, 'is_hex(f)');
assert(!(e.char_class(0x67) & 16), 'is_hex(g) is false');
assert(e.char_class(0x20) & 32, 'is_ws(space)');
assert(e.char_class(0x09) & 32, 'is_ws(tab)');
assert(!(e.char_class(0x41) & 32), 'is_ws(A) is false');
assert(e.char_class(0x2E) & 64, 'is_scheme_byte(.)');
assert(!(e.char_class(0x2F) & 64), 'is_scheme_byte(/) is false');
assert(e.to_lower(0x41) === 0x61, 'to_lower(A)');
assert(e.to_lower(0x61) === 0x61, 'to_lower(a)');
assert(e.to_upper(0x61) === 0x41, 'to_upper(a)');
assert(e.to_upper(0x41) === 0x41, 'to_upper(A)');

// ── 15. SHA-256 ──
console.log('=== sha256 ===');
function sha256_hex(msg) {
  const buf = 0x5000;
  const out = 0x5100;
  str2mem(buf, msg);
  const result = e.sha256(buf, msg.length, out);
  const hi = i64_hi(result);
  assert(hi === 0, `sha256 status ${hi} for "${msg}"`);
  return hex(out, 32);
}
assert(sha256_hex('') === 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 'sha256("")');
assert(sha256_hex('abc') === 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad', 'sha256("abc")');
assert(sha256_hex('hello') === '2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824', 'sha256("hello")');

// ── 16. SHA-1 ──
console.log('=== sha1 ===');
function sha1_hex(msg) {
  const buf = 0x5200;
  const out = 0x5300;
  str2mem(buf, msg);
  const result = e.sha1(buf, msg.length, out);
  const hi = i64_hi(result);
  assert(hi === 0, `sha1 status ${hi} for "${msg}"`);
  return hex(out, 20);
}
assert(sha1_hex('') === 'da39a3ee5e6b4b0d3255bfef95601890afd80709', 'sha1("")');
assert(sha1_hex('abc') === 'a9993e364706816aba3e25717850c26c9cd0d89d', 'sha1("abc")');
assert(sha1_hex('hello') === 'aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d', 'sha1("hello")');

// ── 17. HMAC-SHA256 ──
console.log('=== hmac_sha256 ===');
const hmac_key = 0x5400;
const hmac_msg = 0x5500;
const hmac_out = 0x5600;
str2mem(hmac_key, 'key');
str2mem(hmac_msg, 'The quick brown fox jumps over the lazy dog');
const hmac_result = e.hmac_sha256(hmac_key, 3, hmac_msg, 43, hmac_out, 32);
assert(hmac_result === 0, 'hmac_sha256 status 0');
assert(hex(hmac_out, 32) === 'f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8', 'hmac_sha256(key, fox)');

// ── Report ──
console.log(`\n=== Results: ${passed} passed, ${failed} failed ===`);
process.exit(failed > 0 ? 1 : 0);
