#!/usr/bin/env bun
/**
 * EdgeRun Build System — ALL 3 JIT backends at runtime.
 *
 * Concatenates source fragments in order, wraps in (module), compiles to WASM.
 * All 3 JIT backends (x86-64, ARM32, AArch64) are included simultaneously,
 * with renamed exports and a runtime dispatch wrapper.
 */

import { readFileSync, writeFileSync, existsSync, statSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

const ROOT = resolve(import.meta.dirname, '..');
const argv = process.argv.slice(2);

function showUsage() {
  console.log('Usage: bun tools/build_wat.mjs [--out=<path>] [--no-wasm] [--watch]');
  console.log('Options:');
  console.log('  --out          Output WAT path (default: edgerun.wat)');
  console.log('  --no-wasm      Skip WASM compile step');
  console.log('  --watch        Rebuild on file changes');
  process.exit(0);
}

if (argv.includes('-h') || argv.includes('--help')) {
  showUsage();
}

// ── Backend file groups ──
const ALL_BACKEND_FILES = [
  // x86-64 (5 files)
  'compiler/base-x86-64.wat',
  'compiler/emit-x86-64.wat',
  'compiler/dispatch.wat',
  'compiler/templates-x86-64.wat',
  'compiler/simd-x86-64.wat',
  // ARM32 (compiler + emitter)
  'compiler/compiler-arm32.wat',
  'compiler/emit-arm32.wat',
  // AArch64 (compiler + emitter)
  'compiler/compiler-aarch64.wat',
  'compiler/emit-aarch64.wat',
];

  // ── Module-level names that collide across backends ──
const COLLIDING_NAMES = [
  // Function names
  '$jit_compile', '$copy_compiled_code', '$copy_code_to', '$jit_reset_state',
  // Template functions (shared between ARM32 and AArch64)
  '$template_unreachable', '$template_nop',
  '$template_block', '$template_loop', '$template_if',
  '$template_else', '$template_end', '$template_br', '$template_br_if',
  '$template_br_table', '$template_return', '$template_return_call',
  '$template_drop', '$template_select',
  '$template_local_get', '$template_local_set', '$template_local_tee',
  '$template_global_get', '$template_global_set',
  '$template_call',
  '$template_memory_size', '$template_memory_grow',
  '$template_table_get', '$template_table_set',
  // i32 load/store
  '$template_i32_load', '$template_i32_load8_s', '$template_i32_load8_u',
  '$template_i32_load16_s', '$template_i32_load16_u',
  '$template_i32_store', '$template_i32_store8', '$template_i32_store16',
  // i64 load/store
  '$template_i64_load', '$template_i64_store',
  // i32 comparisons
  '$template_i32_eq', '$template_i32_ne',
  '$template_i32_lt_s', '$template_i32_lt_u',
  '$template_i32_gt_s', '$template_i32_gt_u',
  '$template_i32_le_s', '$template_i32_le_u',
  '$template_i32_ge_s', '$template_i32_ge_u',
  // i64 comparisons
  '$template_i64_eq', '$template_i64_ne',
  '$template_i64_lt_s', '$template_i64_lt_u',
  '$template_i64_gt_s', '$template_i64_gt_u',
  '$template_i64_le_s', '$template_i64_le_u',
  '$template_i64_ge_s', '$template_i64_ge_u',
  // i32 arithmetic
  '$template_i32_eqz', '$template_i32_clz', '$template_i32_ctz', '$template_i32_popcnt',
  '$template_i32_const',
  '$template_i32_add', '$template_i32_sub', '$template_i32_mul',
  '$template_i32_div_s', '$template_i32_div_u',
  '$template_i32_rem_s', '$template_i32_rem_u',
  '$template_i32_and', '$template_i32_or', '$template_i32_xor',
  '$template_i32_shl', '$template_i32_shr_s', '$template_i32_shr_u',
  '$template_i32_rotl', '$template_i32_rotr',
  // i64 arithmetic
  '$template_i64_eqz', '$template_i64_clz', '$template_i64_ctz', '$template_i64_popcnt',
  '$template_i64_const',
  '$template_i64_add', '$template_i64_sub', '$template_i64_mul',
  '$template_i64_div_s', '$template_i64_div_u',
  '$template_i64_rem_s', '$template_i64_rem_u',
  '$template_i64_and', '$template_i64_or', '$template_i64_xor',
  '$template_i64_shl', '$template_i64_shr_s', '$template_i64_shr_u',
  '$template_i64_rotl', '$template_i64_rotr',
  // Conversions
  '$template_i32_wrap_i64',
  '$template_i32_reinterpret_f32', '$template_f32_reinterpret_i32',
  '$template_i64_reinterpret_f64', '$template_f64_reinterpret_i64',
  '$template_i64_extend_i32_s', '$template_i64_extend_i32_u',
  '$template_i32_trunc_f32_s', '$template_i32_trunc_f32_u',
  '$template_i32_trunc_f64_s', '$template_i32_trunc_f64_u',
  '$template_i32_trunc_sat_f32_s', '$template_i32_trunc_sat_f32_u',
  '$template_i32_trunc_sat_f64_s', '$template_i32_trunc_sat_f64_u',
  '$template_i64_trunc_f32_s', '$template_i64_trunc_f32_u',
  '$template_i64_trunc_f64_s', '$template_i64_trunc_f64_u',
  '$template_i64_trunc_sat_f32_s',
  // JIT state globals
  '$JIT_SLOT_SIZE', '$JIT_STATE',
  '$JS_CODE_PTR', '$JS_CACHE_BASE', '$JS_CACHE_END',
  '$JS_FUNC_IDX', '$JS_RESULT_COUNT', '$JS_STACK_DEPTH',
  '$JS_MAX_STACK', '$JS_LABEL_DEPTH', '$JS_RETURN_EMITTED',
  '$JS_LABEL_OFFSETS', '$JS_LABEL_KINDS', '$JS_LABEL_IF_JZ',
  '$JS_FIXUP_COUNT', '$JS_FIXUP_LABEL', '$JS_FIXUP_OFFSET',
  '$JS_INITIALIZED',
  '$JIT_LABEL_BLOCK', '$JIT_LABEL_LOOP', '$JIT_LABEL_IF',
  '$JIT_ERROR', '$CURRENT_DEC_PTR',
  // ELF output globals
  '$ELF_OUT_BUF', '$ELF_OUT_OFF', '$TEXT_VA', '$BSS_VA',
  '$EHDR_SIZE', '$PHDR_SIZE', '$ELF_STUB_OFF', '$ELF_CODE_OFF',
  '$BSS_SIZE',
  '$BSS_JITGLOBALS', '$BSS_MEM', '$BSS_LOCALS', '$BSS_GLOBALS', '$BSS_TABLE',
  // ARM32/AArch64 globals
  '$OP_PREFIX_FC', '$OP_PREFIX_FD', '$WASM_TYPE_V128',
  '$NEXT_OP', '$RESULT_IN_X0',
  '$BIN_OUT_BUF', '$BIN_OUT_OFF',
  '$REG_X0', '$REG_X1', '$REG_X2', '$REG_X19', '$REG_X20',
  '$REG_X21', '$REG_X22', '$REG_XZR', '$REG_SP',
];

// Map relative backend index → suffix for the 3 groups
// Group 0 = x86-64 (files 0-4), Group 1 = ARM32 (files 5-6), Group 2 = AArch64 (files 7-8)
function getBackendSuffix(relIdx) {
  if (relIdx <= 4) return 'x86_64';
  if (relIdx <= 6) return 'arm32';
  return 'aarch64';
}

/**
 * Remove broken WAT constructs from hand-written ARM32/AArch64 files.
 *
 * These files have structural issues: function bodies without a (func header,
 * stray ) characters, orphaned (if / (else / (call blocks at module level.
 * This function strips all content that would cause parse errors:
 * no valid form begins at module level → dropped.
 */
function cleanBrokenWAT(content) {
  const lines = content.split('\n');
  const cleaned = [];
  let depth = 0;
  let inString = false;

  for (const raw of lines) {
    // Build a "code" view: strip inline comments (but not inside strings)
    let code = '';
    for (let i = 0; i < raw.length; i++) {
      const ch = raw[i];
      if (ch === '"' && (i === 0 || raw[i - 1] !== '\\')) inString = !inString;
      if (ch === ';' && raw[i + 1] === ';' && !inString) break;
      code += ch;
    }

    const trimmed = code.trim();
    const opens = (code.match(/\(/g) || []).length;
    const closes = (code.match(/\)/g) || []).length;

    // Detect orphaned opens at module level
    if (depth === 0 && trimmed.startsWith('(')) {
      const firstForm = trimmed.match(/^\((\w+)/)?.[1] || '';
      // Valid forms that can appear at module level
      if (/^(func|global|import|memory|table|data|elem|type|export|module|start)$/.test(firstForm)) {
        depth += opens - closes;
        cleaned.push(raw);
      }
      // else: orphaned block at module level → drop this line
      continue;
    }

    // At module level, skip stray ) that would make depth negative
    if (depth === 0 && closes > opens && /^\s*\)/.test(trimmed)) {
      continue;
    }

    depth += opens - closes;
    cleaned.push(raw);
  }

  return cleaned.join('\n');
}

/**
 * Rename all colliding module-level names in a backend file by appending
 * a backend-specific suffix. Also renames exports and gives local names
 * to anonymous exported functions.
 */
function renameBackend(content, suffix) {
  let result = content;

  // Rename colliding function/global names
  for (const name of COLLIDING_NAMES) {
    const escaped = name.replace(/\$/g, '\\$');
    // Use negative lookahead to avoid matching inside longer identifiers
    result = result.replace(new RegExp(escaped + '(?![a-zA-Z0-9_])', 'g'), `${name}_${suffix}`);
  }

  // Rename exports
  result = result.replace(/"jit_compile"/g, `"jit_compile_${suffix}"`);
  result = result.replace(/"compile_to_elf"/g, `"compile_to_elf_${suffix}"`);
  result = result.replace(/"compile_to_bin"/g, `"compile_to_bin_${suffix}"`);
  result = result.replace(/"get_compiled_code"/g, `"get_compiled_code_${suffix}"`);

  // Give local names to anonymous exported functions so they can be called
  result = result.replace(
    /\(func\s+\(export "(compile_to_elf|compile_to_bin)_([\w-]+)"\)/g,
    (match, name, arch) => `(func $${name}_${arch} (export "${name}_${arch}")`
  );

  return result;
}

// ── Manifest: just file paths, in dependency order ──
// Files are concatenated as-is. module-header.wat provides (module + host imports.
const MANIFEST = [
  // ── Header: opens (module + host imports ──
  'runtime/module-header.wat',

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

  // ── Layer 3a: Socket/Transport ──
  'net/socket-core.wat',

  // Queue/Buffer/CDC stages (slots 48-50)
  'pipeline/queue-stage.wat',
  'pipeline/buffer-stage.wat',
  'pipeline/cdc-stage.wat',

  // ── Layer 4: Interpreter / Compiler ──
  'compiler/interpreter-core.wat',
  'compiler/wasm-emit.wat',
  'compiler/interpreter.wat',

  // ── Layer 5: Edgerun compiler (IR → graph → resolve → lower → privacy) ──
  'lang/edgerun-ir-core.wat',
  'lang/edgerun-parse.wat',
  'lang/edgerun-resolve.wat',
  'lang/edgerun-lower.wat',
  'lang/edgerun-privacy.wat',

  // ── Layer 5a: Edgerun pipeline stages (slots 46-47) ──
  'pipeline/edgerun-parse-stage.wat',
  'pipeline/edgerun-exec-stage.wat',

  // ── Layer 5b: Missing pipeline stages (slots 139-140) ──
  'pipeline/x25519-scalar-mult-stage.wat',
  'pipeline/aes256-encrypt-stage.wat',

  // ── Layer 6: JIT Compiler backends (all 3, compiler + emitter) ──
  'compiler/base-x86-64.wat',
  'compiler/emit-x86-64.wat',
  'compiler/dispatch.wat',
  'compiler/templates-x86-64.wat',
  'compiler/simd-x86-64.wat',
  'compiler/compiler-arm32.wat',
  'compiler/emit-arm32.wat',
  'compiler/compiler-aarch64.wat',
  'compiler/emit-aarch64.wat',

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
  'crypto/crypto-aes128-gcm.wat',
  'crypto/crypto-aes-ctr.wat',
  'crypto/crypto-aes-block.wat',
  'crypto/crypto-hmac-sha256.wat',
  'crypto/crypto-x25519-scalar.wat',

  // ── Layer 7a: Pipeline crypto stages ──
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
  'pipeline/deflate-decode-stage.wat',
  'pipeline/deflate-encode-stage.wat',
  'pipeline/gzip-decode-stage.wat',
  'pipeline/gzip-encode-stage.wat',
  'pipeline/zlib-decode-stage.wat',
  'pipeline/zlib-encode-stage.wat',
  'pipeline/json-parse-stage.wat',
  'pipeline/wasm-exec-stage.wat',
  'pipeline/percent-decode-stage.wat',
  'pipeline/percent-encode-stage.wat',
  'pipeline/utf8-repair-stage.wat',
  'pipeline/to-lower-stage.wat',
  'pipeline/to-upper-stage.wat',
  'pipeline/hex-decode-compat-stage.wat',
  'pipeline/cp1252-decode-stage.wat',
  'pipeline/pem-compact-stage.wat',
  'pipeline/aes128-encrypt-stage.wat',
  'pipeline/aes128-decrypt-stage.wat',
  'pipeline/uuid-format-stage.wat',
  'pipeline/uuid-parse-stage.wat',
  'pipeline/to-title-case-stage.wat',
  'pipeline/cesu8-to-utf8-stage.wat',
  'pipeline/escape-text-stage.wat',
  'pipeline/unescape-text-stage.wat',
  'pipeline/http-request-line-parse-stage.wat',
  'pipeline/http-status-line-parse-stage.wat',
  'pipeline/http-date-parse-stage.wat',
  'pipeline/http-date-format-stage.wat',
  'pipeline/dns-header-decode-stage.wat',
  'pipeline/dns-header-encode-stage.wat',
  'pipeline/tls-record-decode-stage.wat',
  'pipeline/hpack-huffman-decode-stage.wat',
  'pipeline/http-next-header-stage.wat',
  'pipeline/http-chunk-scan-stage.wat',
  'pipeline/dns-name-decompress-stage.wat',
  'pipeline/tls-clienthello-scan-stage.wat',
  'pipeline/generic-tlv-decode-stage.wat',
  'pipeline/generic-tlv-encode-stage.wat',
  'pipeline/frame-header-decode-stage.wat',
  'pipeline/frame-header-encode-stage.wat',
  'pipeline/dns-question-next-stage.wat',
  'pipeline/dns-rr-next-stage.wat',
  'pipeline/endian-read-stage.wat',
  'pipeline/endian-write-stage.wat',
  'pipeline/rsa-pkcs1-verify-stage.wat',
  'pipeline/rsa-pkcs1-emit-stage.wat',
  'pipeline/oauth-url-encode-stage.wat',
  'pipeline/http-classify-body-stage.wat',
  'pipeline/url-scan-stage.wat',
  'pipeline/x509-cert-scan-stage.wat',
  'pipeline/http2-frame-header-decode-stage.wat',
  'pipeline/http3-frame-header-decode-stage.wat',
  'pipeline/tls-cert-list-scan-stage.wat',
  'pipeline/tls-sni-host-stage.wat',
  'pipeline/rfc3339-parse-stage.wat',
  'pipeline/http-lowercase-header-stage.wat',
  'pipeline/tls-alpn-next-stage.wat',
  'pipeline/der-sequence-decode-stage.wat',
  'pipeline/http-validate-headers-stage.wat',
  'pipeline/http-parse-content-length-stage.wat',
  'pipeline/der-integer-decode-stage.wat',
  'pipeline/der-octet-string-decode-stage.wat',
  'pipeline/der-time-decode-stage.wat',
  'pipeline/tls-cert-entry-next-stage.wat',
  'pipeline/oci-reference-scan-stage.wat',
  'pipeline/ssh-auth-key-scan-stage.wat',
  'pipeline/der-bit-string-decode-stage.wat',
  'pipeline/utf8-scan-stage.wat',

  // ── Layer 7b: Pipeline stages (batch 2 — slots 99-118) ──
  'pipeline/hpack-header-block-scan-stage.wat',
  'pipeline/ws-parse-header-stage.wat',
  'pipeline/ws-write-frame-header-stage.wat',
  'pipeline/tls-dns-name-normalize-stage.wat',
  'pipeline/der-oid-root-decode-stage.wat',
  'pipeline/der-oid-next-arc-stage.wat',
  'pipeline/json-emit-string-stage.wat',
  'pipeline/pem-find-boundaries-stage.wat',
  'pipeline/uri-scan-path-query-stage.wat',
  'pipeline/form-urlencoded-next-pair-stage.wat',
  'pipeline/base64url-encode-stage.wat',
  'pipeline/base64url-decode-stage.wat',
  'pipeline/mac-scan-stage.wat',
  'pipeline/utf8-scan-simd-stage.wat',
  'pipeline/toml-scan-scalar-stage.wat',
  'pipeline/toml-scan-key-value-stage.wat',
  'pipeline/yaml-scan-line-stage.wat',
  'pipeline/oauth-json-value-stage.wat',
  'pipeline/oauth-parse-http-response-stage.wat',
  'pipeline/sdk-seed-shape32-stage.wat',

  // ── Layer 7c: Pipeline stages (batch 3 — slots 119-128) ──
  'pipeline/crc32-unrolled-stage.wat',
  'pipeline/adler32-vec-stage.wat',
  'pipeline/base32hex-encode-stage.wat',
  'pipeline/base32hex-decode-stage.wat',
  'pipeline/varint-decode-stage.wat',
  'pipeline/varint-encode-stage.wat',
  'pipeline/tls-record-header-encode-stage.wat',
  'pipeline/tls-handshake-header-decode-stage.wat',
  'pipeline/http-find-crlf-stage.wat',
  'pipeline/json-unescape-string-stage.wat',

  // ── Layer 7d: Pipeline stages (batch 4 — slots 129-138) ──
  'pipeline/http-find-double-crlf-stage.wat',
  'pipeline/tls-extension-next-stage.wat',
  'pipeline/qpack-prefix-int-decode-stage.wat',
  'pipeline/quic-varint-decode-stage.wat',
  'pipeline/quic-varint-encode-stage.wat',
  'pipeline/yaml-scan-document-stage.wat',
  'pipeline/kv-colon-scan-stage.wat',
  'pipeline/unquote-span-stage.wat',
  'pipeline/bracket-list-next-stage.wat',
  'pipeline/host-port-scan-stage.wat',

  // ── Layer 8: Protocol parsers ──
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

  // ── Layer 9: Codec ──
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

  // ── Layer 10: UI Framework ──
  'ui/ui_framework.wat',

  // ── Layer 11: UI Pipeline Stages (slots 141-143) ──
  'pipeline/ui-layout-stage.wat',
  'pipeline/ui-paint-stage.wat',
  'pipeline/ui-event-stage.wat',

  // ── Layer 12: App ──
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
  'app/platform.wat',

  // ── Layer 14: Data ──
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

  // ── Layer 15: Codebase Metadata (auto-generated) ──
  'data/generated-metadata.wat',

  // ── Layer 17: Stage Registry (last — elem entries reference process_* functions) ──
  'pipeline/stage-registry.wat',

  // ── Layer 18: JIT dispatch wrapper ──
  'compiler/dispatch-wrapper.wat',

  // ── Layer 19: Wayland ELF pipeline ──
  'app/wayland-patch-syscalls.wat',

  // ── Footer: closes (module ──
  'runtime/module-footer.wat',
];

function build() {
  const outPath = resolve(ROOT, argv.find((a) => a.startsWith('--out='))?.slice(6) || 'edgerun.wat');
  const skipWasm = process.argv.includes('--no-wasm');
  const noMeta = process.argv.includes('--no-meta');

  // Auto-generate metadata tables
  if (!noMeta) {
    const metaPath = resolve(ROOT, 'data/generated-metadata.wat');
    const metaScript = resolve(ROOT, 'tools/generate_metadata.mjs');
    try {
      execSync(`bun "${metaScript}" --out="${metaPath}"`, { stdio: 'pipe' });
    } catch (e) {
      console.warn(`  ⚠  metadata generation failed: ${e.stderr?.slice(0, 200) || e.message}`);
    }
  }

  console.log(`EdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log(`All 3 JIT backends: x86-64 + ARM32 + AArch64\n`);

  // Map backend file index to its 0/1/2 group
  const backendFileStart = MANIFEST.findIndex(f => f === 'compiler/base-x86-64.wat');
  const backendFileEnd = backendFileStart + 9; // 9 backend files (5 x86-64 + 2 ARM32 + 2 AArch64)

  let body = '';
  let count = 0;
  let fileIdx = 0;

  for (const filePath of MANIFEST) {
    // Skip metadata when --no-meta is active
    if (noMeta && filePath === 'data/generated-metadata.wat') {
      fileIdx++;
      continue;
    }
    const fullPath = resolve(ROOT, filePath);
    if (!existsSync(fullPath)) {
      console.warn(`  ⚠  ${filePath} not found — skipping`);
      fileIdx++;
      continue;
    }
    let content = readFileSync(fullPath, 'utf-8');

    // Apply backend renaming for JIT backend files
    if (fileIdx >= backendFileStart && fileIdx < backendFileEnd) {
      const relIdx = fileIdx - backendFileStart;
      const isArmAarch64 = relIdx >= 5; // files 5-8 = arm32 + aarch64 (compiler + emitter)
      if (isArmAarch64) {
        content = cleanBrokenWAT(content);
      }
      content = renameBackend(content, getBackendSuffix(relIdx));
    }

    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
    fileIdx++;
  }

  // (dispatch-wrapper.wat is now in MANIFEST — no separate append needed)

  writeFileSync(outPath, body, 'utf-8');
  console.log(`✓ ${count} fragments → ${outPath} (${body.length} bytes)`);

  if (!skipWasm) {
    const wasmPath = outPath.replace(/\.wat$/, '.wasm');
    console.log(`Compiling → ${wasmPath}...`);
    const wasmTools = resolveTool('wasm-tools');
    if (wasmTools) {
      try {
        execSync(`"${wasmTools}" parse "${outPath}" -o "${wasmPath}"`, { stdio: 'pipe' });
        let wSize = statSync(wasmPath).size;
        console.log(`  ✓ ${wasmPath} (${(wSize / 1024).toFixed(0)} KB)`);

        // Strip debug names (saves ~20%)
        const strippedPath = wasmPath.replace(/\.wasm$/, '-stripped.wasm');
        try {
          execSync(`"${wasmTools}" strip --all "${wasmPath}" -o "${strippedPath}"`, { stdio: 'pipe' });
          const sSize = statSync(strippedPath).size;
          const saved = ((wSize - sSize) / wSize * 100).toFixed(0);
          console.log(`  ✓ Stripped → ${strippedPath} (${(sSize / 1024).toFixed(0)} KB, -${saved}%)`);
        } catch {
          console.warn('  ⚠  strip skipped');
        }

        // Optimize with wasm-opt if available
        const wasmOpt = resolveTool('wasm-opt');
        if (wasmOpt) {
          const optPath = wasmPath.replace(/\.wasm$/, '-opt.wasm');
          try {
            execSync(`"${wasmOpt}" -Oz "${wasmPath}" -o "${optPath}"`, { stdio: 'pipe' });
            const oSize = statSync(optPath).size;
            const saved = ((wSize - oSize) / wSize * 100).toFixed(0);
            console.log(`  ✓ wasm-opt -Oz → ${optPath} (${(oSize / 1024).toFixed(0)} KB, -${saved}%)`);
          } catch {
            console.warn('  ⚠  wasm-opt skipped (not found)');
          }
        }
      } catch (e) {
        console.error(`  ✗ wasm-tools parse failed: ${e.stderr?.slice(0, 500) || e.message}`);
      }
    } else {
      console.warn('  ⚠  wasm-tools not found — skipping WASM compilation');
    }
  }

  return true;
}

function resolveTool(name) {
  try {
    const out = execSync(`which ${name}`, { encoding: 'utf-8' }).trim();
    return out || null;
  } catch { return null; }
}

// ── Watch mode ──
function watchMode() {
  const { watch } = require('fs');
  const sourceDirs = ['runtime', 'compiler', 'pipeline', 'crypto', 'protocol', 'codec',
                       'ui', 'app', 'data', 'device', 'net', 'system', 'tools', 'lang'];
  console.log(`Watching for changes in ${sourceDirs.join(', ')}...\n`);
  let timer = null;
  const onChange = () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      try { build(); } catch (e) { console.error(`Build error: ${e.message}`); }
    }, 200);
  };
  for (const dir of sourceDirs) {
    const dirPath = resolve(ROOT, dir);
    if (!existsSync(dirPath)) continue;
    try {
      watch(dirPath, { recursive: true }, (event, filename) => {
        if (filename?.endsWith('.wat')) onChange();
      });
    } catch {}
  }
  watch(resolve(ROOT, 'tools/build_wat.mjs'), () => onChange());
  build();
}

if (process.argv.includes('--watch')) {
  watchMode();
} else {
  build();
}
