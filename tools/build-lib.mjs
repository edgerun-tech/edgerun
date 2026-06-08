// build-lib — shared utilities for EdgeRun build tools
//
// Provides: resolveRoot, rootPath, fileExists, ensureDir, readText, writeText,
//           wrapModule, embedCustomSection, writeLEB128, readLEB128, leb128Size

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { resolve } from 'path';
import { gzipSync } from 'zlib';
import { writeLEB128, readLEB128, leb128Size } from './er-codec.mjs';

export { writeLEB128, readLEB128, leb128Size };

export function resolveRoot() {
  return resolve(import.meta.dirname, '..');
}

export function rootPath(...parts) {
  return resolve(resolveRoot(), ...parts);
}

export function fileExists(path) {
  return existsSync(path);
}

export function ensureDir(path) {
  if (!existsSync(path)) mkdirSync(path, { recursive: true });
}

export function readText(path) {
  return readFileSync(path, 'utf-8');
}

export function writeText(path, content) {
  writeFileSync(path, content, 'utf-8');
}

export function wrapModule(body, options = {}) {
  const { variant = 'runtime', memory } = options;
  let result = '(module';
  if (variant === 'cli') {
    result += `
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  (import "wasi_snapshot_preview1" "args_sizes_get" (func $args_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "args_get" (func $args_get (param i32 i32) (result i32)))
  (memory (export "memory") 1)`;
  } else {
    result += '\n  ;; (No imports — all modules included in this build)';
  }
  if (memory !== undefined && variant !== 'cli') {
    result += `\n  (memory (export "memory") ${memory})`;
  }
  result += '\n' + body + '\n)';
  return result;
}

// Embed a custom section into a .wasm file.
// If opts.gzip is true, the data is gzip-compressed before embedding.
// The custom section name is always stored as-is (not compressed).
//
// WASM custom section format:
//   section_id (0x00, 1 byte)
//   section_size (LEB128, size of name_len + name + data)
//   name_len (LEB128)
//   name (UTF-8 bytes)
//   data (arbitrary bytes)
export function embedCustomSection(wasmPath, sectionName, data, opts = {}) {
  const buf = readFileSync(wasmPath);
  let raw = Buffer.isBuffer(data) ? data : Buffer.from(String(data), 'utf-8');
  if (opts.gzip) raw = gzipSync(raw);
  const nameBytes = Buffer.from(sectionName, 'utf-8');
  const nameLenSize = leb128Size(nameBytes.length);
  // section content = name_len(LEB128) + name_bytes + raw_data
  const contentSize = nameLenSize + nameBytes.length + raw.length;
  const sectionSize = 1 + leb128Size(contentSize) + contentSize;

  const out = Buffer.alloc(buf.length + sectionSize);
  buf.copy(out);
  let pos = buf.length;
  out[pos++] = 0; // custom section ID
  pos = writeLEB128(out, pos, contentSize); // section size (rest of section)
  pos = writeLEB128(out, pos, nameBytes.length); // name length
  out.set(nameBytes, pos); pos += nameBytes.length;
  out.set(raw, pos); pos += raw.length;

  writeFileSync(wasmPath, out);
  return { size: data.length, compressed: raw.length, added: out.length - buf.length };
}

