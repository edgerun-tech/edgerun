import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, 'src');
const fragments = readdirSync(srcDir).filter(f => f.endsWith('.wat')).sort();

function parseFragment(text) {
  const funcDefs = {};
  const calls = new Set();
  const globalsDefs = new Set();
  const globalRefs = new Set();
  const lines = text.split('\n');
  let inFunc = null, funcType = '', depth = 0, inString = false;
  
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const defM = line.match(/^\s+\(func\s+\$?(\S+)/);
    if (defM) {
      const name = defM[1];
      if (name.startsWith('(')) { inFunc = '::anon' + i; }
      else {
        inFunc = name;
        funcType = line.replace(/^\s+\(func\s+\$?\S+/, '').replace(/\s*$/, '');
        let j = 0;
        for (const ch of line) {
          if (inString) { if (ch === '"') inString = false; }
          else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
        }
        if (depth === 0) { funcDefs[inFunc] = funcType; inFunc = null; }
      }
      continue;
    }
    if (inFunc) {
      for (const ch of line) {
        if (inString) { if (ch === '"') inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      if (depth === 0) {
        if (!inFunc.startsWith('::anon')) funcDefs[inFunc] = funcType;
        inFunc = null;
      }
    }
    for (const m of line.matchAll(/call\s+\$(\w+)/g)) calls.add(m[1]);
    for (const m of line.matchAll(/global\.get\s+\$(\w+)/g)) globalRefs.add(m[1]);
    for (const m of line.matchAll(/global\.set\s+\$(\w+)/g)) globalRefs.add(m[1]);
    const gd = line.match(/^\s+\(global\s+\$(\w+)/);
    if (gd) globalsDefs.add(gd[1]);
  }
  return { funcDefs, calls, globalsDefs, globalRefs };
}

const allDefs = {}, allCalls = {}, allGlobals = {}, allGlobalRefs = {};
for (const f of fragments) {
  const t = readFileSync(join(srcDir, f), 'utf-8');
  const r = parseFragment(t);
  allDefs[f] = r.funcDefs; allCalls[f] = r.calls;
  allGlobals[f] = r.globalsDefs; allGlobalRefs[f] = r.globalRefs;
}

const masterDefs = {};
for (const [f, defs] of Object.entries(allDefs))
  for (const [n, t] of Object.entries(defs)) masterDefs[n] = { fragment: f, type: t };

const masterGlobals = {};
for (const [f, gs] of Object.entries(allGlobals))
  for (const g of gs) masterGlobals[g] = f;

console.log('\n=== Cross-fragment function call dependencies ===\n');
let totalCross = 0;
const edgeCount = {};
for (const f of fragments) {
  const externalCalls = [...allCalls[f]].filter(n => masterDefs[n] && masterDefs[n].fragment !== f).sort();
  if (externalCalls.length === 0) continue;
  totalCross += externalCalls.length;
  const byTarget = {};
  for (const n of externalCalls) {
    const t = masterDefs[n].fragment;
    byTarget[t] = byTarget[t] || [];
    byTarget[t].push(n);
    const key = f + '→' + t;
    edgeCount[key] = (edgeCount[key] || 0) + 1;
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
