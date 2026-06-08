// EdgeRun UI Builder — scans fragments, concatenates, compiles
import { readFileSync, writeFileSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';
import {
  resolveRoot, rootPath, fileExists, compileWat, wrapModule
} from '../tools/build-lib.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = resolveRoot();

// Directories to scan for fragments (in order)
const scanDirs = [
  'ui/src', 'runtime', 'codec', 'data', 'math', 'crypto',
  'system', 'pipeline', 'device', 'net', 'protocol', 'lang',
  'compiler', 'compiler/gen',
];

// Files that must appear last (elem registries need all funcs in scope)
const lastFiles = new Set([
  rootPath('pipeline', 'stage-registry.wat'),
]);

// Collect all fragment files
const fragFiles = [];
const seen = new Set();

for (const dir of scanDirs) {
  const fullDir = rootPath(dir);
  if (!fileExists(fullDir)) continue;
  const files = readdirSync(fullDir).filter(f => f.endsWith('.wat')).sort();
  for (const f of files) {
    const fullPath = join(fullDir, f);
    if (lastFiles.has(fullPath)) continue;
    if (seen.has(fullPath)) continue;
    seen.add(fullPath);
    fragFiles.push(fullPath);
  }
}

for (const f of lastFiles) {
  if (fileExists(f)) fragFiles.push(f);
}

// Concatenate fragments and wrap in module
const bodyParts = [];
for (const f of fragFiles) {
  bodyParts.push(readFileSync(f, 'utf-8').trimEnd());
}
const wat = wrapModule(bodyParts.join('\n'), { memory: 288 });

const outDir = join(__dirname);
const outWat = join(outDir, 'er.wat');
const outWasm = join(outDir, 'er.wasm');

writeFileSync(outWat, wat + '\n');
console.log(`Wrote ${outWat} (${fragFiles.length} fragments)`);

compileWat(outWat, outWasm);
