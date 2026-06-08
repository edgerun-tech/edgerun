#!/usr/bin/env node
// Embed source files as a WASM custom section ('source').
// Reads a .wasm, appends a custom section with a JSON blob of { path -> content },
// writes to output. The JSON is gzip-compressed before embedding.
//
// Usage: node embed-source.mjs input.wasm output.wasm [source-dir ...]

import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';
import { gzipSync, gunzipSync } from 'zlib';

const SECTION_CUSTOM = 0;

function leb128Size(n) {
  if (n < 128) return 1;
  if (n < 16384) return 2;
  if (n < 2097152) return 3;
  if (n < 268435456) return 4;
  return 5;
}

function writeLEB128(buf, off, n) {
  let pos = off;
  while (n >= 128) {
    buf[pos++] = (n & 127) | 128;
    n >>>= 7;
  }
  buf[pos++] = n;
  return pos;
}

function writeSection(buf, off, id, payload) {
  let pos = off;
  buf[pos++] = id;
  const lenSize = leb128Size(payload.length);
  // shift payload right by lenSize to make room
  buf.copyWithin(pos + lenSize, pos, pos + payload.length);
  pos = writeLEB128(buf, pos, payload.length);
  buf.set(payload, pos);
  return pos + payload.length;
}

function embedSource(inputWasm, outputWasm, sourceDirs) {
  const wasm = readFileSync(inputWasm);
  if (wasm[0] !== 0x00 || wasm[1] !== 0x61 || wasm[2] !== 0x73 || wasm[3] !== 0x6D) {
    console.error('Not a valid WASM file');
    process.exit(1);
  }

  // Collect source files
  const { readdirSync, statSync } = await_import_fs();
  const sourceFiles = {};
  for (const dir of sourceDirs) {
    const abs = resolve(dir);
    collectFiles(abs, abs, sourceFiles);
  }

  const json = JSON.stringify(sourceFiles);
  const compressed = gzipSync(Buffer.from(json, 'utf8'));
  const nameBuf = Buffer.from('source', 'utf8');
  const payload = Buffer.concat([nameBuf, compressed]);
  const sectionLen = 1 + leb128Size(payload.length) + payload.length;

  const out = Buffer.alloc(wasm.length + sectionLen);
  wasm.copy(out);
  writeSection(out, wasm.length, SECTION_CUSTOM, payload);

  writeFileSync(outputWasm, out);
  const n = Object.keys(sourceFiles).length;
  console.log(`Embedded ${n} source files (${compressed.length} bytes gzip) → ${outputWasm}`);
}

function collectFiles(root, dir, map) {
  const { readdirSync, statSync } = require_fs();
  for (const name of readdirSync(dir)) {
    const full = `${dir}/${name}`;
    const st = statSync(full);
    if (st.isDirectory()) {
      collectFiles(root, full, map);
    } else if (name.endsWith('.wat') || name.endsWith('.mjs') || name.endsWith('.json') || name.endsWith('.md')) {
      const rel = full.slice(root.length + 1);
      map[rel] = readFileSync(full, 'utf8');
    }
  }
}

// Helper — dynamic import for ESM
function await_import_fs() {
  return { readdirSync, statSync };
}
function require_fs() {
  return { readdirSync, statSync };
}

// Extract embedded source from a .wasm file (used by CLI)
export function extractSource(wasmPath) {
  const wasm = readFileSync(wasmPath);
  let pos = 8; // skip magic + version
  while (pos < wasm.length) {
    const id = wasm[pos++];
    const [len, lenSize] = readLEB128(wasm, pos);
    pos += lenSize;
    if (id === SECTION_CUSTOM) {
      const nameEnd = wasm.indexOf(0, pos);
      if (nameEnd < 0) break;
      const name = wasm.toString('utf8', pos, nameEnd);
      pos = nameEnd + 1;
      const dataLen = pos + len - (nameEnd + 1);
      if (name === 'source') {
        const compressed = wasm.slice(pos, pos + dataLen);
        const json = gunzipSync(compressed).toString('utf8');
        return JSON.parse(json);
      }
    }
    pos += len;
  }
  return null;
}

function readLEB128(buf, off) {
  let val = 0, shift = 0, pos = off;
  while (pos < buf.length) {
    const b = buf[pos++];
    val |= (b & 127) << shift;
    shift += 7;
    if (!(b & 128)) return [val, pos - off];
  }
  return [val, pos - off];
}

function main() {
  const args = process.argv.slice(2);
  if (args.length < 2) {
    console.error('Usage: node embed-source.mjs <input.wasm> <output.wasm> [source-dirs...]');
    process.exit(1);
  }
  const [input, output, ...dirs] = args;
  embedSource(input, output, dirs.length ? dirs : ['.']);
}

main();
