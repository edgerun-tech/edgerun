#!/usr/bin/env bun
import { readFileSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const BIN_BUF = 0x600000;

function wasmMod(pCount, rCount, bodyBytes) {
  const ps = []; for (let i = 0; i < pCount; i++) ps.push(0x7F);
  const rs = []; for (let i = 0; i < rCount; i++) rs.push(0x7F);
  const tc = [0x01, 0x60, pCount, ...ps, rCount, ...rs];
  const ts = [0x01, tc.length, ...tc];
  const fs = [0x03, 0x02, 0x01, 0x00];
  const es = [0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00];
  const body = [...bodyBytes, 0x0B];
  const cc = [0x01, body.length, ...body];
  const cs = [0x0A, cc.length, ...cc];
  const header = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
  return new Uint8Array([...header, ...ts, ...fs, ...es, ...cs]);
}

const bin = readFileSync(resolve(ROOT, 'edgerun.wasm'));
const wasm = new WebAssembly.Instance(new WebAssembly.Module(bin), {
  host: { sock_open: () => -1, sock_send: () => -1, sock_recv: () => -1, sock_close: () => {} },
  linux: { poll: () => -1, mmap: () => 0, munmap: () => 0, socket: () => -1, connect: () => -1, sendmsg: () => -1, memfd_create: () => -1, ftruncate: () => -1 },
}).exports;
const mem = wasm.memory;
const u32 = new Uint32Array(mem.buffer);

console.log('=== Initial state ===');
console.log(`mem[0] (JS_CODE_PTR): ${u32[0]}`);
console.log(`mem[28] (JS_LABEL_DEPTH): ${u32[7]}`);
console.log(`mem[0xA0000/4] (decoded ops @0xA0000): 0x${u32[0xA0000/4].toString(16)}`);

// Load minimal nop WASM binary
const nb = wasmMod(0, 0, [0x00]);
new Uint8Array(mem.buffer).set(nb, BIN_BUF);
const loadResult = wasm.load(BIN_BUF, nb.length);
console.log(`\n=== After load ===`);
console.log(`load result: ${loadResult}`);
console.log(`mem[0] (JS_CODE_PTR): ${u32[0]}`);
console.log(`mem[256] (TYPE_COUNT): ${u32[256/4]}`);
console.log(`mem[0xA0000/4] (decoded ops @0): 0x${u32[0xA0000/4].toString(16)}`);
console.log(`mem[0xA0020/4] (decoded ops @1): 0x${u32[0xA0020/4].toString(16)}`);

// Read code entry
const SZ_CODE = 64;
const OFF_CODE_BUF = 0x551C;
const ce = OFF_CODE_BUF;
console.log(`\n=== Code entry 0 ===`);
console.log(`body_offset: ${u32[(ce)/4]}`);
console.log(`body_len: ${u32[(ce+8)/4]}`);
console.log(`local_count: ${u32[(ce+16)/4]}`);
console.log(`decoded_start: ${u32[(ce+24)/4]}`);
console.log(`decoded_count: ${u32[(ce+32)/4]}`);

// Call through interpreter
console.log(`\n=== After call ===`);
const r = wasm.call(0, BIN_BUF, 0);
console.log(`call result: ${r}`);

// Now try jit_compile
console.log(`\n=== About to jit_compile ===`);
try {
  const jr = wasm.jit_compile(0, 0);
  console.log(`jit_compile result: ${jr}`);
} catch(e) {
  console.log(`jit_compile ERROR: ${e.message}`);
}
