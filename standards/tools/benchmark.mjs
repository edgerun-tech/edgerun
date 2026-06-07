#!/usr/bin/env bun
/**
 * EdgeRun Benchmark Suite
 *
 * Compares WASM interpreter, JIT compile, and native JS performance.
 *
 * Usage: bun tools/benchmark.mjs
 */

import { readFileSync } from 'fs';
import { resolve } from 'path';
import { createHash } from 'crypto';

const ROOT = resolve(import.meta.dirname, '..');

const BIN_BUF  = 0x600000;
const ARGS_BUF = 0x640000;
const SHA_OUT  = 0x680000;

const BOLD = '\x1b[1m', DIM = '\x1b[2m', RST = '\x1b[0m';
const GRN  = '\x1b[32m', YLW = '\x1b[33m', RED = '\x1b[31m';

let wasm, u8, u32, mem;
try {
  const bin = readFileSync(resolve(ROOT, 'edgerun.wasm'));
  wasm = new WebAssembly.Instance(new WebAssembly.Module(bin), {
    host: { sock_open: () => -1, sock_send: () => -1, sock_recv: () => -1, sock_close: () => {} },
    linux: { poll: () => -1, mmap: () => 0, munmap: () => 0, socket: () => -1, connect: () => -1, sendmsg: () => -1, memfd_create: () => -1, ftruncate: () => -1 },
  }).exports;
  mem = wasm.memory;
  u8 = new Uint8Array(mem.buffer);
  u32 = new Uint32Array(mem.buffer);
} catch (e) {
  console.error(`FAIL: ${e.message}`);
  process.exit(1);
}

const fmt = (ns) => {
  if (ns < 1000) return `${ns.toFixed(2)} ns`;
  if (ns < 1e6)  return `${(ns / 1000).toFixed(2)} µs`;
  return `${(ns / 1e6).toFixed(3)} ms`;
};

function pad(s, n) { return (s + '').padEnd(n); }

function writeBytes(addr, arr) { for (let i = 0; i < arr.length; i++) u8[addr + i] = arr[i]; }

function writeArgs(args) { for (let i = 0; i < args.length; i++) u32[(ARGS_BUF >>> 2) + i] = args[i]; }

function time(fn, n) {
  const t0 = performance.now();
  for (let i = 0; i < n; i++) fn();
  return ((performance.now() - t0) * 1e6) / n;
}

function warmup(fn, n) { for (let i = 0; i < n; i++) fn(); }

// ── WASM binary module builder ──
function wasmMod(pCount, rCount, bodyBytes) {
  const ps = []; for (let i = 0; i < pCount; i++) ps.push(0x7F);
  const rs = []; for (let i = 0; i < rCount; i++) rs.push(0x7F);
  // type section
  const tc = [0x60, pCount, ...ps, rCount, ...rs];
  const ts = [0x01, tc.length, ...tc];
  // func section
  const fs = [0x03, 0x02, 0x01, 0x00];
  // export section: export "f" as func 0
  const es = [0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00];
  // code section: vec of function bodies
  const body = [...bodyBytes, 0x0B];
  const cc = [0x01, body.length, ...body];
  const cs = [0x0A, cc.length, ...cc];
  const header = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
  return new Uint8Array([...header, ...ts, ...fs, ...es, ...cs]);
}

// ── Workloads ──

const WORKLOADS = [
  {
    name: 'nop',
    desc: 'empty function (call overhead)',
    bin: wasmMod(0, 0, [0x00]), // body: 0 locals, 0x0B(end)
    args: [],
    jsFn: () => {},
    iter: 50000,
  },
  {
    name: 'id(42)',
    desc: 'param pass + return',
    bin: wasmMod(1, 1, [0x00, 0x20, 0x00]),
    args: [42],
    jsFn: (x) => x,
    jsArgs: [42],
    iter: 50000,
  },
  {
    name: 'add(3,7)',
    desc: 'i32 arithmetic',
    bin: wasmMod(2, 1, [0x00, 0x20, 0x00, 0x20, 0x01, 0x6A]),
    args: [3, 7],
    jsFn: (a, b) => a + b,
    jsArgs: [3, 7],
    iter: 50000,
  },
  {
    name: 'fib(25)',
    desc: 'recursive fibonacci',
    bin: wasmMod(1, 1, [
      0x00,                // 0 locals
      0x20, 0x00,          // local.get 0
      0x41, 0x01,          // i32.const 1
      0x4C,                // i32.le_s
      0x04, 0x7F,          // if (result i32)
        0x20, 0x00,        //   local.get 0
        0x0F,              //   return
      0x05,                // else
        0x20, 0x00,        //   local.get 0
        0x41, 0x01,        //   i32.const 1
        0x6B,              //   i32.sub
        0x10, 0x00,        //   call 0
        0x20, 0x00,        //   local.get 0
        0x41, 0x02,        //   i32.const 2
        0x6B,              //   i32.sub
        0x10, 0x00,        //   call 0
        0x6A,              //   i32.add
      0x0B,                // end if
    ]),
    args: [25],
    jsFn: (n) => { const f = (x) => x <= 1 ? x : f(x-1) + f(x-2); return f(n); },
    jsArgs: [25],
    iter: 500,
  },
  {
    name: 'sum(100K)',
    desc: 'loop sum 0..99999',
    bin: wasmMod(1, 1, [
      0x01, 0x02, 0x7F,    // 2 i32 locals ($s=1, $i=2; param $n=0)
      // block $exit / loop $loop pattern
      0x02, 0x40,          // block (void)
        0x03, 0x40,        // loop (void)
          0x20, 0x02,      //   local.get $i
          0x20, 0x00,      //   local.get $n
          0x4E,            //   i32.ge_s
          0x0D, 0x01,      //   br_if $exit (label 1)
          0x20, 0x01,      //   local.get $s
          0x20, 0x02,      //   local.get $i
          0x6A,            //   i32.add
          0x21, 0x01,      //   local.set $s
          0x20, 0x02,      //   local.get $i
          0x41, 0x01,      //   i32.const 1
          0x6A,            //   i32.add
          0x21, 0x02,      //   local.set $i
          0x0C, 0x00,      //   br $loop (label 0)
        0x0B,              // end loop
      0x0B,                // end block
      0x20, 0x01,          // local.get $s (result)
    ]),
    args: [100000],
    jsFn: (n) => { let s = 0; for (let i = 0; i < n; i++) s += i; return s; },
    jsArgs: [100000],
    iter: 20,
  },
  {
    name: 'sha256(64B)',
    desc: 'SHA-256 (native export vs Node.js crypto)',
    isDirect: true,
    directFn: () => {
      const a = 0x6C0000;
      for (let i = 0; i < 64; i++) u8[a + i] = 0x41;
      wasm.sha256(a, 64, SHA_OUT);
    },
    jsFn: () => createHash('sha256').update('A'.repeat(64)).digest(),
    iter: 1000,
  },
];

// ── Run ──

console.log(`\n${BOLD}╔══════════════════════════════════════════════════════════════╗${RST}`);
console.log(`${BOLD}║        EdgeRun WASM Benchmark Suite                         ║${RST}`);
console.log(`${BOLD}╚══════════════════════════════════════════════════════════════╝${RST}`);
console.log(`  ${DIM}Platform: ${process.arch} | Node ${process.version} | ${(mem.buffer.byteLength / 1024).toFixed(0)}KB memory${RST}\n`);

for (const wl of WORKLOADS) {
  console.log(`${BOLD}── ${wl.name}${RST} ${DIM}— ${wl.desc}${RST}`);

  let interp_ns = NaN, jit_ns = NaN, jit_elf_ns = NaN, native_ns = NaN, direct_ns = NaN;

  if (!wl.isDirect) {
    // ── Interpreter ──
    const bin = wl.bin;
    writeBytes(BIN_BUF, bin);
    const ls = wasm.load(BIN_BUF, bin.length);
    if (ls !== 0) {
      console.log(`  ${RED}load: ${ls}${RST}`);
    } else {
      writeArgs(wl.args);
      warmup(() => wasm.call(wl.args.length > 0 ? 0 : 0, ARGS_BUF, wl.args.length), 10);
      interp_ns = time(() => wasm.call(0, ARGS_BUF, wl.args.length), wl.iter);
    }

    // ── JIT compile (x86-64) ──
    const bin2 = wl.bin;
    writeBytes(BIN_BUF, bin2);
    const ls2 = wasm.load(BIN_BUF, bin2.length);
    if (ls2 !== 0) {
      console.log(`  ${RED}JIT load: ${ls2}${RST}`);
    } else {
      warmup(() => wasm.jit_compile(0, 0), 3);
      jit_ns = time(() => wasm.jit_compile(0, 0), Math.min(wl.iter, 100));
      warmup(() => wasm.compile_to_elf(0, 0), 3);
      jit_elf_ns = time(() => wasm.compile_to_elf(0, 0), Math.min(wl.iter, 100));
    }
  }

  // ── Direct export ──
  if (wl.isDirect) {
    warmup(wl.directFn, 10);
    direct_ns = time(wl.directFn, wl.iter);
  }

  // ── Native JS ──
  const nativeFn = wl.jsArgs ? () => wl.jsFn(...wl.jsArgs) : wl.jsFn;
  warmup(nativeFn, 1000);
  native_ns = time(nativeFn, wl.iter * 100);

  // ── Results ──
  const speed = (v) => {
    if (v == null || isNaN(v) || v <= 0) return '  —';
    const r = native_ns / v;
    return r >= 1 ? ` ${GRN}${r.toFixed(1)}x${RST}` : ` ${YLW}${r.toFixed(3)}x${RST}`;
  };

  console.log(`  ${pad('Mode', 28)} ${pad('Time/op', 16)} ${pad('Ops/sec', 16)} vs Native`);
  console.log(`  ${DIM}${'─'.repeat(72)}${RST}`);
  if (!wl.isDirect) {
    console.log(`  ${pad('Interpreter (call)', 28)} ${pad(isNaN(interp_ns) ? 'ERR' : fmt(interp_ns), 16)} ${pad(isNaN(interp_ns) ? '-' : (1e9 / interp_ns).toFixed(0), 16)}${speed(interp_ns)}`);
    console.log(`  ${pad('JIT (jit_compile)', 28)} ${pad(isNaN(jit_ns) ? 'ERR' : fmt(jit_ns), 16)} ${pad(isNaN(jit_ns) ? '-' : (1e9 / jit_ns).toFixed(0), 16)}${speed(jit_ns)}`);
    console.log(`  ${pad('JIT (compile_to_elf)', 28)} ${pad(isNaN(jit_elf_ns) ? 'ERR' : fmt(jit_elf_ns), 16)} ${pad(isNaN(jit_elf_ns) ? '-' : (1e9 / jit_elf_ns).toFixed(0), 16)}${speed(jit_elf_ns)}`);
  }
  if (wl.isDirect) {
    console.log(`  ${pad('Direct WASM export', 28)} ${pad(isNaN(direct_ns) ? 'ERR' : fmt(direct_ns), 16)} ${pad(isNaN(direct_ns) ? '-' : (1e9 / direct_ns).toFixed(0), 16)}${speed(direct_ns)}`);
  }
  console.log(`  ${pad('Native JS', 28)} ${pad(fmt(native_ns), 16)} ${pad((1e9 / native_ns).toFixed(0), 16)} 1.000x`);
  console.log();
}

const wasmSize = (readFileSync(resolve(ROOT, 'edgerun.wasm')).length / 1024).toFixed(0);
console.log(`${BOLD}── Platform ──${RST}`);
console.log(`  Memory:   ${(mem.buffer.byteLength / 1024).toFixed(0)} KB`);
console.log(`  WASM:     ${wasmSize} KB`);
console.log(`  Node:     ${process.version} on ${process.arch}\n`);
