#!/usr/bin/env bun
/**
 * EdgeRun Build System — ZERO post-processing.
 *
 * Concatenates source fragments in order, wraps in (module), compiles to WASM.
 * No stripping, no renaming, no deduplication, no anonymous export fixing.
 * Source files must already be clean fragments (no module wrappers, no
 * internal imports, no memory declarations — those live in runtime/).
 */

import { readFileSync, writeFileSync, existsSync, statSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

const ROOT = resolve(import.meta.dirname, '..');
const argv = process.argv.slice(2);

function getArgValue(name, fallback = null) {
  const eqArg = argv.find((item) => item.startsWith(`${name}=`));
  if (eqArg) return eqArg.split('=', 2)[1];
  const idx = argv.indexOf(name);
  if (idx >= 0) {
    const next = argv[idx + 1];
    if (next && !next.startsWith('--')) {
      return next;
    }
  }
  return fallback;
}

function showUsage() {
  console.log('Usage: bun tools/build_wat.mjs [--arch=<x86-64|arm32|aarch64>] [--out=<path>] [--no-wasm] [--watch]');
  console.log('Options:');
  console.log('  --arch         Compiler backend (default: x86-64, env: EDGERUN_COMPILER_ARCH)');
  console.log('  --out          Output WAT path (default: edgerun.wat)');
  console.log('  --no-wasm      Skip WASM compile step');
  console.log('  --watch        Rebuild on file changes');
  process.exit(0);
}

if (argv.includes('-h') || argv.includes('--help')) {
  showUsage();
}

function normalizeCompilerArch(value) {
  const normalized = (value || '').toLowerCase();
  if (!normalized) return 'x86-64';
  if (normalized === 'x86-64' || normalized === 'x86_64' || normalized === 'x86') return 'x86-64';
  if (normalized === 'arm32' || normalized === 'armv7' || normalized === 'armv7-a') return 'arm32';
  if (normalized === 'aarch64' || normalized === 'arm64') return 'aarch64';
  return null;
}

const COMPILER_ARCH = normalizeCompilerArch(
  getArgValue('--arch', process.env.EDGERUN_COMPILER_ARCH || 'x86-64')
);
if (!COMPILER_ARCH) {
  throw new Error('Invalid --arch value. Use --arch=x86-64|arm32|aarch64');
}

const COMPILER_BACKENDS = {
  'x86-64': [
    'compiler/base-x86-64.wat',
    'compiler/emit-x86-64.wat',
    'compiler/dispatch.wat',
    'compiler/templates-x86-64.wat',
    'compiler/simd-x86-64.wat',
  ],
  arm32: [
    'compiler/compiler-arm32.wat',
  ],
  aarch64: [
    'compiler/compiler-aarch64.wat',
  ],
};

const COMPILER_BACKEND_SLOT = '__ER_COMPILER_BACKEND__';

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

  // Queue/Buffer/CDC stages (slots 48-50)
  'pipeline/queue-stage.wat',
  'pipeline/buffer-stage.wat',
  'pipeline/cdc-stage.wat',

  // ── Layer 4: Interpreter / Compiler ──
  'compiler/interpreter-core.wat',
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

  // ── Layer 6: JIT Compiler backend ──
  COMPILER_BACKEND_SLOT,

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

  // ── Layer 12: App ──
  'app/repo-dashboard.wat',
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

  // ── Layer 17: Stage Registry (last — elem entries reference process_* functions) ──
  'pipeline/stage-registry.wat',

  // ── Footer: closes (module ──
  'runtime/module-footer.wat',
];

function build() {
  const outPath = resolve(ROOT, argv.find((a) => a.startsWith('--out='))?.slice(6) || 'edgerun.wat');
  const skipWasm = process.argv.includes('--no-wasm');
  const selectedJitBackend = COMPILER_BACKENDS[COMPILER_ARCH];
  const resolvedManifest = [];

  for (const filePath of MANIFEST) {
    if (filePath === COMPILER_BACKEND_SLOT) {
      resolvedManifest.push(...selectedJitBackend);
    } else {
      resolvedManifest.push(filePath);
    }
  }

  console.log(`EdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log(`Target compiler backend: ${COMPILER_ARCH}\n`);

  let body = '';
  let count = 0;

  for (const filePath of resolvedManifest) {
    const fullPath = resolve(ROOT, filePath);
    if (!existsSync(fullPath)) {
      console.warn(`  ⚠  ${filePath} not found — skipping`);
      continue;
    }
    const content = readFileSync(fullPath, 'utf-8');
    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
  }

  writeFileSync(outPath, body, 'utf-8');
  console.log(`✓ ${count} fragments → ${outPath} (${body.length} bytes)`);

  if (!skipWasm) {
    const wasmPath = outPath.replace(/\.wat$/, '.wasm');
    console.log(`Compiling → ${wasmPath}...`);
    const wasmTools = resolveTool('wasm-tools');
    if (wasmTools) {
      try {
        execSync(`"${wasmTools}" parse "${outPath}" -o "${wasmPath}"`, { stdio: 'pipe' });
        const wSize = statSync(wasmPath).size;
        console.log(`  ✓ ${wasmPath} (${(wSize / 1024).toFixed(0)} KB)`);
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
