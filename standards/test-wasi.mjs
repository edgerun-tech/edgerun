import { readFileSync, writeFileSync } from 'fs';

const wasm = readFileSync(new URL('./edgerun.wasm', import.meta.url));
const mod = await WebAssembly.compile(wasm);
const inst = await WebAssembly.instantiate(mod, {});
const { memory, load, compile_to_elf, compile_all_to_elf_x86_64 } = inst.exports;
const view = new DataView(memory.buffer);

function leb128u(n) {
  const b = [];
  do { b.push(n & 0x7F | 0x80); n >>>= 7; } while (n > 0);
  b[b.length-1] &= 0x7F;
  return b;
}
function vec(ary) { return [...leb128u(ary.length), ...ary]; }
function bytes(s) { return [...Buffer.from(s, 'utf8')]; }
function section(id, body) { return [id, ...vec(body)]; }
function functype(params, results) {
  return [0x60, ...leb128u(params.length), ...params, ...leb128u(results.length), ...results];
}

const MOD = bytes('wasi_snapshot_preview1');

const typeSec = section(1, [
  3,
  ...functype([0x7F, 0x7F, 0x7F, 0x7F], [0x7F]),
  ...functype([0x7F, 0x7F], [0x7F]),
  ...functype([], []),
]);

const importSec = section(2, [
  5,
  ...vec(MOD), ...vec(bytes('fd_write')),       0x00, ...leb128u(0),
  ...vec(MOD), ...vec(bytes('fd_read')),        0x00, ...leb128u(0),
  ...vec(MOD), ...vec(bytes('proc_exit')),      0x00, ...leb128u(0),
  ...vec(MOD), ...vec(bytes('args_sizes_get')), 0x00, ...leb128u(1),
  ...vec(MOD), ...vec(bytes('args_get')),       0x00, ...leb128u(1),
]);

const funcSec = section(3, [
  1,
  2,
]);

const exportSec = section(7, [
  1,
  ...vec(bytes('_start')), 0x00, ...leb128u(5),
]);

const codeSec = section(10, [
  1,
  2, 0, 0x0B,
]);

const wasmBytes = new Uint8Array([
  0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
  ...typeSec, ...importSec, ...funcSec, ...exportSec, ...codeSec,
]);

try { await WebAssembly.compile(wasmBytes); } catch (e) { console.log('Invalid:', e.message); process.exit(1); }

new Uint8Array(memory.buffer).set(wasmBytes, 0x800000);
const err = load(0x800000, wasmBytes.length);
console.log('load:', err);

for (let i = 0; i < 5; i++) {
  const sysno = view.getInt32(0x90000 + i * 4, true);
  const names = ['fd_write','fd_read','proc_exit','args_sizes_get','args_get'];
  const expect = [20, 0, 60, -1, -2];
  const ok = sysno === expect[i];
  console.log(`  sysno[${i}] ${names[i]} = ${sysno} ${ok ? '✓' : '✗'}`);
}

// Try OLD single-function compile
console.log('\n-- Trying compile_to_elf(func=5, backend=0) --');
try {
  const [a1, s1] = compile_to_elf(5, 0);
  console.log('  OK: ELF at 0x' + a1.toString(16) + ', size', s1);
} catch (e) {
  console.log('  ERROR:', e.message);
}

// Try NEW all-functions compile
console.log('\n-- Trying compile_all_to_elf_x86_64 --');
try {
  const [a2, s2] = compile_all_to_elf_x86_64();
  console.log('  OK: ELF at 0x' + a2.toString(16) + ', size', s2);
} catch (e) {
  console.log('  ERROR:', e.message);
}
