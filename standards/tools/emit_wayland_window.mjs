#!/usr/bin/env bun
/**
 * Emit a standalone Wayland window ELF binary.
 *
 * Pipeline:
 *   1. Compile app/wayland-client.wat → WASM binary via wasm-tools parse
 *   2. Load edgerun.wasm (interpreter + JIT + wayland-patch-syscalls)
 *   3. Write Wayland WASM binary into interpreter memory
 *   4. Call compile_wasm_to_elf → produces ELF at ELF_OUT_BUF (0x400000)
 *   5. Read ELF from memory → write to disk as ./wayland-window.elf
 *
 * Usage: bun tools/emit_wayland_window.mjs [--output=./wayland-window.elf]
 */

import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

const ROOT = resolve(import.meta.dirname, '..');
const OUT = process.argv.find(a => a.startsWith('--output='))?.slice(9) || resolve(ROOT, 'wayland-window.elf');

// Step 1: compile wayland-client.wat → WASM binary
console.log('Compiling wayland-client.wat → WASM...');
const waylandWasmPath = resolve(ROOT, '/tmp/wayland-client.wasm');
try {
  execSync(`wasm-tools parse "${resolve(ROOT, 'app/wayland-client.wat')}" -o "${waylandWasmPath}"`, { stdio: 'pipe' });
} catch (e) {
  console.error('Failed to compile wayland-client.wat:', e.stderr?.toString() || e.message);
  process.exit(1);
}
const waylandBinary = readFileSync(waylandWasmPath);
console.log(`  Wayland WASM: ${waylandBinary.length} bytes`);

// Step 2: load edgerun.wasm
console.log('Loading edgerun.wasm...');
const edgerunPath = resolve(ROOT, 'edgerun.wasm');
if (existsSync(edgerunPath)) {
  console.log(`  Found edgerun.wasm at ${edgerunPath}`);
} else {
  console.error(`  edgerun.wasm not found. Run 'bun run build' first.`);
  process.exit(1);
}

const edgerunBinary = readFileSync(edgerunPath);
const mod = new WebAssembly.Module(edgerunBinary);
const instance = new WebAssembly.Instance(mod, {
  host: {
    sock_open: () => -1,
    sock_send: () => -1,
    sock_recv: () => -1,
    sock_close: () => {},
  },
  linux: {
    poll: () => -1,
    mmap: (addr, len, prot, flags, fd, off) => 0,
    munmap: () => 0,
    socket: () => -1,
    connect: () => -1,
    sendmsg: () => -1,
    memfd_create: () => -1,
    ftruncate: () => -1,
  },
});

const wasm = instance.exports;
const mem = wasm.memory;
const u8 = new Uint8Array(mem.buffer);

// Step 3: write Wayland WASM binary into interpreter memory (at a safe address)
const WASM_BASE = 0x300000;  // must not overlap with interpreter state
const WASM_LEN = waylandBinary.length;

if (WASM_LEN > 0x100000) {  // 1MB budget
  console.error(`Wayland WASM too large: ${WASM_LEN} bytes`);
  process.exit(1);
}

// Grow memory if needed
const pagesNeeded = Math.ceil((WASM_BASE + WASM_LEN) / 65536);
while ((mem.buffer.byteLength / 65536) < pagesNeeded) {
  wasm.memory.grow(1);
}

u8.set(waylandBinary, WASM_BASE);
console.log(`  Written ${WASM_LEN} bytes at 0x${WASM_BASE.toString(16)}`);

// Step 4: call compile_wasm_to_elf
if (typeof wasm.compile_wasm_to_elf !== 'function') {
  console.error('edgerun.wasm does not export compile_wasm_to_elf');
  console.error('Available exports:', Object.keys(wasm).filter(k => typeof wasm[k] === 'function').slice(0, 20).join(', '));
  process.exit(1);
}

console.log('Calling compile_wasm_to_elf...');
const result = wasm.compile_wasm_to_elf(WASM_BASE, WASM_LEN);

// compile_wasm_to_elf returns (elf_addr, elf_size) as two i32s
const elfAddr = result[0];
const elfSize = result[1];
console.log(`  ELF at 0x${elfAddr.toString(16)}, size ${elfSize}`);

if (elfAddr === 0 || elfSize === 0) {
  console.error('ELF compilation returned empty result');
  process.exit(1);
}

// Step 5: read ELF from memory and write to disk
const elfBytes = new Uint8Array(mem.buffer.slice(elfAddr, elfAddr + elfSize));
writeFileSync(OUT, elfBytes);
console.log(`✓ Written ${elfBytes.length} bytes → ${OUT}`);
console.log(`  Run: chmod +x ${OUT} && WAYLAND_DISPLAY=wayland-0 ${OUT}`);
