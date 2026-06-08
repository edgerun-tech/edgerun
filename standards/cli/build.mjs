#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';
import { resolve, basename } from 'path';
import {
  resolveRoot, rootPath, fileExists, ensureDir
} from '../tools/build-lib.mjs';

const ROOT = resolveRoot();
const WASM2ELF = rootPath('compiler', 'tools', 'wasm2elf.js');

function showUsage() {
  const msg = `Usage: node cli/build.mjs <user-fragment.wat> [options]

Options:
  -o, --output <file>    Output ELF path (default: <name>.elf)
  --with-args            Include argument parsing (args_sizes_get / args_get)
  --with-memory          Include bump allocator and string utilities
  -h, --help             Show this help

Example:
  node cli/build.mjs examples/hello-app.wat -o hello.elf
  ./hello.elf
  `;
  console.log(msg);
  process.exit(0);
}

const args = process.argv.slice(2);
if (args.length < 1 || args.includes('-h') || args.includes('--help')) {
  showUsage();
}

const userPath = resolve(args[0]);
if (!fileExists(userPath)) {
  console.error(`Error: user fragment not found: ${userPath}`);
  process.exit(1);
}

const oi = args.indexOf('-o');
const oi2 = args.indexOf('--output');
let outputPath;
if (oi !== -1 && oi + 1 < args.length) outputPath = resolve(args[oi + 1]);
else if (oi2 !== -1 && oi2 + 1 < args.length) outputPath = resolve(args[oi2 + 1]);
else outputPath = resolve(basename(userPath).replace(/\.wat$/, '') + '.elf');

const withArgs = args.includes('--with-args');
const withMemory = args.includes('--with-memory');

// ── Library fragments (CLI-specific, not in build-lib) ──
const cliDir = rootPath('cli');
const LIBRARY = [
  resolve(cliDir, 'module-header.wat'),
  resolve(cliDir, 'core.wat'),
  resolve(cliDir, 'io.wat'),
];
if (withArgs) LIBRARY.push(resolve(cliDir, 'args.wat'));
if (withMemory) LIBRARY.push(resolve(cliDir, 'memory.wat'));
const FOOTER = resolve(cliDir, 'module-footer.wat');

let body = '';
for (const libPath of LIBRARY) {
  if (!fileExists(libPath)) {
    console.error(`Warning: library module not found: ${libPath}`);
    continue;
  }
  body += readFileSync(libPath, 'utf-8').trimEnd() + '\n\n';
}
body += `;; ── User code: ${userPath} ──\n`;
body += readFileSync(userPath, 'utf-8').trimEnd() + '\n\n';
body += readFileSync(FOOTER, 'utf-8').trimEnd() + '\n';

const tmpWat = resolve('/tmp', `cli-build-${process.pid}.wat`);
const tmpWasm = resolve('/tmp', `cli-build-${process.pid}.wasm`);
writeFileSync(tmpWat, body, 'utf-8');

try {
  execSync(`wat2wasm "${tmpWat}" -o "${tmpWasm}"`, { stdio: 'pipe' });
} catch (e) {
  console.error(`wat2wasm failed:`);
  console.error(e.stderr?.toString() || e.message);
  console.error(`Assembled WAT written to ${tmpWat} for debugging`);
  process.exit(1);
}

try {
  execSync(`node "${WASM2ELF}" "${tmpWasm}" -o "${outputPath}"`, { stdio: 'inherit' });
} catch (e) {
  console.error(`wasm2elf failed:`);
  console.error(e.stdout?.toString() || e.stderr?.toString() || e.message);
  process.exit(1);
}

try { execSync(`rm -f "${tmpWat}" "${tmpWasm}"`, { stdio: 'pipe' }); } catch {}

console.error(`✓ ${outputPath}`);
