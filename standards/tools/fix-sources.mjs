/**
 * ONE-TIME migration: convert all source files from standalone modules to clean fragments.
 * After this, build.mjs is just concat + compile — no stripping, no dedup, no rename.
 *
 * This script:
 * 1. Removes (module ...) wrappers from all source files
 * 2. Removes (memory ...) declarations (only runtime/memory.wat is canonical)
 * 3. Removes internal (import ...) (keeps host/wasi_snapshot_preview1 only)
 * 4. Applies rename_map entries to resolve cross-file name conflicts
 * 5. Handles strip_funcs entries (removes function defs that are defined canonically)
 */

import { readFileSync, writeFileSync, existsSync, readdirSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const PASS = '\x1b[32m', FAIL = '\x1b[31m', RST = '\x1b[0m';
let ok = 0, err = 0;

// Files that should be skipped entirely
const SKIP_FILES = new Set([
  // Canonical runtime — keep as-is
  'runtime/memory.wat',
  'runtime/memory-map.wat',
  'runtime/edgerun-core.wat',
  'runtime/math-utils.wat',
  // Auto-generated, keep as-is
  'ui/ui_framework.wat',
  'ui/00_prelude.wat',
  // Standalone examples/tests — keep their module wrappers
]);

// ── Rename map: local name → canonical name (from build_wat.mjs) ──
// When a file defines or imports a function with a local alias that differs
// from the canonical name in the merged module, rename all references.
const RENAME_MAP = {
  // crypto rename_map entries from build_wat.mjs
  'crypto/crypto-aes128-gcm.wat': { '$m54memcpy': '$memcpy' },
  'crypto/crypto-aes-ctr.wat': {
    '$m54memcpy': '$memcpy',
    '$sub_word': '$ctr_sub_word',
    '$rot_word': '$ctr_rot_word',
    '$aes128_encrypt_block': '$ctr_aes128_encrypt_block',
    '$store_be32': '$gcm_store_be32',
    '$load_be32': '$gcm_load_be32',
    '$store_be64': '$gcm_store_be64',
  },
  'crypto/crypto-hmac-sha256.wat': { '$m59memcpy': '$memcpy' },
  'crypto/crypto-x25519-scalar.wat': { '$m64memcpy': '$memcpy' },
  // protocol rename_map entries
  'protocol/packet-core.wat': {
    '$memcpy': '$pc_memcpy',
    '$memset': '$pc_memset',
    '$store_be32': '$pc_store_be32',
    '$load_be32': '$pc_load_be32',
    '$mix': '$pc_mix',
    '$read_u8': '$pc_read_u8',
  },
  'protocol/cache.wat': { '$read_u8': '$cache_read_u8' },
  // codec rename_map entries
  'codec/model.wat': { '$read_u8': '$model_read_u8' },
  // Other cross-file import aliases (from "edgerun" imports)
  'protocol/dns.wat': { '$m77lower_ascii': '$to_lower' },
  'protocol/dns-compressed-name.wat': { '$m77lower_ascii': '$to_lower' },
  'protocol/dns-name.wat': { '$m80lower_ascii': '$to_lower' },
  'protocol/dns-core-records.wat': { '$m78lower_ascii': '$to_lower' },
  'protocol/dns-rdata-core.wat': { '$m81lower_ascii': '$to_lower' },
  'protocol/tls-core-state.wat': { '$m180ascii_lower': '$to_lower' },
  'protocol/tls-name.wat': { '$m183lower_ascii': '$to_lower' },
  'protocol/http1-body.wat': { '$m111lower': '$to_lower' },
  'protocol/http1-header-block.wat': { '$m113lower': '$to_lower' },
  'protocol/http1-scan.wat': { '$m115lower': '$to_lower' },
  'protocol/http-node-state.wat': { '$m109lower': '$to_lower' },
};

// ── Strip funcs: function names to remove from specific files ──
// These are functions that exist in the file but are also defined
// canonically in runtime/ modules. Removing them avoids duplication.
const STRIP_FUNCS = {
  // Functions to strip from specific files (regex patterns, no leading $)
  'protocol/dns.wat': ['is_label_byte'],
  'protocol/dns-compressed-name.wat': ['is_label_byte'],
  'protocol/dns-core-records.wat': ['is_label_byte'],
  'protocol/dns-name.wat': ['is_label_byte'],
  'protocol/dns-rdata-core.wat': ['is_label_byte'],
  // codec base64: adler32 and crc32 helpers are in checksum-core.wat
  'codec/base64.wat': ['adler32_update', 'adler32_update_byte', 'adler32_update_vec', 'crc32_update_byte', 'bounds_check'],
  'codec/compress.wat': ['adler32_update', 'adler32_update_byte', 'adler32_update_vec', 'crc32_update_byte', 'bounds_check'],
  'codec/media.wat': ['reverse_bits'],
  // pipeline-core strips some UI dups
  'ui/ui_framework.wat': ['min_f32', 'max_f32'],
};

// ── Source files from build MANIFEST ──
function getAllWatFiles() {
  const dirs = ['app', 'codec', 'compiler', 'crypto', 'data', 'device',
                'lang', 'net', 'pipeline', 'protocol', 'runtime', 'system', 'tools', 'ui'];
  const files = [];
  for (const dir of dirs) {
    const dirPath = resolve(ROOT, dir);
    if (!existsSync(dirPath)) continue;
    for (const entry of readdirSync(dirPath)) {
      if (entry.endsWith('.wat')) {
        const path = dir + '/' + entry;
        if (SKIP_FILES.has(path)) continue;
        files.push(path);
      }
    }
  }
  return files.sort();
}

function stripModuleWrapper(content) {
  const m = content.match(/^\s*\(module\b/m);
  if (!m) return content;

  let depth = 1, inStr = false, inBlock = false;
  let i = m.index + m[0].length;
  while (i < content.length && depth > 0) {
    const ch = content[i];
    if (inBlock) {
      if (ch === ';' && i > 0 && content[i - 1] === ')') inBlock = false;
    } else if (inStr) {
      if (ch === '\\' && i + 1 < content.length) i++;
      else if (ch === '"') inStr = false;
    } else {
      if (ch === ';' && i + 1 < content.length && content[i + 1] === ';')
        { const nl = content.indexOf('\n', i); if (nl === -1) break; i = nl; continue; }
      if (ch === '(' && i + 1 < content.length && content[i + 1] === ';')
        { inBlock = true; i++; continue; }
      if (ch === '"') inStr = true;
      else if (ch === '(') depth++;
      else if (ch === ')') { depth--; if (depth === 0) break; }
    }
    i++;
  }
  if (depth !== 0) return content;
  return content.slice(m.index + m[0].length, i);
}

function stripMemoryExport(content) {
  return content.replace(/^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)\s*\n?/m, '');
}

function stripImport(content) {
  let result = '';
  let i = 0;
  while (i < content.length) {
    if (content[i] === ';' && i + 1 < content.length && content[i + 1] === ';') {
      const nl = content.indexOf('\n', i);
      result += content.slice(i, nl !== -1 ? nl + 1 : content.length);
      i = nl !== -1 ? nl + 1 : content.length;
      continue;
    }
    if (content[i] === '"') {
      let e = i + 1;
      while (e < content.length && content[e] !== '"') { if (content[e] === '\\') e++; e++; }
      result += content.slice(i, e + 1);
      i = e + 1;
      continue;
    }
    const idx = content.indexOf('(import "', i);
    if (idx === -1) { result += content.slice(i); break; }
    result += content.slice(i, idx);
    let depth = 1, j = idx + 1;
    while (j < content.length && depth > 0) {
      if (content[j] === '(') depth++;
      if (content[j] === ')') depth--;
      j++;
    }
    const imp = content.slice(idx, j);
    const modMatch = imp.match(/\(import\s+"([^"]+)"/);
    const moduleName = modMatch ? modMatch[1] : '';
    // ALL imports are stripped from source files.
    // Host imports live in runtime/module-header.wat.
    i = j;
  }
  return result;
}

const CANONICAL_GLOBAL_PATTERNS = [
  /^\s*\(global\s+\$OK\b.*\n?/gm,
  /^\s*\(global\s+\$LINUX_SYS_X64_\w+\s.*\n?/gm,
  /^\s*\(global\s+\$LINUX_SYS_AARCH64_\w+\s.*\n?/gm,
  /^\s*\(global\s+\$JIT_CACHE\b.*\n?/gm,
  /^\s*\(global\s+\$JIT_CACHE_SIZE\b.*\n?/gm,
  /^\s*\(global\s+\$ERR_UNSUP\b.*\n?/gm,
];

function stripCanonicalGlobals(content) {
  let result = content;
  for (const pattern of CANONICAL_GLOBAL_PATTERNS) {
    result = result.replace(pattern, '');
  }
  return result;
}

function stripSpecificFuncs(content, names) {
  // Strip function definitions by name (regex patterns without leading $)
  for (const name of names) {
    const pattern = `\\(func\\s+\\$${name}\\b`;
    const re = new RegExp(pattern, 'g');
    let match;
    while ((match = re.exec(content)) !== null) {
      const start = match.index;
      let depth = 0, i = start;
      while (i < content.length) {
        if (content[i] === ';' && i + 1 < content.length && content[i + 1] === ';') {
          const nl = content.indexOf('\n', i);
          i = nl !== -1 ? nl + 1 : content.length;
          continue;
        }
        if (content[i] === '(') depth++;
        if (content[i] === ')') {
          depth--;
          if (depth === 0) break;
        }
        i++;
      }
      if (depth === 0) {
        content = content.slice(0, start) + content.slice(i + 1);
        re.lastIndex = start;
      }
    }
  }
  return content;
}

function applyRenames(content, renameMap) {
  for (const [from, to] of Object.entries(renameMap)) {
    content = content.split(from).join(to);
  }
  return content;
}

// ── Process each file ──
const files = getAllWatFiles();
console.log(`Processing ${files.length} files...\n`);

for (const filePath of files) {
  const fullPath = resolve(ROOT, filePath);
  if (!existsSync(fullPath)) { console.warn(`  ! ${filePath} not found`); err++; continue; }

  let content = readFileSync(fullPath, 'utf-8');
  const origLen = content.length;
  if (origLen === 0) { console.warn(`  ! ${filePath} is empty`); err++; continue; }

  // Skip runtime/ files — they're the canonical source
  if (filePath.startsWith('runtime/')) {
    console.log(`  ${PASS}SKIP${RST} ${filePath} (canonical)`);
    ok++;
    continue;
  }

  content = stripModuleWrapper(content);
  content = stripMemoryExport(content);
  content = stripImport(content);
  content = stripCanonicalGlobals(content);

  // Apply per-file rename maps
  if (RENAME_MAP[filePath]) {
    content = applyRenames(content, RENAME_MAP[filePath]);
  }

  // Strip specific function definitions
  if (STRIP_FUNCS[filePath]) {
    content = stripSpecificFuncs(content, STRIP_FUNCS[filePath]);
  }

  content = content.trim() + '\n';

  writeFileSync(fullPath, content, 'utf-8');
  const saved = origLen - content.length;
  const label = saved > 0 ? `${saved} bytes stripped` : 'unchanged';
  console.log(`  ${PASS}OK${RST}   ${filePath} (${label})`);
  ok++;
}

console.log(`\n  ${err > 0 ? FAIL : PASS}${ok}/${ok + err} files processed, ${err} errors${RST}\n`);
if (err > 0) process.exit(1);
