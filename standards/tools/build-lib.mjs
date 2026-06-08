// EdgeRun Build Library — shared across all build scripts
import { readFileSync, writeFileSync, existsSync, statSync, mkdirSync } from 'fs';
import { resolve } from 'path';
import { execSync } from 'child_process';

// ── Path helpers ──────────────────────────────────────────────
export function resolveRoot() {
  return resolve(import.meta.dirname, '..');
}

export function rootPath(...parts) {
  return resolve(resolveRoot(), ...parts);
}

export function readFile(path) {
  return readFileSync(path, 'utf-8');
}

export function writeFile(path, content) {
  writeFileSync(path, content, 'utf-8');
}

export function fileExists(path) {
  return existsSync(path);
}

export function ensureDir(path) {
  if (!existsSync(path)) mkdirSync(path, { recursive: true });
}

// ── Fragment concatenation ────────────────────────────────────
// Reads each file in the manifest (relative to rootDir) and returns
// the concatenated body with `;; ── path ──` section markers.
export function concatFragments(manifest, rootDir) {
  let body = '';
  let count = 0;
  for (const filePath of manifest) {
    const fullPath = resolve(rootDir, filePath);
    if (!existsSync(fullPath)) {
      console.warn(`  ⚠  ${filePath} not found — skipping`);
      continue;
    }
    const content = readFileSync(fullPath, 'utf-8');
    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
  }
  return { body, count };
}

// ── WAT compilation ───────────────────────────────────────────
// Compiles a WAT file to WASM. Tries wasm-tools parse first,
// falls back to wat2wasm. Returns true on success.
export function compileWat(watPath, wasmPath) {
  const wasmTools = resolveTool('wasm-tools');
  if (wasmTools) {
    try {
      execSync(`"${wasmTools}" parse "${watPath}" -o "${wasmPath}"`, { stdio: 'pipe' });
      console.log(`  ✓ ${wasmPath} (${(statSync(wasmPath).size / 1024).toFixed(0)} KB)`);
      return true;
    } catch (e) {
      console.error(`  ✗ wasm-tools parse failed: ${e.stderr?.slice(0, 500) || e.message}`);
      return false;
    }
  }

  // Fallback to wat2wasm
  try {
    execSync(`wat2wasm "${watPath}" -o "${wasmPath}"`, { stdio: 'pipe' });
    console.log(`  ✓ ${wasmPath} (${(statSync(wasmPath).size / 1024).toFixed(0)} KB)`);
    return true;
  } catch (e) {
    console.error(`  ✗ wat2wasm failed: ${e.stderr?.toString().slice(0, 500) || e.message}`);
    return false;
  }
}

// ── WASM stripping / optimization ─────────────────────────────
export function stripWasm(wasmPath) {
  const wasmTools = resolveTool('wasm-tools');
  if (!wasmTools) return null;
  const strippedPath = wasmPath.replace(/\.wasm$/, '-stripped.wasm');
  try {
    execSync(`"${wasmTools}" strip --all "${wasmPath}" -o "${strippedPath}"`, { stdio: 'pipe' });
    const wSize = statSync(wasmPath).size;
    const sSize = statSync(strippedPath).size;
    const saved = ((wSize - sSize) / wSize * 100).toFixed(0);
    console.log(`  ✓ stripped → ${strippedPath} (${(sSize / 1024).toFixed(0)} KB, -${saved}%)`);
    return strippedPath;
  } catch {
    console.warn('  ⚠  strip skipped');
    return null;
  }
}

export function optimizeWasm(wasmPath) {
  const wasmOpt = resolveTool('wasm-opt');
  if (!wasmOpt) return null;
  const optPath = wasmPath.replace(/\.wasm$/, '-opt.wasm');
  try {
    execSync(`"${wasmOpt}" -Oz "${wasmPath}" -o "${optPath}"`, { stdio: 'pipe' });
    const wSize = statSync(wasmPath).size;
    const oSize = statSync(optPath).size;
    const saved = ((wSize - oSize) / wSize * 100).toFixed(0);
    console.log(`  ✓ wasm-opt -Oz → ${optPath} (${(oSize / 1024).toFixed(0)} KB, -${saved}%)`);
    return optPath;
  } catch {
    console.warn('  ⚠  wasm-opt skipped');
    return null;
  }
}

// ── LEB128 helpers ────────────────────────────────────────────
export function leb128Size(n) {
  let s = 1;
  while (n >= 128) { s++; n >>>= 7; }
  return s;
}

export function writeLEB128(buf, off, n) {
  let pos = off;
  while (n >= 128) { buf[pos++] = (n & 127) | 128; n >>>= 7; }
  buf[pos++] = n;
  return pos;
}

// ── Source payload helpers ────────────────────────────────────
// Build a binary payload with LEB128-prefixed entries:
//   [entry_count] ([name_len][name_bytes][content_len][content_bytes])*
// where entry_count, name_len, content_len are LEB128-encoded.
export function buildSourcePayload(files) {
  const entries = Object.entries(files);
  const payload = [];
  function writeLeb(n) {
    while (n >= 128) { payload.push((n & 127) | 128); n >>>= 7; }
    payload.push(n);
  }
  writeLeb(entries.length);
  for (const [name, content] of entries) {
    const nb = Buffer.from(name, 'utf8');
    const cb = Buffer.from(content, 'utf8');
    writeLeb(nb.length);
    for (let i = 0; i < nb.length; i++) payload.push(nb[i]);
    writeLeb(cb.length);
    for (let i = 0; i < cb.length; i++) payload.push(cb[i]);
  }
  return Buffer.from(payload);
}

// ── WAT string escaping ───────────────────────────────────────
// Escape binary bytes for embedding in a WAT data section string.
export function bytesToWatString(bytes) {
  let s = '';
  for (let i = 0; i < bytes.length; i++) {
    const b = bytes[i];
    if (b >= 0x20 && b <= 0x7e && b !== 0x22 && b !== 0x5c) {
      s += String.fromCharCode(b);
    } else {
      s += `\\${b.toString(16).padStart(2, '0')}`;
    }
  }
  return s;
}

// ── Tool resolution ───────────────────────────────────────────
export function resolveTool(name) {
  try {
    const out = execSync(`which ${name} 2>/dev/null`, { encoding: 'utf-8' }).trim();
    return out || null;
  } catch { return null; }
}

// ── Module wrapper ────────────────────────────────────────────
// Generates a full WAT module string from body content + options.
// Replaces the old module-header.wat / module-footer.wat files.
export function wrapModule(body, options = {}) {
  const {
    variant = 'runtime',
    memory,
  } = options;

  let result = '(module';

  if (variant === 'cli') {
    result += `
  ;; ── System imports (compiled to inline syscalls by wasm2elf) ──
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  (import "wasi_snapshot_preview1" "args_sizes_get" (func $args_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "args_get" (func $args_get (param i32 i32) (result i32)))
  (memory (export "memory") 1)`;
  } else {
    result += `
  ;; (No imports — network/UI modules excluded from this build)`;
  }

  if (memory !== undefined && variant !== 'cli') {
    result += `
  (memory (export "memory") ${memory})`;
  }

  result += '\n' + body + '\n)';
  return result;
}
