#!/usr/bin/env node
/**
 * EdgeRun Build System
 *
 * Concatenates source fragments into a single `edgerun.wasm`.
 *
 * Usage:
 *   node tools/build_wat.mjs [--out edgerun.wat]
 *
 * Source order is defined by the MANIFEST array below.
 * Each entry can be:
 *   - A filename string (fragment, no (module) wrapper expected)
 *   - An object {file, strip_module, strip_memory, strip_imports}
 *
 * Files that end with "-stage.wat" are automatically treated as stage wrappers
 * and get their (import)s converted to local references when the target is in-module.
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { resolve, dirname, basename } from 'path';

const ROOT = resolve(import.meta.dirname, '..');

// ── Manifest: dependency-order fragment list ──────────────────────
// Order matters: globals before functions, definitions before references.
const MANIFEST = [
  // ── Layer 0: Runtime foundation ──
  'runtime/memory.wat',           // canonical (memory) + LUT data
  'runtime/memory-map.wat',       // address space constants
  'runtime/edgerun-core.wat',     // char helpers, pack, memcpy, status, syscalls
  'runtime/math-utils.wat',       // min, max, clamp, etc.

  // ── Layer 1: Pipeline transport ──
  'pipeline/pipe-core.wat',       // pipes, bump allocator

  // ── Layer 2: Pipeline dispatch ──
  'pipeline/pipeline-core.wat',   // stage_table, pipeline_run, descriptors

  // ── Layer 3: Pipeline stages (alphabetical) ──
  'pipeline/frame-core.wat',
  'pipeline/frame-pacer.wat',
  'pipeline/mux-core.wat',
  // wasm-exec-stage.wat — needs $load/$load_wat/$call stubs (deleted from interpreter split)
  // 'pipeline/wasm-exec-stage.wat',

  // ── Layer 4: Interpreter / Compiler ──
  'compiler/interpreter-core.wat', // interpreter engine
  'compiler/interpreter.wat',      // WAT parser + standalone exports
  // 'pipeline/wasm-interpreter.wat', // pipeline wrapper (thin) — just comments, no functions

  // ── Layer 5: Interpreter fragments ──
  // interpreter-wat.wat content is already in interpreter.wat (standalone version)

  // ── Layer 6: JIT Compiler (x86-64 backend) ──
  // templates-x86-64.wat, simd-x86-64.wat, dispatch.wat excluded —
  // they reference ~100+ $emit_* functions that don't exist yet (JIT is incomplete)
  'compiler/base-x86-64.wat',
  'compiler/emit-x86-64.wat',

  // ── Layer 7: Crypto ──
  'crypto/hash-djb2.wat',
  'crypto/checksum-crc32-bzip.wat',
  'crypto/checksum-inet.wat',
  'crypto/convex-hull.wat',
  'crypto/crypto-isaac.wat',
  'crypto/crypto-xtea.wat',
  'crypto/crypto-sha1.wat',
  'crypto/crypto-sha256.wat',
  'crypto/crypto-sha512.wat',
  'crypto/crypto-bigint-limb.wat',
  'crypto/crypto-ed25519-shape.wat',
  'crypto/crypto-ecdsa-der.wat',
  'crypto/crypto-rsa-pkcs1.wat',
  'crypto/crypto-hmac-hkdf.wat',
  // AES files: data at offset 0 (S-box) — no LUT conflict, zero-page tolerated
  {file: 'crypto/crypto-aes128-gcm.wat', rename_map: {'$m54memcpy': '$memcpy'}},
  {file: 'crypto/crypto-aes-ctr.wat', rename_map: {
    '$m54memcpy': '$memcpy',
    '$sub_word': '$ctr_sub_word',
    '$rot_word': '$ctr_rot_word',
    '$aes128_encrypt_block': '$ctr_aes128_encrypt_block'
  }},
  {file: 'crypto/crypto-hmac-sha256.wat', rename_map: {'$m59memcpy': '$memcpy'}},
  {file: 'crypto/crypto-x25519-scalar.wat', rename_map: {'$m64memcpy': '$memcpy'}},
  // crypto-aes-block.wat deferred (data at 0x2000 conflicts with lower_case LUT)

  // ── Layer 7a: Pipeline crypto stages ──
  'pipeline/sha256-stage.wat',
  'pipeline/hmac-sha256-stage.wat',

  // ── Layer 8: Protocol parsers ──
  // (TODO: convert protocol/*.wat to fragments)

  // ── Layer 9: Codec ──
  // (TODO: convert codec/*.wat to fragments)

  // ── Layer 10: UI Framework ──
  {file: 'ui/ui_framework.wat', strip_funcs: ['\\$min_f32', '\\$max_f32']},  // auto-generated combined fragment (28K lines)

  // ── Layer 11: System ──
  // (TODO: convert system/*.wat to fragments)

  // ── Layer 12: App ──
  // (TODO: convert app/*.wat to fragments)

  // ── Layer 13: Device ──
  // (TODO: convert device/*.wat to fragments)

  // ── Layer 14: Data ──
  // (TODO: convert data/*.wat to fragments)

  // ── Layer 15: Net ──
  // (TODO: convert net/*.wat to fragments)

  // ── Layer 16: Tools ──
  // (TODO: convert tools/*.wat to fragments)
];

// ── Fragments that import from edgerun-core (will be auto-resolved) ──
// Map: fragment file → list of imported names that should become local refs
const IMPORT_MAP = {
  'pipeline/pipe-core.wat':              ['min_u'],
  'pipeline/wasm-interpreter.wat':       ['STATUS_OK'],
  'compiler/interpreter-core.wat':       [],
  // Crypto files importing from "edgerun"
  'crypto/crypto-aes128-gcm.wat':        ['memcpy', 'memset'],
  'crypto/crypto-aes-ctr.wat':           ['memcpy'],
  'crypto/crypto-hmac-sha256.wat':       ['memcpy'],
  'crypto/crypto-x25519-scalar.wat':     ['memcpy'],
};

// ── Globals defined canonically in runtime/ — strip from all other files ──
// These regex patterns match global definitions that should only appear once.
const CANONICAL_GLOBALS = [
  /^\s*\(global\s+\$OK\b.*\n?/gm,
  /^\s*\(global\s+\$LINUX_SYS_X64_\w+\s.*\n?/gm,
  /^\s*\(global\s+\$LINUX_SYS_AARCH64_\w+\s.*\n?/gm,
  /^\s*\(global\s+\$JIT_CACHE\b.*\n?/gm,
  /^\s*\(global\s+\$JIT_CACHE_SIZE\b.*\n?/gm,
  /^\s*\(global\s+\$ERR_UNSUP\b.*\n?/gm,
];

// ── Build ────────────────────────────────────────────────────────────

function stripModuleHeader(content) {
  return content.replace(/^\s*\(module\b[^)]*\)?\s*/m, '');
}

function stripMemory(content) {
  return content.replace(/^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)\s*/m, '');
}

function stripImport(content, file) {
  const imports = IMPORT_MAP[file];
  if (!imports) return content;
  for (const name of imports) {
    // Remove import lines matching this import name
    const regex = new RegExp(
      `\\s*\\(import\\s+"[^"]*"\\s+"${name}"\\s+\\(func\\s+\\$[^)]+\\)\\)\\s*`,
      'g'
    );
    content = content.replace(regex, '');
  }
  return content;
}

function stripFuncs(content, funcNames) {
  for (const name of funcNames) {
    // Strip from (func $name... to matching closing paren
    const startRegex = new RegExp(
      `\\(func\\s+${name}(?:\\s+\\(export\\s+"[^"]*"\\))?`,
      'g'
    );
    let match;
    while ((match = startRegex.exec(content)) !== null) {
      const start = match.index;
      let depth = 0;
      let i = start;
      while (i < content.length) {
        if (content[i] === '(') depth++;
        if (content[i] === ')') {
          depth--;
          if (depth === 0) break;
        }
        i++;
      }
      const end = i + 1;
      content = content.slice(0, start) + content.slice(end);
      startRegex.lastIndex = start; // re-scan from replacement point
    }
  }
  return content;
}

function stripCanonicalGlobals(content, filePath) {
  // Only strip from non-runtime files
  if (filePath.startsWith('runtime/')) return content;
  let stripped = 0;
  for (const pattern of CANONICAL_GLOBALS) {
    const before = content.length;
    content = content.replace(pattern, '');
    stripped += before - content.length;
  }
  if (stripped > 0) {
    console.log(`  ${filePath}: stripped ${stripped} bytes (canonical globals)`);
  }
  return content;
}

function processFile(filePath, opts = {}) {
  const fullPath = resolve(ROOT, filePath);
  if (!existsSync(fullPath)) {
    console.warn(`⚠  WARNING: ${filePath} not found — skipping`);
    return '';
  }

  let content = readFileSync(fullPath, 'utf-8');
  const originalLength = content.length;

  // Strip (module ...) header if this is a standalone module being converted
  if (opts.strip_module !== false && basename(filePath) !== 'edgerun.wat') {
    content = stripModuleHeader(content);
  }

  // Strip (memory ...) if not the canonical source
  if (opts.strip_memory !== false && filePath !== 'runtime/memory.wat') {
    content = stripMemory(content);
  }

  // Strip imports that resolve to local functions
  content = stripImport(content, filePath);

  // Strip canonical globals (defined in runtime/) from non-runtime files
  content = stripCanonicalGlobals(content, filePath);

  // Strip ALL global definitions from this file (used when file is redundant with another)
  if (opts.strip_all_globals) {
    content = content.replace(/^\s*\(global\s+\$\w+(?:\s+\(export\s+"[^"]*"\))?\s+(?:i32|\(mut\s+i32\))\s+\([^)]*\)\s*\).*$/gm, '');
  }

  // Strip specific function definitions by name (to resolve conflicts)
  if (opts.strip_funcs && opts.strip_funcs.length > 0) {
    content = stripFuncs(content, opts.strip_funcs);
  }

  // Rename local identifiers (used when imports use different local names)
  if (opts.rename_map) {
    for (const [from, to] of Object.entries(opts.rename_map)) {
      content = content.split(from).join(to);
    }
  }

  const stripped = originalLength - content.length;
  if (stripped > 0) {
    console.log(`  ${filePath}: stripped ${stripped} bytes (module/memory/imports)`);
  }

  // Add file marker comment
  return `;; ── ${filePath} ──\n${content.trim()}\n\n`;
}

function build() {
  const outPath = resolve(ROOT, process.argv.find(a => a.startsWith('--out='))?.slice(6) || 'edgerun.wat');

  console.log(`\nEdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log('Processing fragments:');

  let body = '';
  let count = 0;

  for (const entry of MANIFEST) {
    if (typeof entry === 'string') {
      body += processFile(entry, {});
      count++;
    } else if (entry.file) {
      body += processFile(entry.file, entry);
      count++;
    }
  }

  // Wrap in module if not already
  const moduleDecl = '(module\n';
  const moduleClose = '\n)\n';

  const final = moduleDecl + body + moduleClose;

  writeFileSync(outPath, final, 'utf-8');
  console.log(`\n✓ Written ${count} fragments → ${outPath} (${final.length} bytes)`);
}

build();
