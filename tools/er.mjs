#!/usr/bin/env bun
// er — EdgeRun CLI source viewer (thin JS bridge to er-tools.wasm)
import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const WASM_PATH = resolve(ROOT, 'out', 'edgerun.wasm');
const TOOLS_PATH = resolve(ROOT, 'out', 'tools', 'er-tools.wasm');

export function main() {
  const cmd = process.argv[2] || 'list';

  const edgerun = readFileSync(WASM_PATH);
  const toolsWasm = readFileSync(TOOLS_PATH);

  const mem = new WebAssembly.Memory({ initial: 1024 }); // 64 MB
  const tools = new WebAssembly.Instance(new WebAssembly.Module(toolsWasm), {
    host: { memory: mem },
  });

  const exports = tools.exports;
  const buf = new Uint8Array(mem.buffer);

  // Copy edgerun.wasm into shared memory at offset 0x1000 (leave low pages free)
  const WASM_ADDR = 0x1000;
  buf.set(edgerun, WASM_ADDR);

  const count = exports.init(WASM_ADDR, edgerun.length);
  if (count < 0) {
    console.error('Failed to initialize: no source section found');
    process.exit(1);
  }

  const OUT_ADDR = 0x400000; // 4 MB — should be plenty for output
  const OUT_MAX = 0x400000;

  if (cmd === 'list') {
    const len = exports.list_files(OUT_ADDR, OUT_MAX);
    process.stdout.write(new TextDecoder().decode(buf.slice(OUT_ADDR, OUT_ADDR + len)));
  } else if (cmd === 'cat') {
    const name = process.argv[3];
    if (!name) { console.error('Usage: er cat <file-path>'); process.exit(1); }
    const nameBytes = new TextEncoder().encode(name);
    // Find space after ARENA; use a temp area at 0x200000
    const NAME_ADDR = 0x200000;
    buf.set(nameBytes, NAME_ADDR);
    const len = exports.cat_file(NAME_ADDR, nameBytes.length, OUT_ADDR, OUT_MAX);
    if (len < 0) { console.error('File not found:', name); process.exit(1); }
    process.stdout.write(new TextDecoder().decode(buf.slice(OUT_ADDR, OUT_ADDR + len)));
  } else {
    console.error('Unknown command:', cmd);
    console.error('Usage: er [list|cat <path>]');
    process.exit(1);
  }
}

main();
