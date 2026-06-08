#!/usr/bin/env bun
// er — EdgeRun CLI: view, edit, build, commit
//
//  er build [--output out.wasm]   Assemble + compile + embed source
//  er view [path]                  List or cat embedded source
//  er edit [path]                  Extract to tmpdir, open $EDITOR, re-embed
//  er commit                       Rebuild from previously edited source
//  er help
//
// View/edit/commit run through er-tools.wasm (self-hosting WAT module).
// Build is the only step that still depends on JS build tools.

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, rmSync, watch } from 'fs';
import { execSync, spawnSync } from 'child_process';
import { resolve, dirname, relative } from 'path';
import { tmpdir } from 'os';
import { gzipSync, gunzipSync } from 'zlib';
import { writeLEB128, readLEB128, leb128Size } from './er-codec.mjs';

const CUSTOM_SECTION_NAME = 'source';

// ── Help ──────────────────────────────────────────────────────────────
function help() {
  console.log(`
  er build [--output out.wasm]  Assemble, compile to .wasm, embed source
  er view [path]                List embedded source files or cat one
  er edit [path]                Extract all (or one) to tmpdir, open editor
  er commit                     Rebuild from previously edited source
  er help                       This message

  ER_OUT  env var  Default output path (default: out/edgerun.wasm)
  `);
}

// ── er-tools WASM module ──────────────────────────────────────────────
let _ert = null;
async function getErTools(wasmPath) {
  if (_ert && !wasmPath) return _ert;

  let toolsBin;
  if (wasmPath) {
    const buf = readFileSync(wasmPath);
    toolsBin = extractCustomSection(buf, 'edgetools');
  }
  if (!toolsBin) {
    const outDir = resolve(dirname(process.argv[1]), '..', 'out', 'compiler');
    const toolsPath = resolve(outDir, 'er-tools.wasm');
    if (!existsSync(toolsPath)) {
      const watPath = resolve(dirname(process.argv[1]), '..', 'compiler', 'er-tools.wat');
      if (existsSync(watPath)) {
        console.log('Compiling er-tools.wat → er-tools.wasm...');
        const { execSync } = await import('child_process');
        try {
          execSync(`wat2wasm "${watPath}" -o "${toolsPath}"`, { stdio: 'pipe' });
        } catch {
          console.error('er-tools.wasm not found and wat2wasm compilation failed.');
          console.error('Install wabt (https://github.com/WebAssembly/wabt) and run:');
          console.error('  wat2wasm compiler/er-tools.wat -o out/compiler/er-tools.wasm');
          process.exit(1);
        }
      } else {
        console.error('er-tools.wasm not found and er-tools.wat source is missing.');
        process.exit(1);
      }
    }
    toolsBin = readFileSync(toolsPath);
  }

  const mod = new WebAssembly.Module(toolsBin);
  const memory = new WebAssembly.Memory({ initial: 1024 });
  const inst = new WebAssembly.Instance(mod, { host: { memory } });
  _ert = { inst, mem: memory, exports: inst.exports };
  return _ert;
}

function withErTools(wasmPath, fn) {
  return getErTools(wasmPath).then(async ({ inst, mem, exports }) => {
    let wasmBuf = readFileSync(wasmPath);
    const memView = new Uint8Array(mem.buffer);

    // Decompress the source section if gzipped (er-tools expects uncompressed)
    // wasmBuf = decompressWasmSource(wasmBuf);

    // Write target .wasm into module memory at offset 0
    if (wasmBuf.length > 0x3200000) {
      console.error('WASM binary too large (>50MB) for er-tools memory');
      process.exit(1);
    }
    memView.set(wasmBuf, 0);

    // Init: parses source section at memory offset 0
    const fileCount = exports.init(0, wasmBuf.length);
    if (fileCount < 0) {
      console.error('Failed to parse source section — not a valid EdgeRun .wasm?');
      process.exit(1);
    }

    return fn(exports, mem, memView, wasmPath, fileCount);
  });
}

// ── VIEW ──────────────────────────────────────────────────────────────
async function cmdView(args) {
  let wasmPath, filePath;
  if (args.length && args[0].endsWith('.wasm')) {
    wasmPath = args[0];
    filePath = args[1];
  } else {
    wasmPath = findWasm();
    filePath = args[0];
  }

  await withErTools(wasmPath, (exports, mem, memView, _wp, fileCount) => {
    const scratch = 0x3300000;
    if (!filePath) {
      // List files — use list_files(scratch_ptr, max_len)
      const maxLen = 0x80000;  // 512KB for listing
      const len = exports.list_files(scratch, maxLen);
      const text = new TextDecoder().decode(memView.slice(scratch, scratch + len));
      console.log(text);
    } else {
      // Cat a file — write name into scratch, call cat_file
      const nameBuf = Buffer.from(filePath, 'utf8');
      memView.set(nameBuf, scratch);
      const outPtr = scratch + nameBuf.length + 4;
      const maxLen = 0x80000;
      const len = exports.cat_file(scratch, nameBuf.length, outPtr, maxLen);
      if (len < 0) {
        console.error(`File not found: ${filePath}`);
        process.exit(1);
      }
      const text = new TextDecoder().decode(memView.slice(outPtr, outPtr + len));
      console.log(text);
    }
  });
}

// ── EDIT ──────────────────────────────────────────────────────────────
async function cmdEdit(args) {
  let wasmPath, filePath;
  if (args.length && args[0].endsWith('.wasm')) {
    wasmPath = args[0];
    filePath = args[1];
  } else {
    wasmPath = findWasm();
    filePath = args[0];
  }

  await withErTools(wasmPath, async (exports, mem, memView, wp, fileCount) => {
    const editor = process.env.EDITOR || process.env.VISUAL || 'vi';
    const dir = `${tmpdir()}/er-src-${process.pid}`;
    const scratch = 0x3300000;
    const outPtr = 0x3308000;

    if (filePath) {
      // Extract single file
      const nameBuf = Buffer.from(filePath, 'utf8');
      memView.set(nameBuf, scratch);
      const len = exports.cat_file(scratch, nameBuf.length, outPtr, 0x80000);
      if (len < 0) {
        console.error(`File not found: ${filePath}`);
        process.exit(1);
      }
      const content = new TextDecoder().decode(memView.slice(outPtr, outPtr + len));

      const outPath = `${dir}/${filePath}`;
      mkdirSync(dirname(outPath), { recursive: true });
      writeFileSync(outPath, content, 'utf8');
      const r = spawnSync(editor, [outPath], { stdio: 'inherit', env: process.env });
      if (r.status !== 0) process.exit(1);

      // Read back and update via er-tools
      const newBuf = Buffer.from(readFileSync(outPath, 'utf8'), 'utf8');
      memView.set(newBuf, outPtr);
      const result = exports.edit_file(scratch, nameBuf.length, outPtr, newBuf.length);
      if (result < 0) {
        console.error('edit_file failed');
        process.exit(1);
      }
      console.log(`✓ Updated ${filePath} in memory`);
    } else {
      // Extract all files, open editor in tmpdir (no re-embed, just extract + edit)
      const files = readFileTableFromMemory(mem, fileCount);
      for (const [name, content] of Object.entries(files)) {
        const p = `${dir}/${name}`;
        mkdirSync(dirname(p), { recursive: true });
        writeFileSync(p, content, 'utf8');
      }
      console.log(`Extracted ${fileCount} files to ${dir}`);
      const r = spawnSync(editor, [], { stdio: 'inherit', env: { ...process.env, CHDIR: dir } });
      if (r.status !== 0) process.exit(1);

      // Read back edits — inline update, no save needed
      const dirFiles = readDirToFiles(dir);
      for (const [name, content] of Object.entries(dirFiles)) {
        const nameBuf = Buffer.from(name, 'utf8');
        const contentBuf = Buffer.from(content, 'utf8');
        memView.set(nameBuf, scratch);
        memView.set(contentBuf, outPtr);
        exports.edit_file(scratch, nameBuf.length, outPtr, contentBuf.length);
      }
      console.log(`✓ Updated ${Object.keys(dirFiles).length} files in memory`);
    }

    // Persist: write edited files back to source tree, recompile
    console.log('• Writing edits to source tree...');
    const updatedFiles = readFileTableFromMemory(mem, fileCount);
    const standards = resolve(dirname(process.argv[1]), '..');
    for (const [name, content] of Object.entries(updatedFiles)) {
      const p = `${standards}/${name}`;
      mkdirSync(dirname(p), { recursive: true });
      writeFileSync(p, content, 'utf8');
    }

    const root = resolve(dirname(process.argv[1]), '..', '..');
    console.log('• Running build...');
    execSync('bun run all', { stdio: 'inherit', cwd: root });

    console.log('• Rebuilding .wasm with embedded source...');
    cmdBuild(['--output', wp]);

    rmSync(dir, { recursive: true });
    console.log(`✓ Persisted to ${wp}`);
  });
}

// ── COMMIT ───────────────────────────────────────────────────────────
async function cmdCommit() {
  const wasm = findWasm();
  const files = await withErTools(wasm, (exports, mem, memView, _wp, fileCount) => {
    return readFileTableFromMemory(mem, fileCount);
  });
  if (!Object.keys(files).length) { console.error('No source in', wasm); process.exit(1); }

  const standards = resolve(dirname(process.argv[1]), '..');
  for (const [name, content] of Object.entries(files)) {
    const p = `${standards}/${name}`;
    mkdirSync(dirname(p), { recursive: true });
    writeFileSync(p, content, 'utf8');
  }

  const root = resolve(dirname(process.argv[1]), '..', '..');
  console.log('Running build...');
  execSync('bun run all', { stdio: 'inherit', cwd: root });

  cmdBuild(['--output', wasm]);
}

// ── Helpers for er-tools source extraction ──────────────────────────
function readFileTableFromMemory(mem, fileCount) {
  const view = new Uint8Array(mem.buffer);
  const decoder = new TextDecoder();
  const files = {};
  const u32 = (off) => view[off] | (view[off+1] << 8) | (view[off+2] << 16) | (view[off+3] << 24);
  for (let i = 0; i < fileCount; i++) {
    const off = 0x3200000 + i * 16;
    const namePtr = u32(off);
    const nameLen = u32(off + 4);
    const contentPtr = u32(off + 8);
    const contentLen = u32(off + 12);
    const name = decoder.decode(view.slice(namePtr, namePtr + nameLen));
    const content = decoder.decode(view.slice(contentPtr, contentPtr + contentLen));
    files[name] = content;
  }
  return files;
}

function readDirToFiles(dir) {
  const files = {};
  function walk(d) {
    const entries = readdirSync(d, { withFileTypes: true });
    for (const e of entries) {
      const full = `${d}/${e.name}`;
      if (e.isDirectory()) walk(full);
      else files[relative(dir, full)] = readFileSync(full, 'utf8');
    }
  }
  walk(dir);
  return files;
}

// ── BUILD (JS-based, not self-hosting yet) ────────────────────────────
function cmdBuild(args) {
  const outIdx = args.indexOf('--output');
  const watchFlag = args.includes('--watch');
  let output = 'out/edgerun.wasm';
  const standards = resolve(dirname(process.argv[1]), '..');
  const sourceDirs = [standards];
  if (outIdx >= 0 && outIdx + 1 < args.length) {
    output = args[outIdx + 1];
  }

  function doBuild() {
    const wat2wasm = findTool('wat2wasm');

    console.log('• Generating JIT dispatch tables...');
    const root = resolve(dirname(process.argv[1]), '..', '..');
    try { execSync('bun run gen', { stdio: 'pipe', cwd: root }); } catch {}

    const ui = `${resolve(dirname(process.argv[1]), '..')}/out/ui/ui_framework.wat`;
    if (!existsSync(ui)) {
      console.error('out/ui/ui_framework.wat not found — run build_wat.mjs first');
      process.exit(1);
    }

    const tmp = `${tmpdir()}/edgerun-${process.pid}.wat`;
    const uiText = readFileSync(ui, 'utf8');

    const lines = uiText.split('\n');
    let lastImport = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].includes('(import')) lastImport = i;
    }
    lines.splice(lastImport + 1, 0, '', '  ;; ── Canonical memory ────────────────────────────────', '  (memory (export "memory") 288)', '');
    writeFileSync(tmp, lines.join('\n'));

    console.log('• Compiling to WASM binary...');
    const wasmTmp = `${tmpdir()}/edgerun-${process.pid}.wasm`;
    const r = spawnSync(wat2wasm, [tmp, '-o', wasmTmp], { stdio: 'pipe' });
    if (r.status !== 0) {
      console.error('✗ wat2wasm failed');
      process.stdout.write(r.stderr.toString());
      rmSync(tmp);
      return false;
    }
    rmSync(tmp);

    console.log('• Collecting source files...');
    const files = collectSources(sourceDirs);
    embedSourceIntoWasm(wasmTmp, output, files);

    console.log('• Embedding er-tools.wasm...');
    const erToolsPath = resolve(dirname(process.argv[1]), '..', 'out', 'compiler', 'er-tools.wasm');
    embedSectionIntoWasm(output, output, 'edgetools', readFileSync(erToolsPath));

    rmSync(wasmTmp);
    console.log(`✓ Built ${output} (${Object.keys(files).length} source files)`);
    return true;
  }

  doBuild();

  if (watchFlag) {
    console.log(`\nWatching ${standards} for changes...`);
    const debounce = {};
    watch(standards, { recursive: true }, (event, filename) => {
      if (!filename) return;
      if (/^\./.test(filename)) return;
      if (!/\.(wat|mjs|json|js)$/i.test(filename)) return;
      const now = Date.now();
      if (debounce[filename] && now - debounce[filename] < 500) return;
      debounce[filename] = now;
      console.log(`\n↻ Change detected: ${filename}`);
      doBuild();
    });
  }
}

// ── Source collection helpers ───────────────────────────────────────
function collectSources(dirs) {
  const files = {};
  const seen = new Set();
  for (const dir of dirs) {
    const abs = resolve(dir);
    if (!existsSync(abs)) continue;
    walk(abs, abs, files, seen);
  }
  return files;
}

function walk(root, dir, map, seen) {
  let entries;
  try { entries = readdirSync(dir, { withFileTypes: true }); }
  catch { return; }
  for (const e of entries) {
    const full = `${dir}/${e.name}`;
    if (e.name.startsWith('.')) continue;
    if (e.isDirectory()) {
      if (e.name === 'node_modules' || e.name === '.git' || e.name === 'out') continue;
      walk(root, full, map, seen);
    } else if (e.name.endsWith('.wat') || e.name.endsWith('.mjs') || e.name.endsWith('.json') || e.name.endsWith('.js') || e.name.endsWith('.md')) {
      const rel = relative(root, full);
      if (!seen.has(rel)) {
        seen.add(rel);
        if (rel === 'out/ui/ui_framework.wat') continue;  // assembled from ui/src/*.wat, skip
        try { map[rel] = readFileSync(full, 'utf8'); } catch {}
      }
    }
  }
}

function embedSourceIntoWasm(wasmPath, outputPath, files) {
  const buf = readFileSync(wasmPath);

  // Build binary file table: count + entries(name_leb + name + content_leb + content)
  let payload = [];
  function writeLeb(n) {
    while (n >= 128) { payload.push((n & 127) | 128); n >>>= 7; }
    payload.push(n);
  }
  const entries = Object.entries(files);
  writeLeb(entries.length);
  for (const [name, content] of entries) {
    const nb = Buffer.from(name, 'utf8');
    const cb = Buffer.from(content, 'utf8');
    writeLeb(nb.length);
    for (let i = 0; i < nb.length; i++) payload.push(nb[i]);
    writeLeb(cb.length);
    for (let i = 0; i < cb.length; i++) payload.push(cb[i]);
  }

  const compressed = gzipSync(Buffer.from(payload));
  const nameBuf = Buffer.from(CUSTOM_SECTION_NAME + '\0', 'utf8');
  const sectionPayload = Buffer.concat([nameBuf, compressed]);
  const sectionPayloadLen = sectionPayload.length;
  const sectionLen = 1 + leb128Size(sectionPayloadLen) + sectionPayloadLen;

  const out = Buffer.alloc(buf.length + sectionLen);
  buf.copy(out);
  let pos = buf.length;
  out[pos++] = 0;  // custom section ID
  pos = writeLEB128(out, pos, sectionPayloadLen);
  out.set(sectionPayload, pos);

  writeFileSync(outputPath, out);
}

// ── Custom section extraction ────────────────────────────────────────
function extractCustomSection(buf, name) {
  let pos = 8;
  while (pos < buf.length) {
    const sectionId = buf[pos++];
    const [payloadLen, ls] = readLEB128(buf, pos);
    pos += ls;
    if (sectionId === 0) {
      let nameEnd = pos;
      while (buf[nameEnd] !== 0) nameEnd++;
      const sectionName = buf.toString('utf8', pos, nameEnd);
      const contentStart = nameEnd + 1;
      if (sectionName === name) {
        return Buffer.from(buf.slice(contentStart, contentStart + payloadLen - (contentStart - pos)));
      }
    }
    pos += payloadLen;
  }
  return null;
}

function embedSectionIntoWasm(wasmPath, outputPath, sectionName, data) {
  const buf = readFileSync(wasmPath);
  const nameBuf = Buffer.from(sectionName + '\0', 'utf8');
  const payload = Buffer.concat([nameBuf, data]);
  const sectionPayloadLen = payload.length;
  const sectionLen = 1 + leb128Size(sectionPayloadLen) + sectionPayloadLen;
  const out = Buffer.alloc(buf.length + sectionLen);
  buf.copy(out);
  let pos = buf.length;
  out[pos++] = 0;
  pos = writeLEB128(out, pos, sectionPayloadLen);
  out.set(payload, pos);
  writeFileSync(outputPath, out);
}

// ── Find tools ─────────────────────────────────────────────────────
function findTool(name) {
  const r = spawnSync('/bin/sh', ['-c', `command -v ${name}`], { stdio: 'pipe' });
  if (r.status === 0) return r.stdout.toString().trim().split('\n')[0];
  console.error(`${name} not found — install from https://github.com/WebAssembly/wabt`);
  process.exit(1);
}

function findWasm() {
  const env = process.env.ER_OUT;
  if (env && existsSync(env)) return env;
  const candidates = ['out/edgerun.wasm', 'edgerun.wasm', 'er.wasm'];
  for (const c of candidates) {
    if (existsSync(c)) return resolve(c);
    const p = resolve(dirname(process.argv[1]), '..', c);
    if (existsSync(p)) return p;
  }
  console.error('No .wasm found. Build one first: er build');
  process.exit(1);
}

// ── Main ─────────────────────────────────────────────────────────────
function main() {
  const cmd = process.argv[2] || 'help';
  const args = process.argv.slice(3);
  switch (cmd) {
    case 'build':  cmdBuild(args); break;
    case 'view':   cmdView(args).catch(e => { console.error(e); process.exit(1); }); break;
    case 'edit':   cmdEdit(args).catch(e => { console.error(e); process.exit(1); }); break;
    case 'commit': cmdCommit().catch(e => { console.error(e); process.exit(1); }); break;
    default:       help(); break;
  }
}

main();
