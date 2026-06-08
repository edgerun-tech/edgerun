#!/usr/bin/env node
// WASM→ELF compiler — uses edgerun.wasm as JIT backend
import { jitCompile, jitCompileTo } from '../jit.mjs';
import { readFileSync, existsSync } from 'fs';

const args = process.argv.slice(2);
if (args.length < 1 || args[0] === '-h' || args[0] === '--help') {
  console.error(`Usage: wasm2elf <input.wasm> [-o <output.elf>]

Compiles a WASM binary to a Linux x86-64 ELF binary using the
built-in JIT compiler in edgerun.wasm.

The module must export a function named "main" or "_start".
`);
  process.exit(1);
}

const inputPath = args[0];
if (!existsSync(inputPath)) {
  console.error(`Error: input not found: ${inputPath}`);
  process.exit(1);
}

const oi = args.indexOf('-o');
const oi2 = args.indexOf('--output');
let outputPath;
if (oi !== -1 && oi + 1 < args.length) outputPath = args[oi + 1];
else if (oi2 !== -1 && oi2 + 1 < args.length) outputPath = args[oi2 + 1];
else outputPath = inputPath.replace(/\.wasm$/, '') + '.elf';

let wasmBytes = readFileSync(inputPath);
if (inputPath.endsWith('.wat')) {
  const { execSync } = require('child_process');
  const tmpWasm = '/tmp/wasm2elf-tmp.wasm';
  execSync(`wat2wasm "${inputPath}" -o "${tmpWasm}"`, { stdio: 'pipe' });
  wasmBytes = readFileSync(tmpWasm);
  outputPath = inputPath.replace(/\.wat$/, '') + '.elf';
}

const elf = jitCompileTo(outputPath, wasmBytes);
if (elf[0] === 0x7f && elf[1] === 0x45 && elf[2] === 0x4c && elf[3] === 0x46) {
  console.error(`✓ ${elf.length}-byte ELF → ${outputPath}`);
} else {
  console.error(`✗ bad ELF magic`);
  process.exit(1);
}
