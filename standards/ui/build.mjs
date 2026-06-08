import { readFileSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, dirname, relative } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, '..');

// Directories to scan for fragments (in order)
const scanDirs = [
  'ui/src',
  'runtime',
  'codec',
  'data',
  'math',
  'crypto',
  'system',
  'pipeline',
  'device',
  'net',
  'protocol',
  'lang',
  'app',
  'compiler',
  'compiler/gen',
];

// Files that must appear last (elem registries need all funcs in scope)
const lastFiles = new Set([
  join(rootDir, 'pipeline', 'stage-registry.wat'),
]);

// Collect all fragment files
const fragFiles = [];
const seen = new Set();

for (const dir of scanDirs) {
  const fullDir = join(rootDir, dir);
  if (!statSync(fullDir, { throwIfNoEntry: false })) continue;
  const files = readdirSync(fullDir).filter(f => f.endsWith('.wat')).sort();
  for (const f of files) {
    const fullPath = join(fullDir, f);
    if (lastFiles.has(fullPath)) continue; // handle last
    if (seen.has(fullPath)) continue;
    seen.add(fullPath);
    fragFiles.push(fullPath);
  }
}

// Add last files
for (const f of lastFiles) {
  if (statSync(f, { throwIfNoEntry: false })) {
    fragFiles.push(f);
  }
}

// Read header, inject memory, concatenate
const header = readFileSync(join(rootDir, 'runtime', 'module-header.wat'), 'utf-8').trimEnd();
const footer = readFileSync(join(rootDir, 'runtime', 'module-footer.wat'), 'utf-8').trim();

const headerLines = header.split('\n');
let lastImport = 0;
for (let i = 0; i < headerLines.length; i++) {
  if (headerLines[i].includes('(import')) lastImport = i;
}
headerLines.splice(lastImport + 1, 0, '', '  ;; ── Memory ────────────────────────────────────────────────', '  (memory (export "memory") 288)', '');

const parts = [headerLines.join('\n')];
for (const f of fragFiles) {
  const body = readFileSync(f, 'utf-8').trimEnd();
  parts.push(body);
}
parts.push(footer);

const wat = parts.join('\n');

const outDir = join(__dirname);
const outWat = join(outDir, 'er.wat');
const outWasm = join(outDir, 'er.wasm');

writeFileSync(outWat, wat + '\n');
console.log(`Wrote ${outWat} (${fragFiles.length} fragments)`);

execSync(`wat2wasm "${outWat}" -o "${outWasm}"`, { stdio: 'inherit' });
const wasmSize = statSync(outWasm).size;
console.log(`Wrote ${outWasm} (${(wasmSize / 1024).toFixed(0)} KB)`);
