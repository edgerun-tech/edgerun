import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parseFragment } from './parse-fragment.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, 'src');
const fragments = readdirSync(srcDir).filter(f => f.endsWith('.wat')).sort();

const allDefs = {}, allCalls = {}, allGlobals = {}, allGlobalRefs = {};
for (const f of fragments) {
  const t = readFileSync(join(srcDir, f), 'utf-8');
  const r = parseFragment(t);
  allDefs[f] = r.funcDefs; allCalls[f] = r.calls;
  allGlobals[f] = r.globalsDefs; allGlobalRefs[f] = r.globalRefs;
}

const masterDefs = {};
for (const [f, defs] of Object.entries(allDefs))
  for (const [n, info] of Object.entries(defs)) masterDefs[n] = { fragment: f, type: info.type };

const masterGlobals = {};
for (const [f, gs] of Object.entries(allGlobals))
  for (const g of gs) masterGlobals[g] = f;

console.log('\n=== Cross-fragment function call dependencies ===\n');
let totalCross = 0;
for (const f of fragments) {
  const externalCalls = [...allCalls[f]].filter(n => masterDefs[n] && masterDefs[n].fragment !== f).sort();
  if (externalCalls.length === 0) continue;
  totalCross += externalCalls.length;
  const byTarget = {};
  for (const n of externalCalls) {
    const t = masterDefs[n].fragment;
    byTarget[t] = byTarget[t] || [];
    byTarget[t].push(n);
  }
  console.log(`  ${f}`);
  for (const [t, names] of Object.entries(byTarget).sort())
    console.log(`    → ${t} (${names.length}): ${names.slice(0, 5).join(', ')}${names.length > 5 ? '...' : ''}`);
}
console.log(`\nTotal cross-fragment calls: ${totalCross}`);

console.log('\n=== Cross-fragment global dependencies ===\n');
for (const f of fragments) {
  const ext = [...allGlobalRefs[f]].filter(n => masterGlobals[n] && masterGlobals[n] !== f).sort();
  if (ext.length === 0) continue;
  const byTarget = {};
  for (const n of ext) {
    const t = masterGlobals[n];
    byTarget[t] = byTarget[t] || []; byTarget[t].push(n);
  }
  console.log(`  ${f} → external globals: ${Object.keys(byTarget).join(', ')}`);
}

console.log(`\nTotal functions: ${Object.keys(masterDefs).length}, globals: ${Object.keys(masterGlobals).length}`);
