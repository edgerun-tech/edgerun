#!/usr/bin/env node
/**
 * Protocol Family Merger v5 — handles all edge cases
 *
 * Usage:
 *   node tools/merge-protocol.mjs [--dry-run]
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');

const FAMILIES = [
  {
    name: 'http', outfile: 'protocol/http.wat', internal: ['http'],
    files: [
      'protocol/http-core.wat', 'protocol/http1-body.wat',
      'protocol/http1-chunk-stream.wat', 'protocol/http1-header-block.wat',
      'protocol/http1-lines.wat', 'protocol/http1-scan.wat',
      'protocol/http2-frame.wat', 'protocol/http3-frame.wat',
      'protocol/http-client-core.wat', 'protocol/http-date.wat',
      'protocol/http-node-state.wat', 'protocol/http-prefix-int.wat',
    ],
    missingImports: [],
    sourceFixes: [],
  },
  {
    name: 'dns', outfile: 'protocol/dns.wat', internal: ['dns'],
    files: [
      'protocol/dns-core.wat', 'protocol/dns-compressed-name.wat',
      'protocol/dns-core-records.wat', 'protocol/dns-message-header.wat',
      'protocol/dns-name.wat', 'protocol/dns-rdata-core.wat',
      'protocol/dns-resolve.wat', 'protocol/dns-section-walk.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "host" "sock_open" (func $sock_open (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_send" (func $sock_send (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_recv" (func $sock_recv (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_close" (func $sock_close (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'tls', outfile: 'protocol/tls.wat', internal: [],
    files: [
      'protocol/tls-certificate-list.wat', 'protocol/tls-clienthello.wat',
      'protocol/tls-core-state.wat', 'protocol/tls-extension-walk.wat',
      'protocol/tls-frame.wat', 'protocol/tls-name.wat',
      'protocol/tls-vector.wat',
    ],
    missingImports: [
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_alnum" (func $is_alnum (param i32) (result i32)))',
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'hpack', outfile: 'protocol/hpack.wat', internal: ['hpack-qpack'],
    files: [
      'protocol/hpack-qpack-core.wat', 'protocol/hpack-header-block.wat',
      'protocol/hpack-huffman.wat', 'protocol/hpack-string.wat',
      'protocol/hpack-table-core.wat', 'protocol/qpack-decoder-stream.wat',
      'protocol/qpack-encoder-stream.wat', 'protocol/qpack-string.wat',
      'protocol/qpack-table-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
    ],
    sourceFixes: [
      // Fix missing $ prefix on function calls (all files in family)
      { file: '*', find: '(call huffman_decode_internal', replace: '(call $huffman_decode_internal' },
      { file: '*', find: '(call qpack_prefix_decode', replace: '(call $qpack_prefix_decode' },
      { file: '*', find: '(call prefix_encode', replace: '(call $prefix_encode' },
    ],
  },
  {
    name: 'ws', outfile: 'protocol/ws.wat', internal: [],
    files: [
      'protocol/ws-accept.wat', 'protocol/ws-client.wat',
      'protocol/ws-frame.wat', 'protocol/ws-stage.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))',
      '(import "crypto" "sha1" (func $sha1 (param i32 i32 i32) (result i64)))',
      '(import "codec" "base64_encode" (func $base64_encode (param i32 i32 i32 i32) (result i64)))',
      '(import "host" "sock_open" (func $sock_open (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_send" (func $sock_send (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_recv" (func $sock_recv (param i32 i32 i32) (result i32)))',
      '(import "host" "sock_close" (func $sock_close (param i32) (result i32)))',
      '(import "pipeline" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))',
      '(import "pipeline" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))',
      '(import "edgerun" "STATUS_MORE" (global $STATUS_MORE i32))',
      '(import "edgerun" "STATUS_TIMEOUT" (global $STATUS_TIMEOUT i32))',
    ],
  },
  {
    name: 'quic', outfile: 'protocol/quic.wat', internal: ['quic'],
    files: [
      'protocol/quic-core.wat', 'protocol/quic-core-state.wat',
    ],
    missingImports: [],
    sourceFixes: [],
    stripExports: ['quic_varint_decode_at'],
  },
  {
    name: 'dhcp', outfile: 'protocol/dhcp.wat', internal: [],
    files: [
      'protocol/dhcp-message-core.wat', 'protocol/dhcpv6-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'der', outfile: 'protocol/der.wat', internal: [],
    files: [
      'protocol/der-asn1-basic.wat', 'protocol/der-integer.wat',
      'protocol/der-oid.wat', 'protocol/der-time.wat',
      'protocol/der-tlv.wat',
    ],
    missingImports: [],
    sourceFixes: [],
  },
  {
    name: 'cache', outfile: 'protocol/cache.wat', internal: [],
    files: [
      'protocol/cache-index.wat', 'protocol/js5-cache.wat',
    ],
    missingImports: [],
    sourceFixes: [],
    dedupGlobals: ['$ERR_BOUNDS'],
  },
  // ════════════════════════════════════════════════════════════════
  // CODEC families (38 files → 9 files)
  // ════════════════════════════════════════════════════════════════
  {
    name: 'codec-base64', outfile: 'codec/base64.wat', internal: [],
    files: [
      'codec/encoding-base64.wat', 'codec/encoding-base64url.wat',
      'codec/encoding-base32hex.wat', 'codec/encoding-core.wat',
      'codec/encoding-text.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))',
    ],
    sourceFixes: [
      // Standardize local names for load8_u
      { file: '*', find: '(import "edgerun" "load8_u"', replace: ';; stripped (load8_u not needed)' },
    ],
    // encoding-text.wat references $b64_encode/$b64_decode from encoding-base64url — internal to family
  },
  {
    name: 'codec-json', outfile: 'codec/json.wat', internal: [],
    files: [
      'codec/json-value-core.wat', 'codec/json-emit.wat',
      'codec/json-scalar.wat', 'codec/json-tape.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-compress', outfile: 'codec/compress.wat', internal: [],
    files: [
      'codec/deflate-inflate.wat', 'codec/deflate-stored.wat',
      'codec/gzip-member.wat', 'codec/zlib-wrapper.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-text', outfile: 'codec/text.wat', internal: [],
    files: [
      'codec/utf8-scan.wat', 'codec/cesu8-mutf8.wat',
      'codec/encoding-cp1252.wat', 'codec/case-convert.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
      '(import "edgerun" "is_cont" (func $is_cont (param i32) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "to_upper" (func $to_upper (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-serialize', outfile: 'codec/serialize.wat', internal: [],
    files: [
      'codec/toml-scan.wat', 'codec/yaml-scan.wat', 'codec/kv-scan.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-binary', outfile: 'codec/binary.wat', internal: [],
    files: [
      'codec/hex-compat.wat', 'codec/endian-fixed.wat',
      'codec/integer-decimal.wat', 'codec/number-format-suffix.wat',
      'codec/length-field.wat', 'codec/length-frame.wat',
      'codec/generic-tlv.wat', 'codec/host-port.wat',
      'codec/encoding-smart-int.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
      '(import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-string-url', outfile: 'codec/string-url.wat', internal: [],
    files: [
      'codec/percent-url-form.wat', 'codec/string-escape.wat',
      'codec/c-string.wat', 'codec/pem-rfc7468.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "to_upper" (func $to_upper (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-media', outfile: 'codec/media.wat', internal: [],
    files: [
      'codec/sixel-decode.wat', 'codec/title-jpeg-entropy.wat',
      'codec/vorbis-sample-decode.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'codec-model', outfile: 'codec/model.wat', internal: [],
    files: [
      'codec/definition-decode.wat', 'codec/model-decode.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "math" "min" (func $min (param i32 i32) (result i32)))',
      '(import "math" "max" (func $max (param i32 i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  // ════════════════════════════════════════════════════════════════
  // APP families (44 files → 11 files)
  // ════════════════════════════════════════════════════════════════
  {
    name: 'app-oauth', outfile: 'app/oauth.wat', internal: [],
    files: [
      'app/oauth-core.wat', 'app/oauth-flow.wat',
      'app/oauth-pkce.wat', 'app/apps-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "strlen" (func $strlen (param i32) (result i32)))',
      '(import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-oci', outfile: 'app/oci.wat', internal: [],
    files: [
      'app/oci-config-core.wat', 'app/oci-elf64.wat',
      'app/oci-reference.wat', 'app/oci-runtime-state.wat',
      'app/oci-tar-header.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
      '(import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "fnv1a_lower" (func $fnv1a_lower (param i32 i32) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-codex', outfile: 'app/codex.wat', internal: [],
    files: [
      'app/codex-agent-core.wat', 'app/codex-app-protocol-core.wat',
      'app/codex-patch-core.wat', 'app/codex-shell-command-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-wallet', outfile: 'app/wallet.wat', internal: [],
    files: [
      'app/wallet-decimal.wat', 'app/wallet-exec-core.wat',
      'app/wallet-order-status.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-security', outfile: 'app/security.wat', internal: [],
    files: [
      'app/x509-certificate-core.wat', 'app/x509-name-core.wat',
      'app/acme-core.wat', 'app/ssh-authorized-key.wat',
      'app/dkim-body.wat', 'app/authority-storage-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
      '(import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
      '(import "edgerun" "is_upper" (func $is_upper (param i32) (result i32)))',
      '(import "edgerun" "is_lower" (func $is_lower (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-identity', outfile: 'app/identity.wat', internal: [],
    files: [
      'app/identity-hardware-core.wat', 'app/sdk-app-identity.wat',
      'app/sdk-standards-seed.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-net', outfile: 'app/net.wat', internal: [],
    files: [
      'app/mesh-core.wat', 'app/network-driver-core.wat',
      'app/node-surfaces-core.wat', 'app/protocol-suite-core.wat',
      'app/work-protocol-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-platform', outfile: 'app/platform.wat', internal: [],
    files: [
      'app/apk-api-scan.wat', 'app/browser-core.wat',
      'app/cdp-core.wat', 'app/codelyzer-source-scan.wat',
      'app/pocketbase-route-model.wat', 'app/url-scan.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "load8_u" (func $load8_u (param i32 i32) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
      '(import "edgerun" "is_hex" (func $is_hex (param i32) (result i32)))',
      '(import "edgerun" "is_alnum" (func $is_alnum (param i32) (result i32)))',
      '(import "edgerun" "memcpy" (func $memcpy (param i32 i32 i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-core', outfile: 'app/core.wat', internal: [],
    files: [
      'app/capability-core.wat', 'app/core-validator-core.wat',
      'app/marketplace-policy.wat', 'app/exchange-provider-status.wat',
      'app/time-rfc3339.wat', 'app/rfc2822-date.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "to_lower" (func $to_lower (param i32) (result i32)))',
      '(import "edgerun" "fnv1a_lower" (func $fnv1a_lower (param i32 i32) (result i32)))',
      '(import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-ui', outfile: 'app/ui.wat', internal: [],
    files: [
      'app/ui-core-semantics.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
      '(import "edgerun" "string_eq" (func $string_eq (param i32 i32 i32 i32) (result i32)))',
    ],
    sourceFixes: [],
  },
  {
    name: 'app-wayland', outfile: 'app/wayland.wat', internal: [],
    files: [
      'app/wayland-wire-core.wat',
    ],
    missingImports: [
      '(import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))',
      '(import "edgerun" "lo" (func $lo (param i64) (result i32)))',
      '(import "edgerun" "hi" (func $hi (param i64) (result i32)))',
    ],
    sourceFixes: [],
  },
];

// ── Helpers ──────────────────────────────────────────────────────────

/** Strip the (module ...) wrapper — handles both proper modules and fragments */
function stripModuleWrapper(content) {
  const m = content.match(/^\s*\(module\b/m);
  if (!m) return content;

  let depth = 0;
  let end = -1;
  for (let i = 0; i < content.length; i++) {
    if (content[i] === '(') depth++;
    if (content[i] === ')') {
      depth--;
      if (depth === 0) { end = i; break; }
    }
  }

  const headerEnd = content.indexOf('\n', m.index) + 1;
  if (headerEnd <= 0) return content;

  if (end === -1) {
    // Fragment module (no closing paren) — just strip the header
    return content.slice(headerEnd);
  }

  // Proper module — strip both header and closing paren
  return content.slice(headerEnd, end);
}

function stripMemory(content) {
  return content.replace(/^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)\s*/m, '');
}

/** Strip stray non-WAT content at start of file */
function stripStrayPreamble(content) {
  return content.replace(/^(?!\s*\(|\s*;;)[^\n]*\n?/, '');
}

/** Extract renames needed when internal imports use different local names */
function extractRenames(content, internalNames) {
  const renames = {};
  if (internalNames.length === 0) return renames;
  const pattern = new RegExp(
    `\\(import\\s+"(${internalNames.join('|')})"\\s+"([^"]+)"\\s+\\(func\\s+\\$(\\w+)`, 'g'
  );
  let m;
  while ((m = pattern.exec(content)) !== null) {
    if (m[3] !== m[2]) renames[`\$${m[3]}`] = `\$${m[2]}`;
  }
  return renames;
}

function applyRenames(content, renames) {
  for (const [oldN, newN] of Object.entries(renames)) {
    content = content.replace(new RegExp(oldN.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), newN);
  }
  return content;
}

/** Check if a line is an import */
function isImportLine(line) {
  return /^\s*\(import\s+"[^"]*"\s+"[^"]*"\s+\((?:func|global)\s+\$\w+/.test(line.trim());
}

/** Check if a line is a data segment */
function isDataLine(line) {
  return /^\s*\(data\s+/.test(line.trim());
}

/** Check if a line is a memory declaration */
function isMemoryLine(line) {
  return /^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)/.test(line.trim());
}

/** Check if a line is a global definition */
function isGlobalDefLine(line) {
  return /^\s*\(global\s+\$\w+/.test(line.trim());
}

function parseGlobalName(line) {
  const m = line.trim().match(/^\(global\s+(\$\w+)/);
  return m ? m[1] : null;
}

/** Check if a line is an exported function and extract the export name */
function isExportLine(line) {
  return /^\s*\(func\s+(?:\$\w+\s+)?\(export\s+"/.test(line.trim());
}

function parseExportName(line) {
  const m = line.trim().match(/^\(func\s+(?:\$\w+\s+)?\(export\s+"([^"]+)"\)/);
  return m ? m[1] : null;
}

/** Parse import info */
function parseImport(line) {
  const m = line.trim().match(/\(import\s+"([^"]+)"\s+"([^"]+)"\s+\((?:func|global)\s+(\$\w+)/);
  if (!m) return null;
  return { mod: m[1], name: m[2], localName: m[3], key: `${m[1]}:${m[2]}` };
}

// ── Main ─────────────────────────────────────────────────────────────

function merge() {
  const dryRun = process.argv.includes('--dry-run');
  let totalOriginal = 0;
  let totalMerged = 0;

  for (const family of FAMILIES) {
    console.log(`\n${'='.repeat(60)}`);
    console.log(`  ${family.name.toUpperCase()}: ${family.files.length} files → ${family.outfile}`);
    console.log(`${'='.repeat(60)}`);

    const allImportLines = [];
    const bodyParts = [];
    const dataParts = [];
    const seenGlobals = {};
    let familyOriginalLines = 0;

    for (const filePath of family.files) {
      const fullPath = resolve(ROOT, filePath);
      if (!existsSync(fullPath)) {
        console.warn(`  ⚠  MISSING: ${filePath} — skipping`);
        continue;
      }

      let content = readFileSync(fullPath, 'utf-8');
      const lines = content.split('\n').length;
      familyOriginalLines += lines;

      // Apply source-specific fixes (file-specific and wildcard '*')
      for (const fix of (family.sourceFixes || [])) {
        if (fix.file === filePath || fix.file === '*') {
          const before = content;
          content = content.replaceAll(fix.find, fix.replace);
          if (content !== before) console.log(`    fixed: ${fix.find} → ${fix.replace}`);
        }
      }

      // Strip stray preamble
      content = stripStrayPreamble(content);

      const hadModule = /^\s*\(module\b/m.test(content);
      if (hadModule) {
        content = stripModuleWrapper(content);
        content = stripMemory(content);
        console.log(`  • ${filePath}: ${lines} lines → stripped module+memory wrapper`);
      } else {
        const hadMemory = /^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)/m.test(content);
        if (hadMemory) {
          content = content.replace(/^\s*\(memory\s+\(export\s+"memory"\)\s+\d+\)\s*/m, '');
          console.log(`  • ${filePath}: ${lines} lines (fragment, stripped inline memory)`);
        } else {
          console.log(`  • ${filePath}: ${lines} lines (fragment)`);
        }
      }

      // Extract rename rules for internal imports
      const renames = extractRenames(content, family.internal);

      // Strip internal import lines
      const keepLines = content.split('\n').filter(l => {
        const trimmed = l.trim();
        if (!trimmed.startsWith('(import')) return true;
        const modMatch = trimmed.match(/^\(import\s+"([^"]+)"/);
        return modMatch ? !family.internal.includes(modMatch[1]) : true;
      });

      // Apply renames
      const cleaned = applyRenames(keepLines.join('\n'), renames);
      const cleanedLines = cleaned.split('\n');

      const internalStripped = keepLines.join('\n').length !== content.length;
      if (internalStripped) {
        console.log(`    stripped ${family.internal.join(', ')} imports`);
      }
      if (Object.keys(renames).length > 0) {
        console.log(`    renamed: ${Object.keys(renames).join(', ')}`);
      }

      // Split into imports, data, and body
      for (const line of cleanedLines) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith(';')) {
          bodyParts.push(line);
        } else if (isImportLine(trimmed)) {
          const info = parseImport(trimmed);
          if (info) allImportLines.push({ ...info, raw: trimmed });
        } else if (isMemoryLine(trimmed)) {
          // Stripped
        } else if (isDataLine(trimmed)) {
          dataParts.push(line);
        } else if (isGlobalDefLine(trimmed)) {
          const name = parseGlobalName(trimmed);
          if (family.dedupGlobals?.includes(name)) {
            if (seenGlobals[name]) {
              // deduplicated
            } else {
              seenGlobals[name] = true;
              bodyParts.push(line);
            }
          } else {
            bodyParts.push(line);
          }
        } else if (isExportLine(trimmed) && family.stripExports) {
          const expName = parseExportName(trimmed);
          if (family.stripExports.includes(expName)) {
            const withoutExport = trimmed.replace(/\(export\s+"[^"]+"\)\s*/, '');
            bodyParts.push(line.replace(trimmed, withoutExport));
          } else {
            bodyParts.push(line);
          }
        } else {
          bodyParts.push(line);
        }
      }
    }

    // ── Deduplicate imports ──
    const seen = {};
    const dedupedImports = [];
    const importRenames = {};

    for (const imp of allImportLines) {
      if (seen[imp.key]) {
        if (imp.localName !== seen[imp.key]) {
          importRenames[imp.localName] = seen[imp.key];
        }
      } else {
        seen[imp.key] = imp.localName;
        dedupedImports.push(imp);
      }
    }

    // Add missing imports
    for (const missingImport of (family.missingImports || [])) {
      const info = parseImport(missingImport);
      if (info && !seen[info.key]) {
        seen[info.key] = info.localName;
        dedupedImports.push({ ...info, raw: missingImport.replace(/^\s*/, '').replace(/\s*$/, '') });
        console.log(`    added missing import: ${info.key}`);
      }
    }

    // Apply import renames to body + data
    let bodyContent = bodyParts.join('\n');
    for (const [oldL, newL] of Object.entries(importRenames)) {
      bodyContent = bodyContent.replace(new RegExp(oldL.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'g'), newL);
    }

    const dedupCount = allImportLines.length - dedupedImports.length;
    if (dedupCount > 0) console.log(`    deduplicated ${dedupCount} imports (${Object.keys(importRenames).length} renames)`);

    // ── Deduplicate exports (strip (export "name") from non-first occurrences) ──
    const seenExports = {};
    const exportDedupLines = [];
    for (const line of bodyContent.split('\n')) {
      const trimmed = line.trim();
      if (isExportLine(trimmed)) {
        const expName = parseExportName(trimmed);
        if (seenExports[expName]) {
          // Strip this export — keep the function body
          const withoutExport = trimmed.replace(/\(export\s+"[^"]+"\)\s*/, '');
          exportDedupLines.push(line.replace(trimmed, withoutExport));
        } else {
          seenExports[expName] = true;
          exportDedupLines.push(line);
        }
      } else {
        exportDedupLines.push(line);
      }
    }
    bodyContent = exportDedupLines.join('\n');
    const exportDedupCount = Object.keys(seenExports).length;
    if (exportDedupCount > 0) {
      // Count how many exports were found total
      const totalExports = bodyContent.match(/\(export\s+"[^"]+"\)/g)?.length || 0;
      console.log(`    deduplicated ${totalExports} → ${exportDedupCount} unique exports`);
    }

    // Combine body + data (data segments at end)
    const allBody = [bodyContent, ...dataParts].join('\n');

    // ── Assemble output ──
    const importSection = dedupedImports.map(i => `  ${i.raw}`).join('\n');
    const moduleHeader = '(module\n';
    const memoryDecl = '  (memory (export "memory") 1)\n';
    const moduleFooter = ')\n';
    const output = moduleHeader + importSection + '\n' + memoryDecl + allBody + '\n' + moduleFooter;

    const mergedLines = output.split('\n').length;
    totalOriginal += familyOriginalLines;
    totalMerged += mergedLines;

    console.log(`  → ${mergedLines} lines (was ${familyOriginalLines})`);

    if (!dryRun) {
      writeFileSync(resolve(ROOT, family.outfile), output, 'utf-8');
      console.log(`  ✓ Wrote ${family.outfile}`);
    } else {
      console.log(`  [dry-run] Would write ${family.outfile}`);
    }
  }

  console.log(`\n${'='.repeat(60)}`);
  console.log(`  TOTAL: ${totalOriginal} → ${totalMerged} lines`);
  console.log(`  Files: ${FAMILIES.reduce((s, f) => s + f.files.length, 0)} → ${FAMILIES.length}`);
  console.log(`${'='.repeat(60)}`);
}

merge();
