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
// All functions are synchronous. compileWat calls load_wat/emit_wasm from
// edgerun.wasm. Small tool WATs are supported directly; the full runtime
// WAT is compiled via wasm-tools parse in build.mjs.

import { readFileSync, writeFileSync } from 'fs';

// ── WASM module (sync init) ────────────────────────────────────────────
let _wasm = null;
function getWasm() {
  if (_wasm) return _wasm;
  const paths = [
    new URL('../out/edgerun.wasm', import.meta.url),
    new URL('../edgerun.wasm', import.meta.url),
  ];
  let wasm;
  for (const p of paths) {
    try { wasm = readFileSync(p); break; } catch {}
  }
  if (!wasm) throw new Error('edgerun.wasm not found — place in project root (ln out/edgerun.wasm edgerun.wasm)');
  const mod = new WebAssembly.Module(wasm);
  const stubs = { read: ()=>-1, write: ()=>-1, open: ()=>-1, close: ()=>-1,
    poll: ()=>-1, mmap: ()=>-1, munmap: ()=>0, socket: ()=>-1,
    connect: ()=>-1, sendmsg: ()=>-1, memfd_create: ()=>-1, ftruncate: ()=>-1 };
  const imports = WebAssembly.Module.imports(mod);
  const importObj = {};
  for (const im of imports) {
    if (!importObj[im.module]) importObj[im.module] = {};
    if (im.module === 'wasi_snapshot_preview1') importObj[im.module][im.name] = stubs[im.name] || (() => -1);
    else importObj[im.module][im.name] = stubs[im.name] || (() => -1);
  }
  const inst = new WebAssembly.Instance(mod, importObj);
  const e = inst.exports;
  const u8 = new Uint8Array(e.memory.buffer);
  _wasm = { e, u8 };
  return _wasm;
}

// ── LEB128 helpers (WASM-backed via varint_encode_u64/varint_decode_u64) ──

export function leb128Size(n) {
  const { e, u8 } = getWasm();
  const v = BigInt(n);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  const r = e.varint_encode_u64(lo, hi, _BUF, _CAP);
  return Number(r & 0xffffffffn);
}

export function writeLEB128(buf, off, n) {
  const { e, u8 } = getWasm();
  const v = BigInt(n);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  const r = e.varint_encode_u64(lo, hi, _BUF, _CAP);
  const written = Number(r & 0xffffffffn);
  buf.set(u8.subarray(_BUF, _BUF + written), off);
  return off + written;
}

export function readLEB128(buf, off) {
  const { e, u8 } = getWasm();
  const COPY_LEN = Math.min(buf.length - off, 10);
  u8.set(buf.subarray(off, off + COPY_LEN), _BUF);
  const r = e.varint_decode_u64(_BUF, COPY_LEN, _BUF + 16);
  const status = Number(r >> 32n);
  const bytesRead = Number(r & 0xffffffffn);
  if (status) return [0, bytesRead];
  const dv = new DataView(u8.buffer);
  return [Number(dv.getBigUint64(_BUF + 16, true)), bytesRead];
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
// Uses edgerun.wasm's built-in load_wat + emit_wasm to compile WAT to WASM.
// load_wat supports: i32.const, i32.add, i32.sub, nop, drop, return,
// unreachable, local.get, param, result, export, func.

function preprocessWat(source) {
  // f32.const <float> → f32.const 0xHEX (raw IEEE 754 bits as hex token)
  source = source.replace(/(f32\.const)\s+([-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?|nan(?::0x[0-9a-fA-F]+)?|inf|[-+]inf|[-+]nan)/g,
    (m, op, num) => {
      try {
        const f32 = new Float32Array([parseFloat(num)]);
        const hex = new Uint32Array(f32.buffer)[0];
        return `${op} 0x${(hex >>> 0).toString(16)}`;
      } catch { return m; }
    }
  );
  // f64.const <float> → f64.const LO_HEX HI_HEX (two raw i32 tokens)
  source = source.replace(/(f64\.const)\s+([-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?|nan(?::0x[0-9a-fA-F]+)?|inf|[-+]inf|[-+]nan)/g,
    (m, op, num) => {
      try {
        const f64 = new Float64Array([parseFloat(num)]);
        const u32 = new Uint32Array(f64.buffer);
        return `${op} 0x${u32[0].toString(16)} 0x${u32[1].toString(16)}`;
      } catch { return m; }
    }
  );
  // v128.const <shape> vals... → v128.const HEX0 HEX1 HEX2 HEX3
  source = source.replace(/v128\.const\s+(i32x4|i16x8|i8x16|f32x4|i64x2|f64x2)\s+([^()\n]+)/g,
    (m, shape, vals) => {
      try {
        const parts = vals.trim().split(/\s+/).map(parseFloat);
        const buf = new ArrayBuffer(16);
        const v = new DataView(buf);
        switch (shape) {
          case 'i32x4': for (let i = 0; i < 4; i++) v.setInt32(i*4, parts[i]|0, true); break;
          case 'i16x8': for (let i = 0; i < 8; i++) v.setInt16(i*2, parts[i]|0, true); break;
          case 'i8x16': for (let i = 0; i < 16; i++) v.setInt8(i, parts[i]|0); break;
          case 'f32x4': for (let i = 0; i < 4; i++) v.setFloat32(i*4, parts[i], true); break;
          case 'i64x2': for (let i = 0; i < 2; i++) v.setBigInt64(i*8, BigInt(parts[i]), true); break;
          case 'f64x2': for (let i = 0; i < 2; i++) v.setFloat64(i*8, parts[i], true); break;
        }
        const u32 = new Uint32Array(buf);
        const hex = Array.from(u32).map(x => '0x' + (x >>> 0).toString(16)).join(' ');
        return `v128.const ${hex}`;
      } catch { return m; }
    }
  );
  return source;
}

export function compileWat(watSource) {
  const { e, u8 } = getWasm();
  const memPages = e.memory?.value ?? 2048;
  const memSize = memPages * 65536;
  const WAT_OFF = 0x500000;
  const WASM_OFF = 0xC00000;
  const WASM_CAP = 0x400000;

  watSource = preprocessWat(watSource);
  const enc = new TextEncoder().encode(watSource);
  const maxLen = memSize - WAT_OFF - 0x100000;
  if (enc.length > maxLen) throw new Error(`WAT source too large (${enc.length} > ${maxLen})`);
  u8.set(enc, WAT_OFF);
  const err = e.load_wat(WAT_OFF, enc.length);
  if (err !== 0) {
    const dv = new DataView(u8.buffer);
    const dbg = dv.getInt32(0x8C020, true);
    const typeCount = dv.getInt32(0x100, true);
    const funcCount = dv.getInt32(17680, true);
    const exportCount = dv.getInt32(38176, true);
    const globalCount = dv.getInt32(40232, true);
    const dataCount = dv.getInt32(43324, true);
    // OFF_WAT_PTR at 0x8C000, OFF_WAT_LEN at 0x8C008
    const watPtr = dv.getInt32(0x8C000, true);
    const watLen = dv.getInt32(0x8C008, true);
    // Read 60 bytes around position from error to understand context
    // Try to find position from OFF_SCRATCH0 (at 8) or scan near end
    const pos = dv.getInt32(8, true);  // OFF_SCRATCH0
    let ctx = '';
    if (watPtr && watLen && pos >= 0 && pos < watLen) {
      const start = Math.max(0, Math.min(pos - 30, watLen - 60));
      const end = Math.min(start + 60, watLen);
      ctx = new TextDecoder().decode(u8.slice(watPtr + start, watPtr + end));
      ctx = `pos=${pos} ctx="{${ctx.replace(/[\x00-\x1f]/g, '.')}}"`;
    }
    throw new Error(`load_wat failed: ${err} (${ctx || `dbg=0x${dbg.toString(16)}`} t=${typeCount} f=${funcCount} e=${exportCount} g=${globalCount} d=${dataCount})`);
  }
  const r = e.emit_wasm(WASM_OFF, WASM_CAP);
  const status = Number(r >> 32n);
  if (status) throw new Error(`emit_wasm failed: ${status}`);
  const written = Number(r & 0xffffffffn);
  return u8.slice(WASM_OFF, WASM_OFF + written);
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
