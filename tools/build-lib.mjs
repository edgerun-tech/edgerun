// build-lib — shared utilities for EdgeRun build tools
//
// Provides: resolveRoot, rootPath, fileExists, ensureDir, readText, writeText,
//           wrapModule, compileWat, writeLEB128, readLEB128, leb128Size

import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { resolve } from 'path';
import { spawnSync } from 'child_process';
import { writeLEB128, readLEB128, leb128Size, compileWat as codecCompile } from './er-codec.mjs';

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

// File-based compileWat: reads WAT from disk, returns success boolean
export function compileWat(watPath, wasmPath) {
  try {
    const src = readText(watPath);
    const wasm = codecCompile(src);
    writeFileSync(wasmPath, wasm);
    const size = wasm.length;
    console.log(`  ✓ ${wasmPath} (${(size / 1024).toFixed(0)} KB)`);
    return true;
  } catch (e) {
    console.error(`  ✗ compile failed: ${e.message}`);
    return false;
  }
}
