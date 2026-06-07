#!/usr/bin/env bun
import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const PASS = '\x1b[32m';
const FAIL = '\x1b[31m';
const RST = '\x1b[0m';

let passed = 0, failed = 0;
function check(ok, msg) {
  console.log(`  ${ok ? PASS + 'PASS' : FAIL + 'FAIL'}${RST}  ${msg}`);
  if (ok) passed++; else failed++;
}

// ── Instantiate WASM ──
let wasm, mem, u8, u32;
try {
  const bin = readFileSync(resolve(ROOT, 'edgerun.wasm'));
  const mod = new WebAssembly.Module(bin);
  const instance = new WebAssembly.Instance(mod, {
    host: {
      sock_open: () => -1,
      sock_send: () => -1,
      sock_recv: () => -1,
      sock_close: () => {},
    },
  });
  wasm = instance.exports;
  mem = wasm.memory;
  u8 = new Uint8Array(mem.buffer);
  u32 = new Uint32Array(mem.buffer);
  check(true, 'edgerun.wasm instantiated');
} catch (e) {
  check(false, `instantiate: ${e.message}`);
  process.exit(1);
}

// ── Load a minimal WASM module ──
// Simple module: (func (export "add") (param i32 i32) (result i32)
//   local.get 0  local.get 1  i32.add)
const GUEST_ADDR = 0x200000;
const guestWasm = readFileSync('/tmp/test_add.wasm');
u8.set(guestWasm, GUEST_ADDR);

const loadResult = wasm.load(GUEST_ADDR, guestWasm.length);
check(loadResult === 0, `load WASM module: status=${loadResult}`);

// ── JIT compile function 0 and wrap in ELF ──
// compile_to_elf returns (start_address, total_size)
let startAddr, totalSize;
try {
  const result = wasm.compile_to_elf(0);
  startAddr = result[0];
  totalSize = result[1];
  check(true, `compile_to_elf(0): start=0x${startAddr.toString(16)}, size=${totalSize}`);
} catch (e) {
  // fallback: some engines return values differently
  try {
    startAddr = Number(wasm.compile_to_elf(0));
    // second value via stack hack
    totalSize = 0;
    check(false, `compile_to_elf multi-value: ${e.message}`);
  } catch(e2) {
    check(false, `compile_to_elf failed: ${e2.message}`);
    process.exit(1);
  }
}

// ── Read ELF from WASM memory ──
// ELF output is at ELF_OUT_BUF = 0x400000
const ELF_ADDR = 0x400000;
const elfBytes = Buffer.from(u8.slice(ELF_ADDR, ELF_ADDR + totalSize));

const elfPath = '/tmp/test_output.elf';
writeFileSync(elfPath, elfBytes);
check(elfBytes.length === totalSize, `wrote ${elfBytes.length} bytes to ${elfPath}`);

// ── Verify ELF magic ──
check(elfBytes[0] === 0x7f && elfBytes[1] === 0x45 && elfBytes[2] === 0x4c && elfBytes[3] === 0x46,
  `ELF magic: ${elfBytes[0].toString(16)} ${elfBytes[1].toString(16)} ${elfBytes[2].toString(16)} ${elfBytes[3].toString(16)}`);

// ── Verify ELF class (2 = ELF64) ──
const elfClass = elfBytes[4];
check(elfClass === 2, `ELF class: ${elfClass} (expect 2=ELF64)`);

// ── Verify ELF data encoding (1 = little-endian) ──
const elfData = elfBytes[5];
check(elfData === 1, `ELF data: ${elfData} (expect 1=little-endian)`);

// ── Verify ELF type (2 = ET_EXEC) ──
const elfType = elfBytes[16] | (elfBytes[17] << 8);
check(elfType === 2, `ELF type: ${elfType} (expect 2=ET_EXEC)`);

// ── Verify machine (0x3E = x86-64) ──
const machine = elfBytes[18] | (elfBytes[19] << 8);
check(machine === 0x3E, `ELF machine: 0x${machine.toString(16)} (expect 0x3E=x86-64)`);

// ── Check program headers ──
const phoff = elfBytes[32] | (elfBytes[33] << 8) | (elfBytes[34] << 16) | (elfBytes[35] << 24);
const phnum = elfBytes[56] | (elfBytes[57] << 8);
check(phnum >= 1, `program headers: ${phnum} at offset ${phoff}`);

// ── Try to make it executable and run ──
import { chmodSync } from 'fs';
import { spawnSync } from 'child_process';
try {
  chmodSync(elfPath, 0o755);
  // Try running it — it should be a statically linked ELF
  const result = spawnSync(elfPath, [], { timeout: 2000 });
  check(result.status !== null, `execution exit code: ${result.status}`);
  if (result.stdout.length) console.log(`    stdout: ${result.stdout.toString().trim()}`);
  if (result.stderr.length) console.log(`    stderr: ${result.stderr.toString().trim()}`);
} catch (e) {
  check(false, `execution error: ${e.message}`);
}

// ── Also try readelf or file to verify ──
import { execSync } from 'child_process';
try {
  const fileOut = execSync(`file ${elfPath}`, { encoding: 'utf-8' }).trim();
  console.log(`    file: ${fileOut}`);
  check(fileOut.includes('ELF'), 'file identifies as ELF');
} catch (e) {
  check(false, `file command: ${e.message}`);
}

try {
  const readelfOut = execSync(`readelf -h ${elfPath} 2>/dev/null || true`, { encoding: 'utf-8' }).trim();
  if (readelfOut) console.log(`    readelf:\n${readelfOut.split('\n').map(l => `      ${l}`).join('\n')}`);
  check(readelfOut.includes('ELF'), 'readelf confirms ELF');
} catch (e) {
  check(false, `readelf: ${e.message}`);
}

// ── Summary ──
const total = passed + failed;
console.log(`\n  ${failed === 0 ? PASS + 'All' : FAIL + failed + '/' + total}${RST} ${failed === 0 ? 'passed' : 'failed'}\n`);
process.exit(failed > 0 ? 1 : 0);
