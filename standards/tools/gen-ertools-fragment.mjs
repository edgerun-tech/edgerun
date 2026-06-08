import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const src = readFileSync(resolve(ROOT, 'compiler/er-tools.wat'), 'utf-8');

const lines = src.split('\n');
const skip = new Set();

function parenDepth(line) {
  let d = 0;
  for (const ch of line) { if (ch === '(') d++; if (ch === ')') d--; }
  return d;
}

function skipFuncBody(startIdx) {
  skip.add(startIdx);
  let d = parenDepth(lines[startIdx]);
  let i = startIdx;
  while (d > 0 && i + 1 < lines.length) {
    i++;
    skip.add(i);
    d += parenDepth(lines[i]);
  }
  return i;
}

// Find module closing paren (last non-empty line)
let moduleCloseIdx = lines.length - 1;
while (moduleCloseIdx >= 0 && lines[moduleCloseIdx].trim() === '') moduleCloseIdx--;

// Identify line ranges (0-indexed)
// Init function: lines 43-55 in the file (func def at line 43, close paren at line 55)
// But the line numbers shift. Let me search for specific identifiers.
let initFuncLine = -1, initCloseLine = -1;
let parseSourceLine = -1, isSourceLine = -1, cstrLenLine = -1;

for (let i = 0; i < lines.length; i++) {
  const line = lines[i];
  if (line.includes('(export "init")')) initFuncLine = i;
  if (line.match(/^\s*\(func \$parse_source_section/)) parseSourceLine = i;
  if (line.match(/^\s*\(func \$is_source_section/)) isSourceLine = i;
  if (line.match(/^\s*\(func \$cstr_len/)) cstrLenLine = i;
}

// Find the closing parens of these functions by tracking paren depth
if (initFuncLine >= 0) {
  let d = 0;
  for (let i = initFuncLine; i < lines.length; i++) {
    d += parenDepth(lines[i]);
    if (d <= 0) { initCloseLine = i; break; }
  }
}

console.log(`init: ${initFuncLine}-${initCloseLine}, parse: ${parseSourceLine}, is: ${isSourceLine}, cstr: ${cstrLenLine}`);

// Pass 1: identify lines to skip
for (let i = 0; i < lines.length; i++) {
  const line = lines[i];

  // Module wrapper
  if (line.trim() === '(module') { skip.add(i); continue; }
  if (line.trim() === ')' && i === moduleCloseIdx) { skip.add(i); continue; }

  // Memory import
  if (line.includes('(import "host" "memory"')) { skip.add(i); continue; }

  // Remove wasm_ptr/wasm_len globals
  if (line.match(/^\s*\(global\s+\$wasm_ptr\s+\(mut i32\)/)) { skip.add(i); continue; }
  if (line.match(/^\s*\(global\s+\$wasm_len\s+\(mut i32\)/)) { skip.add(i); continue; }

  // Remove parse_source_section function and body
  if (i === parseSourceLine) { i = skipFuncBody(i); continue; }

  // Remove is_source_section function and body
  if (i === isSourceLine) { i = skipFuncBody(i); continue; }

  // Remove cstr_len function and body
  if (i === cstrLenLine) { i = skipFuncBody(i); continue; }

  // Remove init function body (everything between the func def and its closing paren)
  if (i > initFuncLine && i < initCloseLine) { skip.add(i); continue; }
}

// Pass 2: build output
let out = '';
let insertedNewBody = false;

for (let i = 0; i < lines.length; i++) {
  if (skip.has(i)) continue;
  let line = lines[i];

  line = line.replace(/\$memcpy/g, '$er_memcpy');

  // Transform init → init_source with new params
  if (line.includes('(export "init")')) {
    line = line.replace(
      '(func (export "init")',
      '(func $init_source (export "init_source")'
    );
  }
  if (line.includes('(func $init)')) {
    line = line.replace('$init', '$init_source');
  }
  // Rename function params (simple variable name replacement)
  if (line.includes('$wp')) line = line.replace(/\$wp/g, '$data_ptr');
  if (line.includes('$wl')) line = line.replace(/\$wl/g, '$data_len');

  // Insert new init body right after the func declaration line
  if (line.includes('(export "init_source")') && !insertedNewBody) {
    insertedNewBody = true;
    out += line + '\n';
    out += '    (global.set $file_count (i32.const 0))\n';
    out += '    (global.set $arena_ptr (global.get $ARENA))\n';
    out += '    (call $leb_u32_read (local.get $data_ptr))\n';
    out += '    (local.set $data_ptr (i32.add (local.get $data_ptr) (global.get $R1)))\n';
    out += '    (call $parse_file_entries (local.get $data_ptr) (global.get $R0))\n';
    out += '    (global.get $file_count)\n';
    continue;
  }

  // Update stale comment about init
  line = line.replace('init(wasm_ptr, wasm_len)', 'init_source(data_ptr, data_len)');

  out += line + '\n';
}

// Verify paren balance
let depth = 0;
for (const ch of out) { if (ch === '(') depth++; if (ch === ')') depth--; }
console.log(`Paren balance: ${depth} (should be 0)`);

writeFileSync(resolve(ROOT, 'compiler/er-tools-fragment.wat'), out, 'utf-8');
const outLines = out.trimEnd().split('\n').length;
console.log(`✓ compiler/er-tools-fragment.wat (${outLines} lines)`);
