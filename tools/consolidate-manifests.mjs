#!/usr/bin/env bun
// Scans two build manifests, validates against actual filesystem,
// fixes stale paths, and writes a unified manifest.json.
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');

const STALE_PATHS = {
  'crypto/hash-djb2.wat':                 'codec/hash-djb2.wat',
  'crypto/checksum-crc32-bzip.wat':       'codec/checksum-crc32-bzip.wat',
  'crypto/checksum-inet.wat':             'codec/checksum-inet.wat',
  'crypto/convex-hull.wat':               'math/convex-hull.wat',
  'protocol/sfx-tables.wat':              'codec/sfx-tables.wat',
};

const DEAD_ENTRIES = new Set([
  'compiler/dispatch.wat',
  'compiler/dispatch-wrapper.wat',
  'compiler/compiler-arm32.wat',
  'compiler/compiler-aarch64.wat',
  'compiler/interpreter-core.wat',
]);

// Files only for the "standard" build (build.mjs)
const STANDARD_ONLY = new Set([
  'data/embedded-source.wat',
]);

// Files only for the "full" build (build_wat.mjs with all 3 backends)
const FULL_ONLY = new Set([
  'pipeline/ui-layout-stage.wat',
  'pipeline/ui-paint-stage.wat',
  'pipeline/ui-event-stage.wat',
  'pipeline/oauth-json-value-stage.wat',
  'pipeline/oauth-parse-http-response-stage.wat',
  'pipeline/stage-registry.wat',
  'pipeline/ws-parse-header-stage.wat',
  'pipeline/ws-write-frame-header-stage.wat',
  'protocol/dns.wat',
  'protocol/ws.wat',
  'app/oauth.wat',
]);

// Backend files — tagged for renaming in full build
const BACKEND_TAGS = {
  'compiler/base-x86-64.wat':              ['x86_64'],
  'compiler/emit-x86-64.wat':              ['x86_64'],
  'compiler/templates-x86-64.wat':         ['x86_64'],
  'compiler/simd-x86-64.wat':              ['x86_64'],
  'compiler/gen/jit-dispatch-x86-64.wat':  ['x86_64'],
  'compiler/emit-arm32.wat':               ['arm32'],
  'compiler/simd-arm32.wat':               ['arm32'],
  'compiler/templates-arm32.wat':          ['arm32'],
  'compiler/emit-aarch64.wat':             ['aarch64'],
  'compiler/simd-aarch64.wat':             ['aarch64'],
  'compiler/templates-aarch64.wat':        ['aarch64'],
};

// Hand-written pipeline infrastructure files (not auto-generated stages)
const PIPELINE_KEEP = new Set([
  'pipeline/pipe-core.wat',
  'pipeline/pipeline-core.wat',
  'pipeline/frame-core.wat',
  'pipeline/frame-pacer.wat',
  'pipeline/mux-core.wat',
  'pipeline/queue-stage.wat',
  'pipeline/buffer-stage.wat',
  'pipeline/cdc-stage.wat',
  'pipeline/stage-registry.wat',
  'pipeline/edgerun-parse-stage.wat',
  'pipeline/edgerun-exec-stage.wat',
  'pipeline/ws-parse-header-stage.wat',
  'pipeline/ws-write-frame-header-stage.wat',
  'pipeline/ui-layout-stage.wat',
  'pipeline/ui-paint-stage.wat',
  'pipeline/ui-event-stage.wat',
  'pipeline/oauth-json-value-stage.wat',
  'pipeline/oauth-parse-http-response-stage.wat',
]);

// Extract the MANIFEST array from a .mjs file
function extractManifest(filePath) {
  const text = readFileSync(filePath, 'utf-8');
  const start = text.indexOf('const MANIFEST = [');
  if (start === -1) throw new Error('MANIFEST not found in ' + filePath);
  let bracket = text.indexOf('[', start);
  let depth = 0;
  let inStr = false;
  let entries = [];
  let cur = '';
  for (let i = bracket; i < text.length; i++) {
    const ch = text[i];
    if (inStr) {
      if (ch === "'" && text[i-1] !== '\\') inStr = false;
      cur += ch;
      continue;
    }
    if (ch === "'") { inStr = true; cur += ch; continue; }
    if (ch === '[') { depth++; if (depth === 1) continue; }
    if (ch === ']') { depth--; if (depth === 0) break; }
    if (ch === ',' && depth === 1) {
      const trimmed = cur.trim();
      if (trimmed) {
        const match = trimmed.match(/['"]([^'"]+)['"]/);
        if (match) entries.push(match[1]);
      }
      cur = '';
      continue;
    }
    if (depth === 1) cur += ch;
  }
  const trimmed = cur.trim();
  if (trimmed) {
    const match = trimmed.match(/['"]([^'"]+)['"]/);
    if (match) entries.push(match[1]);
  }
  return entries;
}

function pathExists(relPath) {
  return existsSync(resolve(ROOT, relPath));
}

// Load registry.json to distinguish generated vs hand-written stages
let registry = [];
try {
  registry = JSON.parse(readFileSync(resolve(ROOT, 'pipeline/registry.json'), 'utf-8'));
} catch {}

const generatedStages = new Set();
const handWrittenStages = new Set();
for (const e of registry) {
  const file = 'pipeline/' + e.name.replace(/_/g, '-') + '-stage.wat';
  if (e.generated === true) generatedStages.add(file);
  else handWrittenStages.add(file);
}
// Also track known generated stage files that exist
for (const file of generatedStages) {
  if (pathExists(file)) generatedStages.delete(file); // exists as file, keep it
}

const buildManifest = extractManifest(resolve(ROOT, 'tools/build.mjs'));
const fullManifest = extractManifest(resolve(ROOT, 'tools/build_wat.mjs'));

// Merge and deduplicate
const allEntries = new Map();
for (const entry of buildManifest) allEntries.set(entry, 'standard');
for (const entry of fullManifest) {
  const existing = allEntries.get(entry);
  allEntries.set(entry, existing ? 'both' : 'full');
}

// Build consolidated manifest
const manifest = [];
const warnings = [];
const fixes = [];
let pipelineStagesNeeded = false;

for (const [entry, source] of allEntries) {
  let path = entry;

  if (STALE_PATHS[path]) {
    fixes.push(`${path} → ${STALE_PATHS[path]}`);
    path = STALE_PATHS[path];
  }

  if (DEAD_ENTRIES.has(path)) {
    warnings.push(`REMOVED dead entry: ${path}`);
    continue;
  }

  // Pipeline stage consolidation
  if (path.startsWith('pipeline/') && path.endsWith('-stage.wat') && !PIPELINE_KEEP.has(path)) {
    if (generatedStages.has(path)) {
      // Generated entry — replace with pipeline-stages.wat
      pipelineStagesNeeded = true;
      continue;
    }
    if (handWrittenStages.has(path) && !pathExists(path)) {
      warnings.push(`REMOVED missing hand-written stage: ${path}`);
      continue;
    }
    // Hand-written stage that exists on disk — keep it
  }

  let builds = ['standard', 'full'];
  if (STANDARD_ONLY.has(path)) builds = ['standard'];
  if (FULL_ONLY.has(path)) builds = ['full'];

  const backends = BACKEND_TAGS[path] || [];

  const exists = pathExists(path);
  if (!exists) {
    if (path === 'data/embedded-source.wat') {
      // Generated during build
    } else {
      warnings.push(`MISSING: ${path} (source: ${source})`);
    }
  }

  const entryObj = { path };
  if (builds.length === 1) entryObj.builds = builds;
  if (backends.length > 0) entryObj.backends = backends;
  if (!exists) entryObj.exists = false;
  manifest.push(entryObj);
}

// Add pipeline-stages.wat if any generated stages were referenced
if (pipelineStagesNeeded) {
  const exists = pathExists('pipeline/pipeline-stages.wat');
  const entryObj = { path: 'pipeline/pipeline-stages.wat' };
  if (!exists) entryObj.exists = false;
  manifest.push(entryObj);
}

// Add orphaned ARM backend files not in either manifest
for (const [path, backends] of Object.entries(BACKEND_TAGS)) {
  if (!manifest.find(e => e.path === path)) {
    const exists = pathExists(path);
    const entryObj = { path, backends, builds: ['full'] };
    if (!exists) entryObj.exists = false;
    manifest.push(entryObj);
  }
}

// Write manifest
const outPath = resolve(ROOT, 'manifest.json');
writeFileSync(outPath, JSON.stringify(manifest, null, 2) + '\n');

console.log(`manifest.json written (${manifest.length} entries)`);
console.log(`\nFixes applied:`);
for (const f of fixes) console.log(`  ${f}`);
console.log(`\nWarnings:`);
for (const w of warnings) console.log(`  ${w}`);

// Also strip (module) from template files that need it
const STRIP_MODULE = [
  'compiler/templates-arm32.wat',
  'compiler/templates-aarch64.wat',
];
for (const relPath of STRIP_MODULE) {
  const fullPath = resolve(ROOT, relPath);
  if (!existsSync(fullPath)) continue;
  let content = readFileSync(fullPath, 'utf-8');
  const lines = content.split('\n');
  // Find first non-blank, non-comment line
  let firstCodeLine = -1;
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trim();
    if (trimmed === '' || trimmed.startsWith(';;')) continue;
    firstCodeLine = i;
    break;
  }
  if (firstCodeLine >= 0 && lines[firstCodeLine].trim() === '(module') {
    // Remove that line and the matching closing )
    // Find last non-blank, non-comment line
    let lastCodeLine = -1;
    for (let i = lines.length - 1; i >= 0; i--) {
      const trimmed = lines[i].trim();
      if (trimmed === '' || trimmed.startsWith(';;')) continue;
      if (trimmed === ')') {
        lastCodeLine = i;
        break;
      }
    }
    if (lastCodeLine > firstCodeLine) {
      const newLines = lines.slice(firstCodeLine + 1, lastCodeLine);
      const newContent = newLines.join('\n').trimEnd() + '\n';
      writeFileSync(fullPath, newContent, 'utf-8');
      fixes.push(`STRIPPED (module) wrapper from ${relPath}`);
    }
  }
}
