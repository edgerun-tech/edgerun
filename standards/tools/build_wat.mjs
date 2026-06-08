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
import {
  resolveRoot, rootPath, fileExists, concatFragments,
  compileWat, stripWasm, optimizeWasm
} from './build-lib.mjs';

const ROOT = resolveRoot();
const argv = process.argv.slice(2);

function showUsage() {
  console.log('Usage: bun tools/build_wat.mjs [--out=<path>] [--no-wasm] [--watch]');
  console.log('Options:');
  console.log('  --out          Output WAT path (default: edgerun.wat)');
  console.log('  --no-wasm      Skip WASM compile step');
  console.log('  --watch        Rebuild on file changes');
  process.exit(0);
}

if (argv.includes('-h') || argv.includes('--help')) showUsage();

// ── Backend file groups ──
const BACKEND_FILES = [
  'compiler/base-x86-64.wat', 'compiler/emit-x86-64.wat',
  'compiler/dispatch.wat', 'compiler/templates-x86-64.wat', 'compiler/simd-x86-64.wat',
  'compiler/compiler-arm32.wat', 'compiler/emit-arm32.wat',
  'compiler/compiler-aarch64.wat', 'compiler/emit-aarch64.wat',
];

// ── Module-level names that collide across backends ──
const COLLIDING_NAMES = [
  '$jit_compile', '$copy_compiled_code', '$copy_code_to', '$jit_reset_state',
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
  '$template_i32_load', '$template_i32_load8_s', '$template_i32_load8_u',
  '$template_i32_load16_s', '$template_i32_load16_u',
  '$template_i32_store', '$template_i32_store8', '$template_i32_store16',
  '$template_i64_load', '$template_i64_store',
  '$template_i32_eq', '$template_i32_ne',
  '$template_i32_lt_s', '$template_i32_lt_u',
  '$template_i32_gt_s', '$template_i32_gt_u',
  '$template_i32_le_s', '$template_i32_le_u',
  '$template_i32_ge_s', '$template_i32_ge_u',
  '$template_i64_eq', '$template_i64_ne',
  '$template_i64_lt_s', '$template_i64_lt_u',
  '$template_i64_gt_s', '$template_i64_gt_u',
  '$template_i64_le_s', '$template_i64_le_u',
  '$template_i64_ge_s', '$template_i64_ge_u',
  '$template_i32_eqz', '$template_i32_clz', '$template_i32_ctz', '$template_i32_popcnt',
  '$template_i32_const',
  '$template_i32_add', '$template_i32_sub', '$template_i32_mul',
  '$template_i32_div_s', '$template_i32_div_u',
  '$template_i32_rem_s', '$template_i32_rem_u',
  '$template_i32_and', '$template_i32_or', '$template_i32_xor',
  '$template_i32_shl', '$template_i32_shr_s', '$template_i32_shr_u',
  '$template_i32_rotl', '$template_i32_rotr',
  '$template_i64_eqz', '$template_i64_clz', '$template_i64_ctz', '$template_i64_popcnt',
  '$template_i64_const',
  '$template_i64_add', '$template_i64_sub', '$template_i64_mul',
  '$template_i64_div_s', '$template_i64_div_u',
  '$template_i64_rem_s', '$template_i64_rem_u',
  '$template_i64_and', '$template_i64_or', '$template_i64_xor',
  '$template_i64_shl', '$template_i64_shr_s', '$template_i64_shr_u',
  '$template_i64_rotl', '$template_i64_rotr',
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
  '$JIT_SLOT_SIZE', '$JIT_STATE',
  '$JS_CODE_PTR', '$JS_CACHE_BASE', '$JS_CACHE_END',
  '$JS_FUNC_IDX', '$JS_RESULT_COUNT', '$JS_STACK_DEPTH',
  '$JS_MAX_STACK', '$JS_LABEL_DEPTH', '$JS_RETURN_EMITTED',
  '$JS_LABEL_OFFSETS', '$JS_LABEL_KINDS', '$JS_LABEL_IF_JZ',
  '$JS_FIXUP_COUNT', '$JS_FIXUP_LABEL', '$JS_FIXUP_OFFSET',
  '$JS_INITIALIZED',
  '$JIT_LABEL_BLOCK', '$JIT_LABEL_LOOP', '$JIT_LABEL_IF',
  '$JIT_ERROR', '$CURRENT_DEC_PTR',
  '$ELF_OUT_BUF', '$ELF_OUT_OFF', '$TEXT_VA', '$BSS_VA',
  '$EHDR_SIZE', '$PHDR_SIZE', '$ELF_STUB_OFF', '$ELF_CODE_OFF',
  '$BSS_SIZE',
  '$BSS_JITGLOBALS', '$BSS_MEM', '$BSS_LOCALS', '$BSS_GLOBALS', '$BSS_TABLE',
  '$OP_PREFIX_FC', '$OP_PREFIX_FD', '$WASM_TYPE_V128',
  '$NEXT_OP', '$RESULT_IN_X0',
  '$BIN_OUT_BUF', '$BIN_OUT_OFF',
  '$REG_X0', '$REG_X1', '$REG_X2', '$REG_X19', '$REG_X20',
  '$REG_X21', '$REG_X22', '$REG_XZR', '$REG_SP',
];

function getBackendSuffix(relIdx) {
  if (relIdx <= 4) return 'x86_64';
  if (relIdx <= 6) return 'arm32';
  return 'aarch64';
}

function cleanBrokenWAT(content) {
  const lines = content.split('\n');
  const cleaned = [];
  let depth = 0;
  let inString = false;

  for (const raw of lines) {
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

    if (depth === 0 && trimmed.startsWith('(')) {
      const firstForm = trimmed.match(/^\((\w+)/)?.[1] || '';
      if (/^(func|global|import|memory|table|data|elem|type|export|module|start)$/.test(firstForm)) {
        depth += opens - closes;
        cleaned.push(raw);
      }
      continue;
    }

    if (depth === 0 && closes > opens && /^\s*\)/.test(trimmed)) continue;

    depth += opens - closes;
    cleaned.push(raw);
  }

  return cleaned.join('\n');
}

function renameBackend(content, suffix) {
  let result = content;

  for (const name of COLLIDING_NAMES) {
    const escaped = name.replace(/\$/g, '\\$');
    result = result.replace(new RegExp(escaped + '(?![a-zA-Z0-9_])', 'g'), `${name}_${suffix}`);
  }

  result = result.replace(/"jit_compile"/g, `"jit_compile_${suffix}"`);
  result = result.replace(/"compile_to_elf"/g, `"compile_to_elf_${suffix}"`);
  result = result.replace(/"compile_to_bin"/g, `"compile_to_bin_${suffix}"`);
  result = result.replace(/"get_compiled_code"/g, `"get_compiled_code_${suffix}"`);

  result = result.replace(
    /\(func\s+\(export "(compile_to_elf|compile_to_bin)_([\w-]+)"\)/g,
    (match, name, arch) => `(func $${name}_${arch} (export "${name}_${arch}")`
  );

  return result;
}

// ── Manifest ──
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
  'net/socket-core.wat',
  'pipeline/queue-stage.wat',
  'pipeline/buffer-stage.wat',
  'pipeline/cdc-stage.wat',
  'compiler/interpreter-core.wat',
  'compiler/wasm-emit.wat',
  'compiler/interpreter.wat',
  'lang/edgerun-ir-core.wat',
  'lang/edgerun-parse.wat',
  'lang/edgerun-resolve.wat',
  'lang/edgerun-lower.wat',
  'lang/edgerun-privacy.wat',
  'pipeline/edgerun-parse-stage.wat',
  'pipeline/edgerun-exec-stage.wat',
  'pipeline/x25519-scalar-mult-stage.wat',
  'pipeline/aes256-encrypt-stage.wat',
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
  'crypto/crypto-aes-block.wat',
  'crypto/crypto-hmac-sha256.wat',
  'crypto/crypto-x25519-scalar.wat',
  'pipeline/sha256-stage.wat', 'pipeline/hmac-sha256-stage.wat',
  'pipeline/sha1-stage.wat', 'pipeline/sha512-stage.wat', 'pipeline/sha384-stage.wat',
  'pipeline/aes128-gcm-encrypt-stage.wat', 'pipeline/aes128-gcm-decrypt-stage.wat',
  'pipeline/aes128-ctr-xor-stage.wat', 'pipeline/djb2-hash-stage.wat',
  'pipeline/inet-checksum-stage.wat', 'pipeline/crc32-bzip-stage.wat',
  'pipeline/deflate-decode-stage.wat', 'pipeline/deflate-encode-stage.wat',
  'pipeline/gzip-decode-stage.wat', 'pipeline/gzip-encode-stage.wat',
  'pipeline/zlib-decode-stage.wat', 'pipeline/zlib-encode-stage.wat',
  'pipeline/json-parse-stage.wat', 'pipeline/wasm-exec-stage.wat',
  'pipeline/percent-decode-stage.wat', 'pipeline/percent-encode-stage.wat',
  'pipeline/utf8-repair-stage.wat', 'pipeline/to-lower-stage.wat', 'pipeline/to-upper-stage.wat',
  'pipeline/hex-decode-compat-stage.wat', 'pipeline/cp1252-decode-stage.wat',
  'pipeline/pem-compact-stage.wat', 'pipeline/aes128-encrypt-stage.wat', 'pipeline/aes128-decrypt-stage.wat',
  'pipeline/uuid-format-stage.wat', 'pipeline/uuid-parse-stage.wat', 'pipeline/to-title-case-stage.wat',
  'pipeline/cesu8-to-utf8-stage.wat', 'pipeline/escape-text-stage.wat', 'pipeline/unescape-text-stage.wat',
  'pipeline/http-request-line-parse-stage.wat', 'pipeline/http-status-line-parse-stage.wat',
  'pipeline/http-date-parse-stage.wat', 'pipeline/http-date-format-stage.wat',
  'pipeline/dns-header-decode-stage.wat', 'pipeline/dns-header-encode-stage.wat',
  'pipeline/tls-record-decode-stage.wat', 'pipeline/hpack-huffman-decode-stage.wat',
  'pipeline/http-next-header-stage.wat', 'pipeline/http-chunk-scan-stage.wat',
  'pipeline/dns-name-decompress-stage.wat', 'pipeline/tls-clienthello-scan-stage.wat',
  'pipeline/generic-tlv-decode-stage.wat', 'pipeline/generic-tlv-encode-stage.wat',
  'pipeline/frame-header-decode-stage.wat', 'pipeline/frame-header-encode-stage.wat',
  'pipeline/dns-question-next-stage.wat', 'pipeline/dns-rr-next-stage.wat',
  'pipeline/endian-read-stage.wat', 'pipeline/endian-write-stage.wat',
  'pipeline/rsa-pkcs1-verify-stage.wat', 'pipeline/rsa-pkcs1-emit-stage.wat',
  'pipeline/oauth-url-encode-stage.wat',
  'pipeline/http-classify-body-stage.wat', 'pipeline/url-scan-stage.wat',
  'pipeline/x509-cert-scan-stage.wat', 'pipeline/http2-frame-header-decode-stage.wat',
  'pipeline/http3-frame-header-decode-stage.wat', 'pipeline/tls-cert-list-scan-stage.wat',
  'pipeline/tls-sni-host-stage.wat', 'pipeline/rfc3339-parse-stage.wat',
  'pipeline/http-lowercase-header-stage.wat', 'pipeline/tls-alpn-next-stage.wat',
  'pipeline/der-sequence-decode-stage.wat', 'pipeline/http-validate-headers-stage.wat',
  'pipeline/http-parse-content-length-stage.wat', 'pipeline/der-integer-decode-stage.wat',
  'pipeline/der-octet-string-decode-stage.wat', 'pipeline/der-time-decode-stage.wat',
  'pipeline/tls-cert-entry-next-stage.wat', 'pipeline/oci-reference-scan-stage.wat',
  'pipeline/ssh-auth-key-scan-stage.wat', 'pipeline/der-bit-string-decode-stage.wat',
  'pipeline/utf8-scan-stage.wat',
  'pipeline/hpack-header-block-scan-stage.wat', 'pipeline/ws-parse-header-stage.wat',
  'pipeline/ws-write-frame-header-stage.wat', 'pipeline/tls-dns-name-normalize-stage.wat',
  'pipeline/der-oid-root-decode-stage.wat', 'pipeline/der-oid-next-arc-stage.wat',
  'pipeline/json-emit-string-stage.wat', 'pipeline/pem-find-boundaries-stage.wat',
  'pipeline/uri-scan-path-query-stage.wat', 'pipeline/form-urlencoded-next-pair-stage.wat',
  'pipeline/base64url-encode-stage.wat', 'pipeline/base64url-decode-stage.wat',
  'pipeline/mac-scan-stage.wat', 'pipeline/utf8-scan-simd-stage.wat',
  'pipeline/toml-scan-scalar-stage.wat', 'pipeline/toml-scan-key-value-stage.wat',
  'pipeline/yaml-scan-line-stage.wat', 'pipeline/oauth-json-value-stage.wat',
  'pipeline/oauth-parse-http-response-stage.wat', 'pipeline/sdk-seed-shape32-stage.wat',
  'pipeline/crc32-unrolled-stage.wat', 'pipeline/adler32-vec-stage.wat',
  'pipeline/base32hex-encode-stage.wat', 'pipeline/base32hex-decode-stage.wat',
  'pipeline/varint-decode-stage.wat', 'pipeline/varint-encode-stage.wat',
  'pipeline/tls-record-header-encode-stage.wat', 'pipeline/tls-handshake-header-decode-stage.wat',
  'pipeline/http-find-crlf-stage.wat', 'pipeline/json-unescape-string-stage.wat',
  'pipeline/http-find-double-crlf-stage.wat', 'pipeline/tls-extension-next-stage.wat',
  'pipeline/qpack-prefix-int-decode-stage.wat', 'pipeline/quic-varint-decode-stage.wat',
  'pipeline/quic-varint-encode-stage.wat', 'pipeline/yaml-scan-document-stage.wat',
  'pipeline/kv-colon-scan-stage.wat', 'pipeline/unquote-span-stage.wat',
  'pipeline/bracket-list-next-stage.wat', 'pipeline/host-port-scan-stage.wat',
  'protocol/binary-core.wat', 'protocol/cache.wat', 'protocol/der.wat', 'protocol/dhcp.wat',
  'protocol/dns.wat', 'protocol/hpack.wat', 'protocol/http.wat', 'protocol/ipv4-net.wat',
  'protocol/packet-core.wat', 'protocol/quic.wat', 'protocol/sfx-tables.wat', 'protocol/tls.wat',
  'protocol/ws.wat',
  'codec/checksum-core.wat', 'codec/base64.wat', 'codec/json.wat', 'codec/compress.wat',
  'codec/text.wat', 'codec/serialize.wat', 'codec/binary.wat', 'codec/string-url.wat',
  'codec/media.wat', 'codec/model.wat',
  'pipeline/base64-encode-stage.wat', 'pipeline/base64-decode-stage.wat',
  'ui/ui_framework.wat',
  'pipeline/ui-layout-stage.wat', 'pipeline/ui-paint-stage.wat', 'pipeline/ui-event-stage.wat',
  'app/oauth.wat', 'app/oci.wat', 'app/codex.wat', 'app/wallet.wat',
  'app/security.wat', 'app/identity.wat', 'app/net.wat', 'app/core.wat',
  'app/ui.wat', 'app/wayland.wat', 'app/platform.wat',
  'data/hex-core.wat', 'data/byte-search.wat', 'data/cache-archive-split.wat',
  'data/color-util.wat', 'data/glob-match.wat', 'data/html-strip.wat',
  'data/mime-quoted-printable.wat', 'data/secret-service-core.wat', 'data/shell-word-scan.wat',
  'data/sse-event-stream.wat', 'data/storage-core-index.wat', 'data/string-distance.wat',
  'data/tag-list.wat', 'data/terminal-control-scan.wat', 'data/terminal-state-core.wat',
  'data/uuid-util.wat',
  'data/generated-metadata.wat',
  'pipeline/stage-registry.wat',
  'compiler/dispatch-wrapper.wat',
  'app/wayland-patch-syscalls.wat',
  'runtime/module-footer.wat',
];

function build() {
  const outPath = resolve(ROOT, argv.find((a) => a.startsWith('--out='))?.slice(6) || 'edgerun.wat');
  const skipWasm = process.argv.includes('--no-wasm');
  const noMeta = process.argv.includes('--no-meta');

  if (!noMeta) {
    const metaScript = rootPath('tools/generate_metadata.mjs');
    const metaPath = rootPath('data/generated-metadata.wat');
    try {
      execSync(`bun "${metaScript}" --out="${metaPath}"`, { stdio: 'pipe' });
    } catch (e) {
      console.warn(`  ⚠  metadata generation failed: ${e.stderr?.slice(0, 200) || e.message}`);
    }
  }

  console.log(`EdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log(`All 3 JIT backends: x86-64 + ARM32 + AArch64\n`);

  const backendFileStart = MANIFEST.findIndex(f => f === 'compiler/base-x86-64.wat');
  const backendFileEnd = backendFileStart + 9;

  let body = '';
  let count = 0;
  let fileIdx = 0;

  for (const filePath of MANIFEST) {
    if (noMeta && filePath === 'data/generated-metadata.wat') { fileIdx++; continue; }
    const fullPath = resolve(ROOT, filePath);
    if (!existsSync(fullPath)) { console.warn(`  ⚠  ${filePath} not found — skipping`); fileIdx++; continue; }
    let content = readFileSync(fullPath, 'utf-8');

    if (fileIdx >= backendFileStart && fileIdx < backendFileEnd) {
      const relIdx = fileIdx - backendFileStart;
      const isArmAarch64 = relIdx >= 5;
      if (isArmAarch64) content = cleanBrokenWAT(content);
      content = renameBackend(content, getBackendSuffix(relIdx));
    }

    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
    fileIdx++;
  }

  writeFileSync(outPath, body, 'utf-8');
  console.log(`✓ ${count} fragments → ${outPath} (${body.length} bytes)`);

  if (!skipWasm) {
    const wasmPath = outPath.replace(/\.wat$/, '.wasm');
    console.log(`Compiling → ${wasmPath}...`);
    if (compileWat(outPath, wasmPath)) {
      stripWasm(wasmPath);
      optimizeWasm(wasmPath);
    }
  }

  return true;
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
