#!/usr/bin/env bun
// er-codec — centralized encoding library backed by out/edgerun.wasm
//
// Functions:
//   lebEncode(value)         LEB128 encode a non-negative i32 (Uint8Array)
//   lebEncode64(value)       LEB128 encode any non-negative integer (Uint8Array)
//   leb128Size(n)            Number of bytes needed for LEB128 encoding
//   writeLEB128(buf,off,n)   JS-native LEB128 encode, returns new offset
//   readLEB128(buf,off)      JS-native LEB128 decode, returns [value, bytesRead]
//   endianWriteLE(value,width)  Little-endian encode (Uint8Array)
//   endianWriteBE(value,width)  Big-endian encode (Uint8Array)
//   endianReadLE(buf,off,width) Read little-endian (BigInt)
//   endianReadBE(buf,off,width) Read big-endian (BigInt)
//   compileWat(watSource)    Compile WAT to WASM binary (Uint8Array)
//
// All functions are synchronous. compileWat falls back to wat2wasm when
// load_wat doesn't support the instruction. Width is in bytes: 2,4,8.

import { readFileSync, writeFileSync, rmSync } from 'fs';
import { spawnSync } from 'child_process';

// ── WASM module (sync init) ────────────────────────────────────────────
let _wasm = null;
function getWasm() {
  if (_wasm) return _wasm;
  const wasm = readFileSync(new URL('../out/edgerun.wasm', import.meta.url));
  const mod = new WebAssembly.Module(wasm);
  const inst = new WebAssembly.Instance(mod);
  const e = inst.exports;
  const u8 = new Uint8Array(e.memory.buffer);
  _wasm = { e, u8 };
  return _wasm;
}

// ── LEB128 helpers (JS-native — no WASM call needed) ────────────────

export function leb128Size(n) {
  let s = 1;
  while (n >= 128) { s++; n >>>= 7; }
  return s;
}

export function writeLEB128(buf, off, n) {
  let pos = off;
  while (n >= 128) { buf[pos++] = (n & 127) | 128; n >>>= 7; }
  buf[pos++] = n;
  return pos;
}

export function readLEB128(buf, off) {
  let val = 0, shift = 0, pos = off;
  while (pos < buf.length) {
    const b = buf[pos++];
    val |= (b & 127) << shift;
    shift += 7;
    if (!(b & 128)) return [val, pos - off];
  }
  return [val, pos - off];
}

// ── LEB128 encoding (WASM-backed) ──────────────────────────────────

const _BUF = 16384;  // WORK_BUF
const _CAP = 8192;   // BUF_SIZE_8K

export function lebEncode64(value) {
  const { e, u8 } = getWasm();
  const v = BigInt(value);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  const r = e.varint_encode_u64(lo, hi, _BUF, _CAP);
  const status = Number(r >> 32n);
  if (status) throw new Error(`varint encode failed: ${status}`);
  const written = Number(r & 0xffffffffn);
  return u8.slice(_BUF, _BUF + written);
}

export function lebEncode(value) {
  return lebEncode64(BigInt(value >>> 0));
}

// ── Endian encoding ──────────────────────────────────────────────────

export function endianWriteLE(value, width) {
  const { e, u8 } = getWasm();
  const v = BigInt(value);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  const r = e.endian_write(lo, hi, width, 0, _BUF, _CAP);
  const status = Number(r >> 32n);
  if (status) throw new Error(`endian_write failed: ${status}`);
  const written = Number(r & 0xffffffffn);
  return u8.slice(_BUF, _BUF + written);
}

export function endianWriteBE(value, width) {
  const { e, u8 } = getWasm();
  const v = BigInt(value);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  const r = e.endian_write(lo, hi, width, 1, _BUF, _CAP);
  const status = Number(r >> 32n);
  if (status) throw new Error(`endian_write failed: ${status}`);
  const written = Number(r & 0xffffffffn);
  return u8.slice(_BUF, _BUF + written);
}

export function endianReadLE(buf, off, width) {
  const { e, u8 } = getWasm();
  const RESULT_OFF = 0x6000;
  u8.set(buf.subarray(off, off + width), _BUF);
  const status = e.endian_read(_BUF, width, 0, width, 0, 0, RESULT_OFF);
  if (status) throw new Error(`endian_read failed: ${status}`);
  const v = Number(u8[RESULT_OFF]) | (Number(u8[RESULT_OFF+1]) << 8)
          | (Number(u8[RESULT_OFF+2]) << 16) | (Number(u8[RESULT_OFF+3]) << 24);
  return BigInt(v);
}

export function endianReadBE(buf, off, width) {
  const { e, u8 } = getWasm();
  const RESULT_OFF = 0x6000;
  u8.set(buf.subarray(off, off + width), _BUF);
  const status = e.endian_read(_BUF, width, 0, width, 1, 0, RESULT_OFF);
  if (status) throw new Error(`endian_read failed: ${status}`);
  const v = Number(u8[RESULT_OFF]) | (Number(u8[RESULT_OFF+1]) << 8)
          | (Number(u8[RESULT_OFF+2]) << 16) | (Number(u8[RESULT_OFF+3]) << 24);
  return BigInt(v);
}

// ── WAT compilation ─────────────────────────────────────────────────
// load_wat only supports: i32.const, i32.add, i32.sub, nop, drop,
// return, unreachable, local.get, param, result, export, func.
// Falls back to wat2wasm (from wabt) when load_wat returns an error.

export function compileWat(watSource) {
  const { e, u8 } = getWasm();
  const WAT_OFF = 0x600000;
  const WASM_OFF = 0x700000;
  const WASM_CAP = 0x100000;

  const enc = new TextEncoder().encode(watSource);
  if (enc.length > 0x500000) throw new Error('WAT source too large');
  u8.set(enc, WAT_OFF);
  const err = e.load_wat(WAT_OFF, enc.length);
  if (err === 0) {
    const r = e.emit_wasm(WASM_OFF, WASM_CAP);
    const status = Number(r >> 32n);
    if (status) throw new Error(`emit_wasm failed: ${status}`);
    const written = Number(r & 0xffffffffn);
    return u8.slice(WASM_OFF, WASM_OFF + written);
  }

  // Fallback to wat2wasm
  const tmpWat = `/tmp/er-codec-${process.pid}.wat`;
  const tmpWasm = `/tmp/er-codec-${process.pid}.wasm`;
  writeFileSync(tmpWat, watSource, 'utf-8');
  const r = spawnSync('wat2wasm', [tmpWat, '-o', tmpWasm], { stdio: 'pipe' });
  try { rmSync(tmpWat); } catch {}
  if (r.status !== 0) {
    try { rmSync(tmpWasm); } catch {}
    throw new Error(`load_wat failed (${err}) and wat2wasm also failed: ${r.stderr?.toString()?.slice(0, 200)}`);
  }
  const wasm = readFileSync(tmpWasm);
  try { rmSync(tmpWasm); } catch {}
  return wasm;
}

// ── CLI ──
if (process.argv[1] === new URL(import.meta.url).pathname) {
  const [cmd, ...args] = process.argv.slice(2);
  switch (cmd) {
    case 'encode': {
      const v = BigInt(args[0]);
      process.stdout.write(lebEncode64(v));
      break;
    }
    case 'le': {
      const v = BigInt(args[0]);
      const w = parseInt(args[1] || '4');
      process.stdout.write(endianWriteLE(v, w));
      break;
    }
    case 'be': {
      const v = BigInt(args[0]);
      const w = parseInt(args[1] || '4');
      process.stdout.write(endianWriteBE(v, w));
      break;
    }
    case 'compile': {
      const input = args.find(a => !a.startsWith('-'));
      const outIdx = args.indexOf('-o');
      const output = outIdx >= 0 ? args[outIdx + 1] : input?.replace(/\.wat$/, '.wasm');
      if (!input || !output) { console.error('Usage: er-codec compile <input.wat> [-o output.wasm]'); process.exit(1); }
      const src = readFileSync(input, 'utf-8');
      const wasm = compileWat(src);
      writeFileSync(output, wasm);
      break;
    }
    default:
      console.error('Usage: er-codec.mjs <encode|le|be|compile> [...]');
      process.exit(1);
  }
}
