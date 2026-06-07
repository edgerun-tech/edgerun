#!/usr/bin/env bun
/**
 * EdgeRun Codebase Metadata Generator
 *
 * Scans all source .wat files in the build manifest and produces a WAT fragment
 * containing data tables + query functions for introspection.
 *
 * Usage: bun tools/generate_metadata.mjs [--out <path>]
 *   Default output: data/generated-metadata.wat
 */

import { readFileSync, writeFileSync, existsSync, statSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');

// ── WAT Fragment Scanner ──────────────────────────────────────────────

function scanFile(filePath) {
  const fullPath = resolve(ROOT, filePath);
  if (!existsSync(fullPath)) return null;
  const text = readFileSync(fullPath, 'utf-8');
  const lines = text.split('\n');
  const lineCount = lines.length;
  const byteCount = text.length;

  // Extract exports
  const exports = [];
  const funcEx = /\(func\s+(\$\w[\w-]*)\s*(?:\(export\s+"([^"]+)"\))?/g;
  let m;
  while ((m = funcEx.exec(text)) !== null) {
    const funcName = m[1];
    const exportName = m[2] || null;

    // Find params and results after this position
    const rest = text.slice(m.index + m[0].length);
    const params = [];
    const results = [];
    const paramRe = /\(param\s+(?:\$\w+\s+)?((?:i32|i64|f32|f64)(?:\s+i32|\s+i64|\s+f32|\s+f64)*)/g;
    const resultRe = /\(result\s+((?:i32|i64|f32|f64)(?:\s+i32|\s+i64|\s+f32|\s+f64)*)/g;
    let pm;
    let depth = 0;
    let pos = 0;
    // Scan rest for param/result until function body starts (open paren at depth 0)
    for (let i = 0; i < rest.length; i++) {
      const ch = rest[i];
      if (ch === '(') depth++;
      else if (ch === ')') { depth--; if (depth < 0) break; }

      if (depth === 1 && rest.startsWith('(param ', i)) {
        const end = rest.indexOf(')', i);
        if (end > 0) {
          const decl = rest.slice(i + 7, end).trim();
          const parts = decl.split(/\s+/);
          // parts could be: "$name i32" or just "i32" or "i32 i32 i32"
          const types = parts.filter(p => p === 'i32' || p === 'i64' || p === 'f32' || p === 'f64');
          params.push(...types);
          i = end;
        }
      }
      if (depth === 1 && rest.startsWith('(result ', i)) {
        const end = rest.indexOf(')', i);
        if (end > 0) {
          const decl = rest.slice(i + 8, end).trim();
          const parts = decl.split(/\s+/);
          const types = parts.filter(p => p === 'i32' || p === 'i64' || p === 'f32' || p === 'f64');
          results.push(...types);
          i = end;
        }
      }
    }

      // Compute source line number from match position
      const lineNum = text.slice(0, m.index).split('\n').length;

      exports.push({
        kind: 'function',
        name: funcName,
        exportName,
        params: params.join(' '),
        results: results.join(' '),
        sourceLine: lineNum,
      });
  }

  // Extract globals
  const globals = [];
  const globalRe = /\(global\s+(\$\w[\w-]*)\s*(?:\(export\s+"([^"]+)"\))?\s*(?:\(mut\s+)?(i32|i64|f32|f64)\s*(?:\(i32\.const\s+([-\d]+)\))?\s*(?:\(i64\.const\s+([-\d]+)\))?/g;
  while ((m = globalRe.exec(text)) !== null) {
    const gName = m[1];
    const exportName = m[2] || null;
    const type = m[3];
    let value = null;
    if (type === 'i32' && m[4] !== undefined) value = BigInt(m[4]);
    else if (type === 'i64' && m[5] !== undefined) value = BigInt(m[5]);
    globals.push({ name: gName, exportName, type, value });
  }

  return {
    filePath,
    lineCount,
    byteCount,
    exports,
    globals,
  };
}

// ── Manifest (mirrors build_wat.mjs) ──────────────────────────────────

const MANIFEST = [
  'runtime/module-header.wat',
  'runtime/memory.wat',
  'runtime/memory-map.wat',
  'runtime/edgerun-core.wat',
  'runtime/math-utils.wat',
  'pipeline/pipe-core.wat',
  'pipeline/pipeline-core.wat',
  'pipeline/frame-core.wat',
  'pipeline/frame-pacer.wat',
  'pipeline/mux-core.wat',
  'pipeline/queue-stage.wat',
  'pipeline/buffer-stage.wat',
  'pipeline/cdc-stage.wat',
  'compiler/interpreter-core.wat',
  'compiler/interpreter.wat',
  'lang/edgerun-ir-core.wat',
  'lang/edgerun-parse.wat',
  'lang/edgerun-resolve.wat',
  'lang/edgerun-lower.wat',
  'lang/edgerun-privacy.wat',
  'pipeline/edgerun-parse-stage.wat',
  'pipeline/edgerun-exec-stage.wat',
  'compiler/base-x86-64.wat',
  'compiler/emit-x86-64.wat',
  'compiler/dispatch.wat',
  'compiler/templates-x86-64.wat',
  'compiler/simd-x86-64.wat',
  'compiler/compiler-arm32.wat',
  'compiler/emit-arm32.wat',
  'compiler/compiler-aarch64.wat',
  'compiler/emit-aarch64.wat',
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
  'crypto/crypto-aes128-gcm.wat',
  'crypto/crypto-aes-ctr.wat',
  'crypto/crypto-hmac-sha256.wat',
  'crypto/crypto-x25519-scalar.wat',
  'pipeline/sha256-stage.wat',
  'pipeline/hmac-sha256-stage.wat',
  'pipeline/sha1-stage.wat',
  'pipeline/sha512-stage.wat',
  'pipeline/sha384-stage.wat',
  'pipeline/aes128-gcm-encrypt-stage.wat',
  'pipeline/aes128-gcm-decrypt-stage.wat',
  'pipeline/aes128-ctr-xor-stage.wat',
  'pipeline/djb2-hash-stage.wat',
  'pipeline/inet-checksum-stage.wat',
  'pipeline/crc32-bzip-stage.wat',
  'protocol/binary-core.wat',
  'protocol/cache.wat',
  'protocol/der.wat',
  'protocol/dhcp.wat',
  'protocol/dns.wat',
  'protocol/hpack.wat',
  'protocol/http.wat',
  'protocol/ipv4-net.wat',
  'protocol/packet-core.wat',
  'protocol/quic.wat',
  'protocol/sfx-tables.wat',
  'protocol/tls.wat',
  'protocol/ws.wat',
  'codec/checksum-core.wat',
  'codec/base64.wat',
  'codec/json.wat',
  'codec/compress.wat',
  'codec/text.wat',
  'codec/serialize.wat',
  'codec/binary.wat',
  'codec/string-url.wat',
  'codec/media.wat',
  'codec/model.wat',
  'pipeline/base64-encode-stage.wat',
  'pipeline/base64-decode-stage.wat',
  'ui/ui_framework.wat',
  'pipeline/ui-layout-stage.wat',
  'pipeline/ui-paint-stage.wat',
  'pipeline/ui-event-stage.wat',
  'app/oauth.wat',
  'app/oci.wat',
  'app/codex.wat',
  'app/wallet.wat',
  'app/security.wat',
  'app/identity.wat',
  'app/net.wat',
  'app/core.wat',
  'app/ui.wat',
  'app/wayland.wat',
  'data/hex-core.wat',
  'data/byte-search.wat',
  'data/cache-archive-split.wat',
  'data/color-util.wat',
  'data/glob-match.wat',
  'data/html-strip.wat',
  'data/mime-quoted-printable.wat',
  'data/secret-service-core.wat',
  'data/shell-word-scan.wat',
  'data/sse-event-stream.wat',
  'data/storage-core-index.wat',
  'data/string-distance.wat',
  'data/tag-list.wat',
  'data/terminal-control-scan.wat',
  'data/terminal-state-core.wat',
  'data/uuid-util.wat',
  'pipeline/stage-registry.wat',
  'runtime/module-footer.wat',
];

// ── WAT Encoder Helpers ──

function encodeLEB128(value) {
  const bytes = [];
  let v = BigInt(value);
  const negative = v < 0n;
  if (negative) v = -v - 1n;
  let more = true;
  while (more) {
    let byte = Number(v & 0x7fn);
    v >>= 7n;
    if (negative) byte = (~byte) & 0x7f;
    if ((v === 0n && !(byte & 0x40)) || (v === -1n && (byte & 0x40))) {
      more = false;
    } else {
      byte |= 0x80;
    }
    bytes.push(byte);
  }
  return bytes;
}

function encodeI32LE(value) {
  const v = Number(value) >>> 0;
  return [v & 0xff, (v >> 8) & 0xff, (v >> 16) & 0xff, (v >> 24) & 0xff];
}

function encodeI64LE(value) {
  const v = BigInt(value);
  const lo = Number(v & 0xffffffffn);
  const hi = Number((v >> 32n) & 0xffffffffn);
  return [
    lo & 0xff, (lo >> 8) & 0xff, (lo >> 16) & 0xff, (lo >> 24) & 0xff,
    hi & 0xff, (hi >> 8) & 0xff, (hi >> 16) & 0xff, (hi >> 24) & 0xff,
  ];
}

// ── Generator ──

function generate() {
  const METADATA_BASE = 0x400000;
  const MAX_MODULES = 256;
  const MAX_FUNCTIONS = 4096;
  const MAX_GLOBALS = 512;
  const MAX_STRINGS = 65536;

  let stringTable = [];
  let stringOffsets = new Map();

  function intern(str) {
    if (stringOffsets.has(str)) return stringOffsets.get(str);
    const offset = stringTable.length;
    // Compute UTF-8 byte length
    let utf8Len = 0;
    for (let i = 0; i < str.length; i++) {
      const cp = str.charCodeAt(i);
      if (cp < 0x80) utf8Len += 1;
      else if (cp < 0x800) utf8Len += 2;
      else utf8Len += 3;
    }
    // Write 2-byte LE length prefix
    stringTable.push(utf8Len & 0xff);
    stringTable.push((utf8Len >> 8) & 0xff);
    for (let i = 0; i < str.length; i++) {
      const cp = str.charCodeAt(i);
      if (cp < 0x80) {
        stringTable.push(cp);
      } else if (cp < 0x800) {
        stringTable.push(0xc0 | (cp >> 6));
        stringTable.push(0x80 | (cp & 0x3f));
      } else {
        stringTable.push(0xe0 | (cp >> 12));
        stringTable.push(0x80 | ((cp >> 6) & 0x3f));
        stringTable.push(0x80 | (cp & 0x3f));
      }
    }
    stringOffsets.set(str, offset);
    return offset;
  }

  // Scan all files
  const files = [];
  for (const fp of MANIFEST) {
    const result = scanFile(fp);
    if (result) files.push(result);
  }

  // Build module table: name, file path, line count, byte count, export count
  const moduleTable = []; // each entry: [nameOff, fileOff, lineCnt, byteCnt, exportCnt]
  const funcTable = [];   // each entry: [nameOff, moduleIdx, exportNameOff, sourceLine]
  const globalTable = []; // each entry: [nameOff, moduleIdx, typeCode, valueLo, valueHi]

  for (let mi = 0; mi < files.length; mi++) {
    const f = files[mi];
    const nameOff = intern(f.filePath);
    const fileOff = nameOff; // same as name for now
    const exportCount = f.exports.filter(e => e.exportName).length;
    moduleTable.push([nameOff, fileOff, f.lineCount, f.byteCount, exportCount]);

    // Process exports
    for (const ex of f.exports) {
      const nameOff2 = intern(ex.name.replace(/^\$/, ''));
      const exportNameOff = ex.exportName ? intern(ex.exportName) : 0;
      funcTable.push([nameOff2, mi, exportNameOff, ex.sourceLine]);
    }

    // Process globals
    for (const g of f.globals) {
      const nameOff2 = intern(g.name.replace(/^\$/, ''));
      let typeCode = 0;
      if (g.type === 'i32') typeCode = 1;
      else if (g.type === 'i64') typeCode = 2;
      else if (g.type === 'f32') typeCode = 3;
      else if (g.type === 'f64') typeCode = 4;
      const value = g.value !== null ? g.value : 0n;
      globalTable.push([nameOff2, mi, typeCode, Number(value & 0xffffffffn), Number((value >> 32n) & 0xffffffffn)]);
    }
  }

  // Pad string table to 4-byte alignment
  while (stringTable.length % 4 !== 0) stringTable.push(0);

  // Compute table offsets from METADATA_BASE
  const HEADER_SIZE = 32; // 8 × i32 fields
  let offset = METADATA_BASE;

  const STRING_TABLE_OFF = offset;
  const STRING_TABLE_LEN = stringTable.length;
  offset += STRING_TABLE_LEN;

  const MODULE_TABLE_OFF = offset;
  const MODULE_RECORD_SIZE = 20; // 5 × i32
  const MODULE_COUNT = moduleTable.length;
  const moduleTableBytes = [];
  for (const entry of moduleTable) {
    moduleTableBytes.push(...encodeI32LE(entry[0]));
    moduleTableBytes.push(...encodeI32LE(entry[1]));
    moduleTableBytes.push(...encodeI32LE(entry[2]));
    moduleTableBytes.push(...encodeI32LE(entry[3]));
    moduleTableBytes.push(...encodeI32LE(entry[4]));
  }
  offset += moduleTableBytes.length;

  const FUNC_TABLE_OFF = offset;
  const FUNC_RECORD_SIZE = 16; // 4 × i32 (nameOff, moduleIdx, exportNameOff, sourceLine)
  const FUNC_COUNT = funcTable.length;
  const funcTableBytes = [];
  for (const entry of funcTable) {
    funcTableBytes.push(...encodeI32LE(entry[0]));
    funcTableBytes.push(...encodeI32LE(entry[1]));
    funcTableBytes.push(...encodeI32LE(entry[2]));
    funcTableBytes.push(...encodeI32LE(entry[3]));
  }
  offset += funcTableBytes.length;

  const GLOBAL_TABLE_OFF = offset;
  const GLOBAL_RECORD_SIZE = 20; // 5 × i32 (nameOff, moduleIdx, typeCode, valueLo, valueHi)
  const GLOBAL_COUNT = globalTable.length;
  const globalTableBytes = [];
  for (const entry of globalTable) {
    globalTableBytes.push(...encodeI32LE(entry[0]));
    globalTableBytes.push(...encodeI32LE(entry[1]));
    globalTableBytes.push(...encodeI32LE(entry[2]));
    globalTableBytes.push(...encodeI32LE(entry[3]));
    globalTableBytes.push(...encodeI32LE(entry[4]));
  }
  offset += globalTableBytes.length;

  const METADATA_SIZE = offset - METADATA_BASE;

  // ── Generate WAT fragment ──

  let wat = `;; ── Auto-generated: codebase metadata tables ──
;; Generated by tools/generate_metadata.mjs
;; DO NOT EDIT — regenerate with: bun tools/generate_metadata.mjs

;; ── Metadata header constants ──
(global $META_BASE          i32 (i32.const ${METADATA_BASE}))
(global $META_MAGIC         i32 (i32.const 0x4D455441))  ;; "META"
(global $META_VERSION       i32 (i32.const 1))

;; String table
(global $META_STRINGS       i32 (i32.const ${STRING_TABLE_OFF}))
(global $META_STRINGS_LEN   i32 (i32.const ${STRING_TABLE_LEN}))

;; Module table
(global $META_MODULES       i32 (i32.const ${MODULE_TABLE_OFF}))
(global $META_MODULE_COUNT  i32 (i32.const ${MODULE_COUNT}))
(global $META_MODULE_RSIZE  i32 (i32.const ${MODULE_RECORD_SIZE}))

;; Function table
(global $META_FUNCTIONS     i32 (i32.const ${FUNC_TABLE_OFF}))
(global $META_FUNC_COUNT    i32 (i32.const ${FUNC_COUNT}))
(global $META_FUNC_RSIZE    i32 (i32.const ${FUNC_RECORD_SIZE}))

;; Global table
(global $META_GLOBALS       i32 (i32.const ${GLOBAL_TABLE_OFF}))
(global $META_GLOBAL_COUNT  i32 (i32.const ${GLOBAL_COUNT}))
(global $META_GLOBAL_RSIZE  i32 (i32.const ${GLOBAL_RECORD_SIZE}))

;; ── Data segments ──

;; String table (length-prefixed UTF-8)
(data (i32.const ${STRING_TABLE_OFF})
`;

  // Write string table as hex bytes, 16 per line
  for (let i = 0; i < stringTable.length; i += 16) {
    const chunk = stringTable.slice(i, i + 16);
    const hex = chunk.map(b => '\\' + b.toString(16).padStart(2, '0')).join('');
    wat += `  "${hex}"\n`;
  }

  wat += `)\n\n;; Module table (${MODULE_COUNT} entries × ${MODULE_RECORD_SIZE} bytes)\n(data (i32.const ${MODULE_TABLE_OFF})\n`;
  for (let i = 0; i < moduleTableBytes.length; i += 16) {
    const chunk = moduleTableBytes.slice(i, i + 16);
    const hex = chunk.map(b => '\\' + b.toString(16).padStart(2, '0')).join('');
    wat += `  "${hex}"\n`;
  }
  wat += `)\n\n;; Function table (${FUNC_COUNT} entries × ${FUNC_RECORD_SIZE} bytes)\n(data (i32.const ${FUNC_TABLE_OFF})\n`;
  for (let i = 0; i < funcTableBytes.length; i += 16) {
    const chunk = funcTableBytes.slice(i, i + 16);
    const hex = chunk.map(b => '\\' + b.toString(16).padStart(2, '0')).join('');
    wat += `  "${hex}"\n`;
  }
  wat += `)\n\n;; Global table (${GLOBAL_COUNT} entries × ${GLOBAL_RECORD_SIZE} bytes)\n(data (i32.const ${GLOBAL_TABLE_OFF})\n`;
  for (let i = 0; i < globalTableBytes.length; i += 16) {
    const chunk = globalTableBytes.slice(i, i + 16);
    const hex = chunk.map(b => '\\' + b.toString(16).padStart(2, '0')).join('');
    wat += `  "${hex}"\n`;
  }
  wat += `)\n\n;; ── Query Functions ──\n\n`;

  // ── Helper: load string from offset, returns ptr + len in scratch ──
  wat += `(func $meta_string_ptr (param $off i32) (result i32)
  (i32.add (i32.add (global.get $META_STRINGS) (local.get $off)) (i32.const 2))
)
`;

  wat += `(func $meta_string_len (param $off i32) (result i32)
  (i32.or
    (i32.load8_u (i32.add (global.get $META_STRINGS) (local.get $off)))
    (i32.shl (i32.load8_u (i32.add (i32.add (global.get $META_STRINGS) (local.get $off)) (i32.const 1))) (i32.const 8))
  )
)
`;

  wat += `(func $memmem (param $haystack i32) (param $hlen i32) (param $needle i32) (param $nlen i32) (result i32)
  (local $i i32) (local $j i32)
  (if (i32.eqz (local.get $nlen)) (then (return (i32.const 1))))
  (if (i32.gt_u (local.get $nlen) (local.get $hlen)) (then (return (i32.const 0))))
  (block $done
    (loop $outer
      (if (i32.gt_u (local.get $i) (i32.sub (local.get $hlen) (local.get $nlen)))
        (then (br $done))
      )
      (local.set $j (i32.const 0))
      (block $nomatch
        (loop $inner
          (if (i32.ge_u (local.get $j) (local.get $nlen))
            (then (return (i32.const 1)))
          )
          (if (i32.ne (i32.load8_u (i32.add (local.get $haystack) (i32.add (local.get $i) (local.get $j)))) (i32.load8_u (i32.add (local.get $needle) (local.get $j))))
            (then (br $nomatch))
          )
          (local.set $j (i32.add (local.get $j) (i32.const 1)))
          (br $inner)
        )
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $outer)
    )
  )
  (i32.const 0)
)
`;

  wat += `
;; ── Metadata count functions ──

(func $metadata_module_count (export "metadata_module_count") (result i32)
  (global.get $META_MODULE_COUNT)
)

(func $metadata_function_count (export "metadata_function_count") (result i32)
  (global.get $META_FUNC_COUNT)
)

(func $metadata_global_count (export "metadata_global_count") (result i32)
  (global.get $META_GLOBAL_COUNT)
)

(func $metadata_string_len (export "metadata_string_len") (param $off i32) (result i32)
  (call $meta_string_len (local.get $off))
)

;; ── Module query functions ──

(func $metadata_module_file (export "metadata_module_file") (param $idx i32) (result i32)
  (i32.load (i32.add (global.get $META_MODULES) (i32.mul (local.get $idx) (global.get $META_MODULE_RSIZE))))
)

(func $metadata_module_line_count (export "metadata_module_line_count") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_MODULES) (i32.mul (local.get $idx) (global.get $META_MODULE_RSIZE))) (i32.const 8)))
)

(func $metadata_module_byte_count (export "metadata_module_byte_count") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_MODULES) (i32.mul (local.get $idx) (global.get $META_MODULE_RSIZE))) (i32.const 12)))
)

(func $metadata_module_export_count (export "metadata_module_export_count") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_MODULES) (i32.mul (local.get $idx) (global.get $META_MODULE_RSIZE))) (i32.const 16)))
)

;; ── Function query functions ──

(func $metadata_function_name (export "metadata_function_name") (param $idx i32) (result i32)
  (i32.load (i32.add (global.get $META_FUNCTIONS) (i32.mul (local.get $idx) (global.get $META_FUNC_RSIZE))))
)

(func $metadata_function_module (export "metadata_function_module") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_FUNCTIONS) (i32.mul (local.get $idx) (global.get $META_FUNC_RSIZE))) (i32.const 4)))
)

(func $metadata_function_params (export "metadata_function_params") (param $idx i32) (result i32)
  (i32.const 0)
)

(func $metadata_function_results (export "metadata_function_results") (param $idx i32) (result i32)
  (i32.const 0)
)

(func $metadata_function_export_name (export "metadata_function_export_name") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_FUNCTIONS) (i32.mul (local.get $idx) (global.get $META_FUNC_RSIZE))) (i32.const 8)))
)

(func $metadata_function_source_line (export "metadata_function_source_line") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_FUNCTIONS) (i32.mul (local.get $idx) (global.get $META_FUNC_RSIZE))) (i32.const 12)))
)

;; ── Find function by exact name match ──

(func $metadata_find_function (export "metadata_find_function") (param $name_ptr i32) (param $name_len i32) (result i32)
  (local $i i32) (local $fname_off i32) (local $fname_len i32)
  (local.set $i (i32.const 0))
  (block $not_found
    (loop $search
      (if (i32.ge_u (local.get $i) (global.get $META_FUNC_COUNT))
        (then (br $not_found))
      )
      (local.set $fname_off (call $metadata_function_name (local.get $i)))
      (local.set $fname_len (call $meta_string_len (local.get $fname_off)))
      (if (call $string_eq (local.get $name_ptr) (local.get $name_len) (call $meta_string_ptr (local.get $fname_off)) (local.get $fname_len))
        (then (return (local.get $i)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $search)
    )
  )
  (i32.const -1)
)

;; ── Global query functions ──

(func $metadata_global_name (export "metadata_global_name") (param $idx i32) (result i32)
  (i32.load (i32.add (global.get $META_GLOBALS) (i32.mul (local.get $idx) (global.get $META_GLOBAL_RSIZE))))
)

(func $metadata_global_module (export "metadata_global_module") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_GLOBALS) (i32.mul (local.get $idx) (global.get $META_GLOBAL_RSIZE))) (i32.const 4)))
)

(func $metadata_global_type (export "metadata_global_type") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_GLOBALS) (i32.mul (local.get $idx) (global.get $META_GLOBAL_RSIZE))) (i32.const 8)))
)

(func $metadata_global_value_lo (export "metadata_global_value_lo") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_GLOBALS) (i32.mul (local.get $idx) (global.get $META_GLOBAL_RSIZE))) (i32.const 12)))
)

(func $metadata_global_value_hi (export "metadata_global_value_hi") (param $idx i32) (result i32)
  (i32.load (i32.add (i32.add (global.get $META_GLOBALS) (i32.mul (local.get $idx) (global.get $META_GLOBAL_RSIZE))) (i32.const 16)))
)

;; ── Find global by exact name match ──

(func $metadata_find_global (export "metadata_find_global") (param $name_ptr i32) (param $name_len i32) (result i32)
  (local $i i32) (local $gname_off i32) (local $gname_len i32)
  (local.set $i (i32.const 0))
  (block $not_found
    (loop $search
      (if (i32.ge_u (local.get $i) (global.get $META_GLOBAL_COUNT))
        (then (br $not_found))
      )
      (local.set $gname_off (call $metadata_global_name (local.get $i)))
      (local.set $gname_len (call $meta_string_len (local.get $gname_off)))
      (if (call $string_eq (local.get $name_ptr) (local.get $name_len) (call $meta_string_ptr (local.get $gname_off)) (local.get $gname_len))
        (then (return (local.get $i)))
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $search)
    )
  )
  (i32.const -1)
)

;; Find exported function by export name (returns i64 pack(1, idx) or pack(0, -1))
(func $metadata_find_export (export "metadata_find_export") (param $name_ptr i32) (param $name_len i32) (result i64)
  (local $i i32) (local $export_off i32) (local $export_len i32)
  (local.set $i (i32.const 0))
  (block $not_found
    (loop $search
      (if (i32.ge_u (local.get $i) (global.get $META_FUNC_COUNT))
        (then (br $not_found))
      )
      (local.set $export_off (call $metadata_function_export_name (local.get $i)))
      (if (local.get $export_off)
        (then
          (local.set $export_len (call $meta_string_len (local.get $export_off)))
          (if (call $string_eq (local.get $name_ptr) (local.get $name_len) (call $meta_string_ptr (local.get $export_off)) (local.get $export_len))
            (then (return (call $pack (i32.const 1) (local.get $i))))
          )
        )
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $search)
    )
  )
  (call $pack (i32.const 0) (i32.const -1))
)

;; List function indices belonging to a module (writes i32 LE indices to buffer, returns count)
(func $metadata_module_functions (export "metadata_module_functions") (param $module_idx i32) (param $out i32) (param $cap i32) (result i32)
  (local $i i32) (local $written i32)
  (local.set $i (i32.const 0))
  (loop $scan
    (if (i32.ge_u (local.get $i) (global.get $META_FUNC_COUNT))
      (then (return (local.get $written)))
    )
    (if (i32.eq (call $metadata_function_module (local.get $i)) (local.get $module_idx))
      (then
        (if (i32.lt_u (local.get $written) (local.get $cap))
          (then
            (i32.store (i32.add (local.get $out) (i32.shl (local.get $written) (i32.const 2))) (local.get $i))
            (local.set $written (i32.add (local.get $written) (i32.const 1)))
          )
          (else (return (local.get $written)))
        )
      )
    )
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $scan)
  )
  (local.get $written)
)

;; Search functions by substring match (writes matching indices as i32 LE to buffer, returns count)
(func $metadata_search_functions (export "metadata_search_functions") (param $pattern_ptr i32) (param $pattern_len i32) (param $out i32) (param $cap i32) (result i32)
  (local $i i32) (local $written i32) (local $name_off i32) (local $name_len i32)
  (local.set $i (i32.const 0))
  (block $done
    (loop $scan
      (if (i32.ge_u (local.get $i) (global.get $META_FUNC_COUNT))
        (then (br $done))
      )
      (local.set $name_off (call $metadata_function_name (local.get $i)))
      (local.set $name_len (call $meta_string_len (local.get $name_off)))
      (if (call $memmem (call $meta_string_ptr (local.get $name_off)) (local.get $name_len) (local.get $pattern_ptr) (local.get $pattern_len))
        (then
          (if (i32.lt_u (local.get $written) (local.get $cap))
            (then
              (i32.store (i32.add (local.get $out) (i32.shl (local.get $written) (i32.const 2))) (local.get $i))
              (local.set $written (i32.add (local.get $written) (i32.const 1)))
            )
            (else (br $done))
          )
        )
      )
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br $scan)
    )
  )
  (local.get $written)
)

;; List all function names (writes to buffer, returns count)
(func $metadata_list_functions (export "metadata_list_functions") (param $out i32) (param $cap i32) (result i32)
  (local $i i32) (local $written i32) (local $name_off i32) (local $name_len i32)
  (local $remaining i32)
  (local.set $i (i32.const 0))
  (loop $list
    (if (i32.ge_u (local.get $i) (global.get $META_FUNC_COUNT))
      (then (return (local.get $written)))
    )
    (local.set $name_off (call $metadata_function_name (local.get $i)))
    (local.set $name_len (call $meta_string_len (local.get $name_off)))
    (local.set $remaining (i32.sub (local.get $cap) (local.get $written)))
    (if (i32.ge_u (local.get $name_len) (local.get $remaining))
      (then (return (local.get $written)))
    )
    (call $memcpy (i32.add (local.get $out) (local.get $written)) (call $meta_string_ptr (local.get $name_off)) (local.get $name_len))
    (local.set $written (i32.add (local.get $written) (local.get $name_len)))
    (i32.store8 (i32.add (local.get $out) (local.get $written)) (i32.const 10))  ;; newline
    (local.set $written (i32.add (local.get $written) (i32.const 1)))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (br $list)
  )
  (local.get $written)
)
`;

  return wat;
}

// ── Main ──

function main() {
  const outPath = resolve(ROOT, process.argv.find(a => a.startsWith('--out='))?.slice(6) || 'data/generated-metadata.wat');
  const wat = generate();
  writeFileSync(outPath, wat, 'utf-8');
  console.log(`✓ metadata → ${outPath} (${wat.length} bytes)`);
}

main();
