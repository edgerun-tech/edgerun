import { readFileSync, writeFileSync, readdirSync, existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, 'src');
const fragmentsDir = join(__dirname, 'fragments'); // proper modules output
const headerFile = join(__dirname, '..', 'runtime', 'module-header.wat');
const footerFile = join(__dirname, '..', 'runtime', 'module-footer.wat');
const outFile = join(__dirname, 'ui_framework.wat');

if (!existsSync(fragmentsDir)) mkdirSync(fragmentsDir, { recursive: true });

// ── 1. Read fragments ──────────────────────────────────────────────
const fragFiles = readdirSync(srcDir).filter(f => f.endsWith('.wat')).sort();
const fragments = {};
for (const f of fragFiles) {
  fragments[f] = readFileSync(join(srcDir, f), 'utf-8');
}

// ── 2. Parse: extract function defs (name + type) and call refs ─────
// Strip parameter names from a type string: (param $a f32) (param $b f32) (result f32)
// → (param f32 f32) (result f32)
function stripParamNames(type) {
  return type.replace(/\$(\w+)/g, '').replace(/  +/g, ' ');
}

function stripExportFromType(type) {
  return type.replace(/\(export\s+"[^"]*"\)\s*/g, '');
}

function parseFragment(text) {
  const funcDefs = {};     // name → { type, export: bool }
  const calls = new Set();   // function names called
  const globalRefs = new Set();
  const lines = text.split('\n');
  let inFunc = null, accType = '', depth = 0, inString = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Calls & global refs (check EVERY line, even inside functions)
    for (const m of line.matchAll(/call\s+\$(\w+)/g)) calls.add(m[1]);
    for (const m of line.matchAll(/global\.(?:get|set)\s+\$(\w+)/g)) globalRefs.add(m[1]);

    // Function start
    const defM = line.match(/^\s+\(func\s+\$?(\S+)/);
    if (defM) {
      const raw = defM[1];
      if (raw.startsWith('(')) { inFunc = '::anon' + i; continue; }
      inFunc = raw;
      // Type signature = everything on first line after func name
      accType = line.replace(/^\s+\(func\s+\$?\S+\s*/, '').trim();
      depth = 0; inString = false;
      for (const ch of line) {
        if (inString) { if (ch === '"') inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      if (depth === 0) {
        const hasExport = accType.includes('(export "');
        funcDefs[inFunc] = { type: stripParamNames(stripExportFromType(accType)), export: hasExport };
        inFunc = null;
      }
      continue;
    }

    if (inFunc) {
      for (const ch of line) {
        if (inString) { if (ch === '"') inString = false; }
        else { if (ch === '"') inString = true; else if (ch === '(') depth++; else if (ch === ')') depth--; }
      }
      if (depth === 0) {
        const hasExport = accType.includes('(export "') || line.includes('(export "');
        funcDefs[inFunc] = { type: stripParamNames(stripExportFromType(accType)), export: hasExport };
        inFunc = null;
      }
    }
  }
  return { funcDefs, calls, globalRefs };
}

const parsed = {};
for (const [f, text] of Object.entries(fragments)) parsed[f] = parseFragment(text);

// ── 3. Build master definition table ────────────────────────────────
const master = {}; // name → { fragment, type }
for (const [f, { funcDefs }] of Object.entries(parsed)) {
  for (const [name, info] of Object.entries(funcDefs)) master[name] = { fragment: f, type: info.type };
}

// ── 4. Determine exports (functions called from other fragments) ────
const neededAsExport = {}; // name → true
for (const [f, { calls }] of Object.entries(parsed)) {
  for (const name of calls) {
    if (master[name] && master[name].fragment !== f) neededAsExport[name] = true;
  }
}

// ── 5. Build annotated fragments ────────────────────────────────────
for (const f of fragFiles) {
  const { funcDefs, calls, globalRefs } = parsed[f];
  const text = fragments[f];
  const lines = text.split('\n');

  // Collect imports needed by this fragment
  const imports = [];
  for (const name of calls) {
    if (!master[name]) continue;   // local func (e.g. from within same fragment)
    if (master[name].fragment === f) continue; // defined here
    const type = master[name].type;
    imports.push(`  (import "ui" "${name}" (func $${name} ${type}))`);
  }

  // Sort imports by name for deterministic output
  imports.sort();

  // Build header
  const header = [];
  if (f === '00_globals.wat') {
    header.push('(module');
    header.push(`  ;; Global declarations and string data for all UI fragments`);
  } else {
    header.push('(module');
    if (imports.length > 0) {
      header.push(`  ;; Imports from other UI fragments`);
      header.push(...imports);
    }
  }

  // Build body — insert (export "name") on function defs that need it
  const body = [];
  let skipBlank = false;
  for (const line of lines) {
    // Add export annotation to function definitions that need cross-fragment exports
    const defM = line.match(/^(\s+)\(func\s+\$(\w+)/);
    if (defM) {
      const name = defM[2];
      if (neededAsExport[name] && !line.includes('(export "')) {
        // Insert export after the function name
        body.push(line.replace(/^(\s+\(func\s+\$\w+)/, '$1 (export "' + name + '")'));
        continue;
      }
    }
    body.push(line);
  }

  // Build footer
  if (f === '00_globals.wat') {
    // Only add footer
  }

  const moduleText = header.join('\n') + '\n' + body.join('\n') + '\n)\n';

  writeFileSync(join(fragmentsDir, f), moduleText);
  const exportCount = Object.entries(funcDefs).filter(([n]) => neededAsExport[n]).length;
  console.log(`${f}: ${imports.length} imports, ${exportCount} exports, ${Object.keys(funcDefs).length} funcs`);
}

// ── 6. Link: produce final module ────────────────────────────────────
const header = readFileSync(headerFile, 'utf-8');
const footer = readFileSync(footerFile, 'utf-8');

// Collect all unique host imports
const hostImports = new Set();
for (const [f, { calls }] of Object.entries(parsed)) {
  // Host imports are defined in the header — fragments don't directly reference them
  // So no additional host imports needed
}

// Build the linked output
const partLines = [header.trimEnd()];
for (const f of fragFiles) {
  const fragText = readFileSync(join(fragmentsDir, f), 'utf-8');
  // Strip module wrapper: remove (module header and trailing ), keep body
  const modLines = fragText.split('\n');
  // Find where body starts (first line that isn't (module, import, or comment-preamble)
  let bodyStart = 0;
  for (let i = 0; i < modLines.length; i++) {
    const l = modLines[i];
    if (l.match(/^\s*$|^\(module|^\)\s*$|^\s+;;/)) continue;
    if (l.startsWith('  (import "ui"')) continue;
    if (l.startsWith('  (import "host"') || l.startsWith('  (import "linux"')) continue;
    bodyStart = i;
    break;
  }
  // Find where body ends (last top-level paren that's a standalone ))
  let bodyEnd = modLines.length;
  for (let i = modLines.length - 1; i >= 0; i--) {
    if (modLines[i].match(/^\s*$|^\)\s*$/)) { bodyEnd = i; continue; }
    break;
  }
  // If bodyEnd is before the actual content, cap at the actual last ) that closes the module
  // Simple: just take everything from bodyStart to last non-whitespace before the closing )
  let realEnd = bodyEnd;
  for (let i = bodyEnd - 1; i > bodyStart; i--) {
    if (!modLines[i].match(/^\s*$/)) { realEnd = i + 1; break; }
  }

  const bodyLines = modLines.slice(bodyStart, realEnd);
  partLines.push(bodyLines.join('\n').trimEnd());
}
partLines.push(footer.trim());

const outContent = partLines.join('\n');

writeFileSync(outFile, outContent + '\n');

const funcCount = (outContent.match(/\(func\s+\$/g) || []).length;

console.log(`\nWrote ${outFile}`);
console.log(`  Header: ${header.split('\n').length} lines`);
console.log(`  Fragments: ${fragFiles.length}`);
console.log(`  Footer: 1 line`);

const totalLines = readFileSync(outFile, 'utf-8').split('\n').length;
console.log(`  Total: ${totalLines} lines, ~${funcCount} functions`);
