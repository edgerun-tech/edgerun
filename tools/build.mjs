#!/usr/bin/env bun
// EdgeRun Build System — consolidated build tool

import { readFileSync, writeFileSync, existsSync, mkdirSync, readdirSync, statSync, unlinkSync } from 'fs';
import { resolve } from 'path';
import { writeLEB128, readLEB128, leb128Size, compileWat as codecCompile } from './er-codec.mjs';
import { resolveRoot, rootPath, fileExists, ensureDir, readText, writeText, embedCustomSection } from './build-lib.mjs';

// ── Memory range validation & template resolution ──

function parseAddress(v) {
  if (typeof v === 'number') return v;
  if (typeof v === 'string') {
    if (v.startsWith('0x') || v.startsWith('0X')) return parseInt(v, 16);
    return parseInt(v, 10);
  }
  return 0;
}

function loadAddressTable(pkg) {
  const ranges = pkg?.edgerun?.memory_ranges || {};
  const table = {};
  for (const [name, r] of Object.entries(ranges)) {
    table[name] = parseAddress(r.start);
  }
  return table;
}

function validateMemoryRanges(ranges) {
  const list = [];
  for (const [name, r] of Object.entries(ranges)) {
    const start = parseAddress(r.start);
    const end = start + r.size;
    list.push({ name, start, end });
  }
  list.sort((a, b) => a.start - b.start);
  for (let i = 1; i < list.length; i++) {
    if (list[i - 1].end > list[i].start) {
      console.error(`  ✗ MEMORY OVERLAP: "${list[i-1].name}" ends at ${list[i-1].end}, "${list[i].name}" starts at ${list[i].start}`);
      process.exit(1);
    }
  }
  console.log(`  ✓ memory ranges: ${list.length} entries, no overlaps`);
}

function resolveTemplates(body, table) {
  let result = body;
  for (const [name, value] of Object.entries(table)) {
    const pattern = `{{${name}}}`;
    const replacement = `0x${value.toString(16)}`;
    result = result.split(pattern).join(replacement);
  }
  return result;
}

// ── Fragment concatenation ──

function stripModuleWrapper(content) {
  const s = content.trimStart();
  if (!s.startsWith('(module')) return content;
  let depth = 0;
  let closePos = -1;
  for (let i = 0; i < s.length; i++) {
    if (s[i] === '(') depth++;
    else if (s[i] === ')') { depth--; if (depth === 0) { closePos = i; break; } }
  }
  if (closePos === -1) return content;
  const inner = s.slice(7, closePos).trim();
  if (inner.startsWith('(module')) return stripModuleWrapper(inner);
  return inner;
}

function extractImports(content) {
  const lines = content.split('\n');
  const result = [];
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trimStart();
    if (!trimmed.startsWith('(import ')) continue;

    let depth = 0;
    let started = false;
    let block = '';
    for (let j = i; j < lines.length; j++) {
      const part = lines[j];
      for (let k = 0; k < part.length; k++) {
        const ch = part[k];
        if (ch === '(') { depth++; started = true; }
        else if (ch === ')') depth--;
        if (started) block += ch;
        if (started && depth === 0) {
          i = j;
          result.push(block);
          break;
        }
      }
      if (depth === 0 && started) break;
      if (started) block += '\n';
    }
  }
  return result.join('\n');
}

function removeImports(content) {
  const lines = content.split('\n');
  const result = [];
  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trimStart();
    if (!trimmed.startsWith('(import ')) { result.push(lines[i]); continue; }

    let depth = 0;
    for (let j = i; j < lines.length; j++) {
      for (const ch of lines[j]) {
        if (ch === '(') depth++;
        else if (ch === ')') depth--;
      }
      if (depth === 0) { i = j; break; }
    }
  }
  return result.join('\n');
}

function concatFragments(manifest, rootDir) {
  let body = '';
  let allImports = '';
  let count = 0;
  for (const filePath of manifest) {
    const fullPath = [rootDir, filePath].join('/');
    if (!fileExists(fullPath)) {
      console.warn(`  ⚠  ${filePath} not found — skipping`);
      continue;
    }
    let content = readText(fullPath);
    content = stripModuleWrapper(content);
    const fragImports = extractImports(content);
    if (fragImports) {
      allImports += `;; ── imports from ${filePath} ──\n${fragImports}\n\n`;
    }
    content = removeImports(content);
    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
  }
  return { body, imports: allImports, count };
}

// ── WAT compilation ──
// Uses edgerun.wasm's built-in load_wat + emit_wasm via er-codec.mjs:compileWat.
// Falls back to wasm-tools parse only for bootstrapping (first build when
// edgerun.wasm doesn't exist yet, or when the internal parser can't handle
// a construct it emits but can't parse). TODO: remove after bootstrap stable.

function compileWat(watPath, wasmPath) {
  const src = readText(watPath);
  const label = `${(src.length / 1024).toFixed(0)} KB`;
  try {
    const wasm = codecCompile(src);
    writeFileSync(wasmPath, wasm);
    console.log(`  ✓ ${wasmPath} (${(label)})`);
    return true;
  } catch (e) {
    console.log(`  ↻ internal compile failed (${label}): ${e.message?.slice(0, 120)}`);
  }
  console.log(`  ↻ trying wasm-tools parse (${label})...`);
  try {
    const r = Bun.spawnSync(['wasm-tools', 'parse', watPath, '-o', wasmPath]);
    if (r.exitCode === 0) {
      const size = readFileSync(wasmPath).length;
      console.log(`  ✓ ${wasmPath} (${(size / 1024).toFixed(0)} KB)`);
      return true;
    }
    console.error(`  ✗ wasm-tools parse failed: ${r.stderr.toString().slice(0, 500)}`);
  } catch {}
  return false;
}

// ── Module wrapper ──

function wrapModule(body, options = {}) {
  const { variant = 'runtime', memory } = options;
  let result = '(module';
  // WASI imports for CLI builds
  result += `
  ;; ── System imports ──
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_read" (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (param i32)))
  (import "wasi_snapshot_preview1" "args_sizes_get" (func $args_sizes_get (param i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "args_get" (func $args_get (param i32 i32) (result i32)))`;
  if (variant === 'cli') {
    result += `
  (memory (export "memory") 1)`;
  }
  if (memory !== undefined && variant !== 'cli') {
    result += `\n  (memory (export "memory") ${memory})`;
  }
  result += '\n' + body + '\n)';
  return result;
}

// ── Fragment discovery ──

// These files MUST appear first, in order
const PREFIX = [
  'out/gen/config.wat',
  'runtime/edgerun-core.wat',
];

function discoverFragments() {
  const ROOT = resolveRoot();

  const EXCLUDE_DIRS = ['node_modules', '.git', '.opencode', 'out', 'tests', 'tools'];

  const EXCLUDE_FILES = new Set([
    'cli/examples/cat.wat',           // standalone examples, not production code
    'cli/examples/hello-app.wat',
    'cli/examples/hello.wat',
    'pipeline/stage-registry.wat',    // references non-generated stage functions excluded by -stage regex
  ]);

  function isExcluded(relPath) {
    if (relPath === '' || relPath.startsWith('.')) return true;
    for (const d of EXCLUDE_DIRS) {
      if (relPath === d || relPath.startsWith(d + '/')) return true;
    }
    if (EXCLUDE_FILES.has(relPath)) return true;
    // Individual stage files are consolidated into generated pipeline-stages.wat
    if (/^pipeline\/.+?-stage\.wat$/.test(relPath) &&
        !['pipeline/queue-stage.wat', 'pipeline/buffer-stage.wat', 'pipeline/cdc-stage.wat'].includes(relPath)) return true;
    return false;
  }

  const files = [];

  function walk(subdir) {
    let entries;
    try { entries = readdirSync(ROOT + '/' + subdir); } catch { return; }
    for (const entry of entries) {
      const relPath = subdir ? subdir + '/' + entry : entry;
      if (isExcluded(relPath)) continue;
      const fullPath = ROOT + '/' + relPath;
      try {
        if (statSync(fullPath).isDirectory()) { walk(relPath); continue; }
      } catch { continue; }
      if (entry.endsWith('.wat')) files.push(relPath);
    }
  }

  walk('');

  const prefixSet = new Set(PREFIX);
  const result = [];

  for (const f of PREFIX) {
    if (fileExists(rootPath(f))) result.push(f);
  }

  files.sort();
  for (const p of files) {
    if (!prefixSet.has(p)) result.push(p);
  }

  return result;
}

// ══════════════════════════════════════════════════════════════════════════
// Commands
// ══════════════════════════════════════════════════════════════════════════

// ── gen-config ──

function cmdGenConfig() {
  const pkg = JSON.parse(readText(rootPath('package.json')));
  const cfg = pkg.edgerun;
  const OUT = rootPath('out/gen/config.wat');
  ensureDir(rootPath('out/gen'));

  // Validate memory ranges
  if (cfg?.memory_ranges) {
    validateMemoryRanges(cfg.memory_ranges);
  }

  const addrs = loadAddressTable(pkg);

  const lines = [';; Auto-generated by build.ts gen-config — do not edit'];
  lines.push(';; Source: package.json edgerun.{memory,data,config,globals}');
  lines.push('');

  // ── Memory declaration ──
  const memPages = cfg?.memory?.pages ?? 1024;
  lines.push(`(memory (export "memory") ${memPages})`);

  // ── Data blocks (LUTs) ──
  if (cfg?.data) {
    for (const block of cfg.data) {
      let offset = block.offset;
      const hex = block.hex;
      if (offset && hex) {
        // Resolve {{NAME}} template references if any
        if (typeof offset === 'string' && offset.startsWith('{{') && offset.endsWith('}}')) {
          const name = offset.slice(2, -2);
          if (addrs[name] !== undefined) offset = `0x${addrs[name].toString(16)}`;
        }
        lines.push(`(data (i32.const ${offset}) "${hex}")`);
      }
    }
  }
  lines.push('');

  // ── Config constants (grouped) ──
  if (cfg?.config) {
    for (const [group, entries] of Object.entries(cfg.config)) {
      lines.push(`;; ── ${group} ──`);
      for (const [name, value] of Object.entries(entries)) {
        lines.push(`(global $${name} (export "${name}") i32 (i32.const ${value}))`);
      }
      lines.push('');
    }
  }

  // ── All other globals (flat map) ──
  if (cfg?.globals) {
    const gnames = Object.keys(cfg.globals).sort();
    for (const name of gnames) {
      const g = cfg.globals[name];
      const exp = g.export ? ` (export "${name}")` : '';
      const typeStr = g.mut ? `(mut ${g.type})` : g.type;
      lines.push(`(global $${name}${exp} ${typeStr} (${g.type}.const ${g.value}))`);
    }
    lines.push('');
  }

  writeText(OUT, lines.join('\n') + '\n');
  let gcount = 0;
  if (cfg?.config) gcount += Object.values(cfg.config).reduce((n, g) => n + Object.keys(g).length, 0);
  if (cfg?.globals) gcount += Object.keys(cfg.globals).length;
  console.log(`✓ ${OUT} (${memPages} pages, ${gcount} globals)`);
}



// ── gen-stages ──

function genCfgReads(cfg_reads) {
  if (!cfg_reads || !cfg_reads.length) return { locals: '', setup: '' };
  const locs = cfg_reads.map(c =>
    `    (local \$${c.name} ${c.type || 'i32'})`
  ).join('\n');
  const setup = cfg_reads.map(c => {
    let s = '';
    if (c.default !== undefined) {
      s += `    (local.set \$${c.name} (${c.type || 'i32'}.const ${c.default}))\n`;
    }
    s += `    (if (i32.ge_u (local.get $clen) (i32.const ${c.offset + 4 || 4})) (then (local.set \$${c.name} (${c.type || 'i32'}.load${c.offset ? ` offset=${c.offset}` : ''} (local.get $cfg)))))`;
    return s;
  }).join('\n');
  return { locals: locs, setup };
}

function wrapStage(slot, name, locals, read_check, body, out_write, return_expr) {
  const hasRead = locals.includes('(local $read ');
  return `;; Process ${name} Stage — slot ${slot}
  (func $process_${name} (export "process_${name}")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    ${locals}${hasRead ? '' : '\n    (local $read i32)'}
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    ${read_check}
    ${body}
    ${out_write}
    ${return_expr})`;
}

function makeStage(st) {
  const patterns = {
    scan(st) {
      const { slot, name, fn, out_size, min_read, args, out_buf, checks, cfg_reads, extra_locals, prep, fn_ret_i64 } = st;
      const mr = min_read || 1;
      const read_check = checks === 'eqz' || mr === 1
        ? '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))'
        : `(if (i32.lt_u (local.get $read) (i32.const ${mr})) (then (return (i32.const 0))))`;
      const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
      const ob = out_buf || 'local.get $scratch';
      const cfg = genCfgReads(cfg_reads);
      const xtra = extra_locals ? '\n    ' + extra_locals.join('\n    ') : '';
      const prep_lines = prep ? prep.map(l => `    ${l}`).join('\n') + '\n' : '';
      const locals = fn_ret_i64
        ? `(local $result i64)${cfg.locals}${xtra}`
        : `(local $status i32)${cfg.locals}${xtra}`;
      const callAndCheck = fn_ret_i64
        ? `    (local.set $result (call $${fn} (${a})))
    (if (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))) (then (return (i32.sub (i32.const 0) (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32)))))))`
        : `    (local.set $status (call $${fn} (${a})))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))`;
      return wrapStage(slot, name,
        locals,
        read_check,
        `${cfg.setup}${prep_lines}${callAndCheck}`,
        `(drop (call $pipe_write (local.get $output) (${ob}) (i32.const ${out_size})))`,
        `i32.const ${out_size}`);
    },
    transform(st) {
      const { slot, name, fn, args } = st;
      const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
      const re = '(return (call $stage_write_output (local.get $output) (local.get $scratch)\n      (call $' + fn + ' (' + a + '))))';
      return wrapStage(slot, name, '',
        '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
        '', '', re);
    },
    decode(st) {
      const { slot, name, fn, out_buf, args, prep, cfg_reads, extra_locals } = st;
      const ob = out_buf || 'local.get $scratch';
      const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch', '(i32.sub (local.get $scap) (i32.const 8))']).join(') (');
      const prep_lines = prep ? prep.map(l => `    ${l}`).join('\n') + '\n' : '';
      const cfg = genCfgReads(cfg_reads);
      const xtra = extra_locals ? '\n    ' + extra_locals.join('\n    ') : '';
      return wrapStage(slot, name,
        `(local $read i32) (local $result i64) (local $status i32) (local $written i32)${cfg.locals}${xtra}`,
        '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
        `${cfg.setup}${prep_lines}    (local.set $result (call $${fn} (${a})))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $written (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))`,
        `(drop (call $pipe_write (local.get $output) (${ob}) (local.get $written)))`,
        `local.get $written`);
    },
    store(st) {
      const { slot, name, fn, out_buf, out_size, store_type, args } = st;
      const ob = out_buf || 'global.get $SHA256_OUT_BUF';
      const stype = store_type || 'i32.store';
      const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read']).join(') (');
      return wrapStage(slot, name,
        '(local $val i32)',
        '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
        `(local.set $val (call $${fn} (${a})))
    (${stype} (${ob}) (local.get $val))`,
        `(drop (call $pipe_write (local.get $output) (${ob}) (i32.const ${out_size || 4})))`,
        `i32.const ${out_size || 4}`);
    },
    map(st) {
      const { slot, name, fn, args, out_buf } = st;
      const ob = out_buf || 'local.get $scratch';
      const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
      return wrapStage(slot, name,
        '(local $out_len i32)',
        '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
        `(local.set $out_len (call $${fn} (${a})))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))`,
        `(drop (call $pipe_write (local.get $output) (${ob}) (local.get $out_len)))`,
        `local.get $out_len`);
    },
    raw(st) { return st.body; },
    custom(st) { return st.body; },
  };
  const pattern = st.pattern;
  if (!patterns[pattern]) { console.error(`Unknown pattern "${pattern}" for ${st.name}, skipping`); return ''; }
  return patterns[pattern](st);
}

function cmdGenStages(args) {
  const pkg = JSON.parse(readText(rootPath('package.json')));
  const REGISTRY = pkg.edgerun.registry;

  let header = ';; ── Auto-generated pipeline stages ──\n';
  header += ';; Generated by build.mjs gen-stages from edgerun.registry in package.json\n';
  header += ';; DO NOT EDIT — regenerate with: bun build.mjs gen-stages\n\n';

  const bodies = [];
  for (const st of REGISTRY) {
    if (st.generated === false) continue;
    const body = makeStage(st);
    if (body) bodies.push(body);
  }

  const outDir = rootPath('out/gen');
  // Pipeline stages now written to out/gen/ so they're included as part of the explicit build
  const combinedPath = [outDir, 'pipeline-stages.wat'].join('/');
  writeText(combinedPath, header + bodies.join('\n\n') + '\n');
  console.log(`Generated ${bodies.length} stages into out/gen/pipeline-stages.wat`);

  if (args.includes('--split')) {
    const splitDir = rootPath('pipeline');
    for (const st of REGISTRY) {
      if (st.generated === false) continue;
      const body = makeStage(st);
      if (!body) continue;
      const fname = st.name.replace(/_/g, '-');
      writeText([splitDir, `${fname}-stage.wat`].join('/'), '\n' + body + '\n');
    }
    const c = REGISTRY.filter(s => s.generated !== false).length;
    console.log(`Generated ${c} individual stage files (legacy mode)`);
  }
}

// ── build ──

function embedSource(wasmPath, fragmentPaths, rootDir) {
  const files = [];
  for (const path of fragmentPaths) {
    if (path.startsWith('out/')) continue;
    const fullPath = rootPath(path);
    if (!fileExists(fullPath)) continue;
    files.push({ name: path, content: readText(fullPath) });
  }

  let totalSize = leb128Size(files.length);
  for (const { name, content } of files) {
    const nb = Buffer.byteLength(name, 'utf-8');
    const cb = Buffer.byteLength(content, 'utf-8');
    totalSize += leb128Size(nb) + nb + leb128Size(cb) + cb;
  }

  const buf = Buffer.alloc(totalSize);
  let pos = 0;
  pos = writeLEB128(buf, pos, files.length);
  for (const { name, content } of files) {
    const nb = Buffer.from(name, 'utf-8');
    const cb = Buffer.from(content, 'utf-8');
    pos = writeLEB128(buf, pos, nb.length);
    nb.copy(buf, pos); pos += nb.length;
    pos = writeLEB128(buf, pos, cb.length);
    cb.copy(buf, pos); pos += cb.length;
  }

  const { size, compressed, added } = embedCustomSection(wasmPath, 'source', buf);
  console.log(`  ✓ embedded source (${files.length} files, ${(size / 1024).toFixed(0)} KB, +${added} bytes)`);
}

function cmdBuild(args) {
  const ROOT = resolveRoot();

  cmdGenConfig();
  cmdGenStages([]);

  const pkg = JSON.parse(readText(rootPath('package.json')));
  ensureDir(rootPath('out/gen'));

  const fragments = discoverFragments();
  const PREFIX_COUNT = PREFIX.length;
  const prefix = fragments.slice(0, PREFIX_COUNT);

  const outArg = args.find(a => a.startsWith('--out='));
  const outPath = outArg ? resolve(ROOT, outArg.slice(6)) : rootPath('out/edgerun.wat');
  const skipWasm = args.includes('--no-wasm');

  console.log(`\nEdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);

  const pipelineStagesPath = ['out/gen/pipeline-stages.wat'];
  const allPaths = [...prefix, ...pipelineStagesPath, ...fragments.slice(PREFIX_COUNT)];
  const { body, imports, count } = concatFragments(allPaths, ROOT);

  // Resolve all {{NAME}} template references from memory_ranges
  const addrs = loadAddressTable(pkg);
  let resolvedBody = resolveTemplates(imports + '\n' + body, addrs);

  const moduleWat = wrapModule(resolvedBody);
  writeText(outPath, moduleWat);
  console.log(`✓ ${count} fragments → ${outPath} (${moduleWat.length} bytes)`);

  if (!skipWasm) {
    const wasmPath = outPath.replace(/\.wat$/, '.wasm');
    console.log(`Compiling → ${wasmPath}...`);
    if (compileWat(outPath, wasmPath)) {
      embedSource(wasmPath, allPaths, ROOT);
    }
  }
}

// ══════════════════════════════════════════════════════════════════════════
// Main
// ══════════════════════════════════════════════════════════════════════════

function main() {
  const cmd = Bun.argv[2] || 'help';
  const args = Bun.argv.slice(3);

  switch (cmd) {
    case 'gen-config':
      cmdGenConfig();
      break;
    case 'gen-stages':
      cmdGenStages(args);
      break;
    case 'build':
    case 'build:full':
    case 'build-full':
      cmdBuild(args);
      break;
    case 'help':
    default:
      console.log(`
EdgeRun Build System — consolidated build tool

Usage: bun tools/build.mjs <command> [options]

Commands:
  gen-config              Generate config.wat (globals + memory + data)
  gen-stages [--split]    Generate pipeline stages (out/gen/pipeline-stages.wat)
  build [opts]            Build edgerun.wasm from fragments
  help                    Show this help
`);
  }
}

main();
