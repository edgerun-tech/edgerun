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
// Small tool WATs use er-codec.mjs:compileWat (load_wat/emit_wasm from
// the project-root edgerun.wasm). The full runtime WAT is too large
// for the built-in parser, so it uses wasm-tools parse.

function compileWat(watPath, wasmPath) {
  const src = readText(watPath);
  // Use self-hosted compile for small WATs (<500KB)
  if (src.length < 500000) {
    try {
      const wasm = codecCompile(src);
      writeFileSync(wasmPath, wasm);
      console.log(`  ✓ ${wasmPath} (${(wasm.length / 1024).toFixed(0)} KB)`);
      return true;
    } catch {}
  }
  // Full runtime: wasm-tools parse
  console.log(`  ↻ large WAT (${(src.length / 1024).toFixed(0)} KB) — using wasm-tools parse...`);
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
  // WASI imports needed by CLI code — compiled to inline syscalls by wasm2elf
  result += `
  ;; ── System imports (compiled to inline syscalls by wasm2elf) ──
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
    'compiler/compiler.wat',          // template — not a valid fragment
    'compiler/er-tools.wat',          // standalone module (memory import) — use er-tools-fragment.wat instead
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

// ── gen-compiler ──

function loadTemplates(pkg) {
  const templates = pkg.edgerun.templates;
  const archs = {};
  for (const [key, tmpl] of Object.entries(templates)) {
    if (key === 'base') continue;
    const base = templates.base || {};
    const merged = { ...base, ...tmpl };
    for (const dk of ['ops', 'fc_ops', 'fd_ops']) {
      merged[dk] = { ...(base[dk] || {}), ...(tmpl[dk] || {}) };
    }
    archs[key] = merged;
  }
  return archs;
}

function buildOpTable(ops, prefix, suffix) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  return entries.map(([hexcode, opname]) => {
    const funcname = `${prefix}${opname}${suffix}`;
    const code = parseInt(hexcode, 16);
    if (code === 0x00)
      return `          (if (i32.eqz (local.get $opcode))\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`;
    return `          (if (i32.eq (local.get $opcode) (i32.const ${hexcode}))\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`;
  }).join('\n');
}

function buildSubTable(ops, prefix, suffix, label) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  return entries.map(([hexcode, opname]) => {
    const funcname = `${prefix}${opname}${suffix}`;
    return `                (if (i32.eq (local.get $imm0) (i32.const ${hexcode}))\n                  (then (call $${funcname} (local.get $dec_ptr)) (br $${label})))`;
  }).join('\n');
}

function generateJIT(archName, pkg, compilerPath) {
  const archs = loadTemplates(pkg);
  const tmpl = archs[archName];
  if (!tmpl) { console.error(`Unknown arch: ${archName}`); process.exit(1); }
  let wat = readText(compilerPath);
  const arch = tmpl.arch;
  const opPrefix = tmpl.op_prefix;
  const opSuffix = tmpl.op_suffix;
  const simdPrefix = tmpl.simd_prefix !== undefined ? tmpl.simd_prefix : opPrefix;
  const simdSuffix = tmpl.simd_suffix !== undefined ? tmpl.simd_suffix : opSuffix;
  const opTable = buildOpTable(tmpl.ops, opPrefix, opSuffix);
  const fcTable = buildSubTable(tmpl.fc_ops || {}, opPrefix, opSuffix, 'fc_done');
  const fdTable = buildSubTable(tmpl.fd_ops || {}, simdPrefix, simdSuffix, 'fd_done');
  const subs = {
    '{SUFFIX}': `_${arch}`,
    '{ARCH}': arch,
    '{OP_TABLE}': opTable,
    '{FC_TABLE}': fcTable,
    '{FD_TABLE}': fdTable,
    '{PROLOGUE}': tmpl.prologue,
    '{EPILOGUE}': tmpl.epilogue,
    '{EMIT_BYTE}': tmpl.emit_byte,
    '{EMIT_DWORD}': tmpl.emit_dword,
    '{EMIT_ELF_STUB}': tmpl.emit_elf_stub,
    '{EMIT_ELF64_EHDR}': tmpl.emit_elf64_ehdr,
    '{EMIT_ELF64_PHDR}': tmpl.emit_elf64_phdr,
    '{FIXUP_CALLS}': tmpl.fixup_calls,
    '{COPY_COMPILED_CODE}': tmpl.copy_compiled_code,
    '{RESULT_GLOBAL}': tmpl.result_global,
    '{NEXT_OP_GLOBAL}': tmpl.next_op_global,
    '{JIT_ERROR_GLOBAL}': tmpl.jit_error_global,
    '{CODE_PTR_GLOBAL}': tmpl.code_ptr_global,
    '{LABEL_DEPTH_GLOBAL}': tmpl.label_depth_global,
    '{FUNC_OFF_TABLE}': tmpl.func_off_table,
  };
  for (const [k, v] of Object.entries(subs)) wat = wat.split(k).join(v);
  return wat;
}

function assembleJIT(arch, pkg, dir) {
  const tmpl = loadTemplates(pkg)[arch];
  if (!tmpl) { console.error(`Unknown arch: ${arch}`); process.exit(1); }
  const dispatch = generateJIT(arch, pkg, [dir, 'compiler.wat'].join('/'));
  const genDir = [dir, '..', 'out', 'gen'].join('/');
  const fa = arch.replace(/_/g, '-');

  const config = readText([dir, '..', 'out', 'gen', 'config.wat'].join('/')).replace(/^\(module\s*\n/, '').replace(/\n\)\n?$/, '');
  const emitCore = readText([dir, 'emit-core.wat'].join('/'));
  const emit = readText([dir, `emit-${fa}.wat`].join('/'));
  let templates = readText([dir, `templates-${fa}.wat`].join('/'));
  const simd = readText([dir, `simd-${fa}.wat`].join('/'));

  if (templates.trimStart().startsWith('(module')) {
    templates = templates.replace(/^\(module\s*\n/, '');
    if (templates.endsWith(')\n')) templates = templates.slice(0, -2);
    else if (templates.endsWith(')')) templates = templates.slice(0, -1);
  }

  if (tmpl.op_suffix) {
    const bareTargets = ['i32_trunc_f32_u', 'i32_trunc_f64_u', 'i64_trunc_f32_u', 'i64_trunc_f64_u'];
    const aliases = bareTargets.map(name =>
      `  (func $template_${name} (export "template_${name}") (call $template_${name}${tmpl.op_suffix}))`
    ).join('\n');
    templates += `\n\n${aliases}\n`;
  }

  const parts = [config, dispatch, emitCore, emit, templates, simd];
  const full = `(module\n${parts.join('\n\n')})\n`;

  ensureDir(genDir);
  const outPath = [genDir, `jit-full-${fa}.wat`].join('/');
  writeText(outPath, full);
  console.log(`Assembled: ${outPath} (${full.length} bytes)`);
}

function cmdGenCompiler(args) {
  const pkg = JSON.parse(readText(rootPath('package.json')));
  const dir = rootPath('compiler');
  const compiler = [dir, 'compiler.wat'].join('/');

  if (args[0] === '--assemble') {
    const target = args[1];
    const archs = ['x86_64', 'aarch64', 'arm32'];
    if (target === 'all') {
      for (const arch of archs) assembleJIT(arch, pkg, dir);
    } else {
      if (!archs.includes(target)) { console.error(`Unknown arch: ${target}`); process.exit(1); }
      assembleJIT(target, pkg, dir);
    }
    return;
  }

  if (args.length < 1) {
    console.error('Usage: build.mjs gen-compiler [--assemble <arch>|all]');
    process.exit(1);
  }
  const arch = args[0].replace(/-/g, '_');
  process.stdout.write(generateJIT(arch, pkg, compiler));
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

// ── build (unified — all backends) ──

function stripMultiplexer(content) {
  const marker = ';; Runtime Backend Dispatch';
  const idx = content.indexOf(marker);
  if (idx === -1) return content;
  return content.slice(0, idx).trimEnd() + '\n';
}

function genTemplateStubs(dispatchPaths, fragments, root) {
  // Collect all user-defined function CALLS from dispatch files
  const called = new Map();
  const callRe = /\(call\s+\$([a-zA-Z_][a-zA-Z0-9_]*)/g;
  for (const dp of dispatchPaths) {
    const content = readText(root + '/' + dp);
    let m;
    while ((m = callRe.exec(content)) !== null) {
      const name = m[1];
      // Count how many args are passed at this call site
      const after = content.slice(m.index + m[0].length);
      let depth = 0, args = 0, i = 0;
      for (; i < after.length; i++) {
        const ch = after[i];
        if (ch === '(') depth++;
        else if (ch === ')') { if (depth === 0) break; depth--; }
        else if (ch === ' ' && depth === 0) { if (args === 0 && i > 0) args++; }
      }
      // Count args between the fn name and closing paren
      let j = 0, argCount = 0;
      let d = 0;
      while (j < i) {
        while (j < i && after[j] === ' ') j++;
        if (j >= i) break;
        if (after[j] === '(') { d++; argCount++; j++; while (j < i && d > 0) { if (after[j] === '(') d++; else if (after[j] === ')') d--; j++; } }
        else { j++; }
      }
      const existing = called.get(name);
      if (existing === undefined || argCount > existing) called.set(name, argCount);
    }
  }
  if (called.size === 0) return [];

  // Collect all function DEFINITIONS from dispatch files and fragment source files
  const defined = new Set();
  const funcRe = /\(func\s+\$([a-zA-Z_][a-zA-Z0-9_]*)/g;
  for (const dp of dispatchPaths) {
    const content = readText(root + '/' + dp);
    let m;
    while ((m = funcRe.exec(content)) !== null) defined.add(m[1]);
  }
  for (const frag of fragments) {
    const content = readText(root + '/' + frag);
    let m;
    while ((m = funcRe.exec(content)) !== null) defined.add(m[1]);
  }

  // Also collect imported functions (no stubs needed)
  const imported = new Set();
  const importRe = /\(import[^)]+\(func\s+\$([a-zA-Z_][a-zA-Z0-9_]*)\)/g;
  for (const frag of fragments) {
    const content = readText(root + '/' + frag);
    let m;
    while ((m = importRe.exec(content)) !== null) imported.add(m[1]);
  }

  // Generate stubs for called-but-not-defined-and-not-imported functions
  const missing = [];
  for (const [name, argCount] of [...called].sort((a, b) => a[0].localeCompare(b[0]))) {
    if (!defined.has(name) && !imported.has(name)) missing.push(name);
  }
  if (missing.length === 0) return [];

  const lines = [
    ';; Auto-generated stubs for missing functions',
    ';; These trap at runtime — replace with real implementations',
    ';; Generated by build.mjs cmdBuild — do not edit',
    '',
  ];
  for (const name of missing) {
    const params = called.get(name) || 0;
    const p = params > 0 ? ` (param i32${params > 1 ? ` i32`.repeat(params - 1) : ''})` : '';
    lines.push(`  (func $${name}${p} (unreachable))`);
  }
  const outPath = root + '/out/gen/template-stubs.wat';
  writeText(outPath, lines.join('\n') + '\n');
  console.log(`✓ out/gen/template-stubs.wat (${missing.length} stubs)`);
  return ['out/gen/template-stubs.wat'];
}

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
  const compilerPath = rootPath('compiler/compiler.wat');
  const genDir = rootPath('out/gen');
  ensureDir(genDir);

  const backends = ['x86_64', 'aarch64', 'arm32'];
  const dispatchPaths = [];

  for (const arch of backends) {
    const dispatch = generateJIT(arch, pkg, compilerPath);
    const fa = arch.replace('_', '-');
    const path = rootPath(`out/gen/jit-dispatch-${fa}.wat`);
    const final = arch !== 'x86_64' ? stripMultiplexer(dispatch) : dispatch;
    writeText(path, final);
    console.log(`  ✓ jit-dispatch-${fa}.wat (${final.length} bytes)`);
    dispatchPaths.push(`out/gen/jit-dispatch-${fa}.wat`);
  }

  const fragments = discoverFragments();
  const PREFIX_COUNT = PREFIX.length;
  const prefix = fragments.slice(0, PREFIX_COUNT);

  // Generate stub template functions for arm32/aarch64 (pre-existing gaps)
  const stubPaths = genTemplateStubs(dispatchPaths, fragments, ROOT);

  const outArg = args.find(a => a.startsWith('--out='));
  const outPath = outArg ? resolve(ROOT, outArg.slice(6)) : rootPath('out/edgerun.wat');
  const skipWasm = args.includes('--no-wasm');

  console.log(`\nEdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log(`All 3 JIT backends: x86-64 + ARM32 + AArch64`);

  const pipelineStagesPath = ['out/gen/pipeline-stages.wat'];
  const allPaths = [...prefix, ...dispatchPaths, ...stubPaths, ...pipelineStagesPath, ...fragments.slice(PREFIX_COUNT)];
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

// ── build-er-tools ──

function cmdBuildErTools() {
  const ROOT = resolveRoot();
  const OUT_DIR = resolve(ROOT, 'out', 'tools');
  ensureDir(OUT_DIR);
  const watPath = resolve(ROOT, 'compiler', 'er-tools.wat');
  const wasmPath = resolve(OUT_DIR, 'er-tools.wasm');

  const src = readText(watPath);
  const wasm = codecCompile(src);
  writeFileSync(wasmPath, wasm);
  const size = wasm.length;
  console.log(`  ✓ ${wasmPath} (${(size / 1024).toFixed(0)} KB)`);
}

// ── build:cli ──

function cmdBuildCli(args) {
  const ROOT = resolveRoot();
  const WASM2ELF = rootPath('compiler', 'tools', 'wasm2elf.mjs');

  function showUsage() {
    const msg = `Usage: bun build.mjs build-cli <user-fragment.wat> [options]

Options:
  -o, --output <file>    Output ELF path (default: <name>.elf)
  --with-args            Include argument parsing (args_sizes_get / args_get)
  --with-memory          Include bump allocator and string utilities
  -h, --help             Show this help

Example:
  bun build.mjs build-cli cli/examples/hello-app.wat -o hello.elf
  ./hello.elf
  `;
    console.log(msg);
    process.exit(0);
  }

  if (args.length < 1 || args.includes('-h') || args.includes('--help')) showUsage();

  const userPath = args[0];
  if (!fileExists(userPath)) {
    console.error(`Error: user fragment not found: ${userPath}`);
    process.exit(1);
  }

  const oi = args.indexOf('-o');
  const oi2 = args.indexOf('--output');
  let outputPath;
  if (oi !== -1 && oi + 1 < args.length) outputPath = resolve(args[oi + 1]);
  else if (oi2 !== -1 && oi2 + 1 < args.length) outputPath = resolve(args[oi2 + 1]);
  else {
    const name = userPath.replace(/\.wat$/, '');
    outputPath = name.endsWith('.elf') ? name : name + '.elf';
  }

  const withArgs = args.includes('--with-args');
  const withMemory = args.includes('--with-memory');

  const cliDir = rootPath('cli');
  const LIBRARY = [
    resolve(cliDir, 'core.wat'),
    resolve(cliDir, 'io.wat'),
  ];
  if (withArgs) LIBRARY.push(resolve(cliDir, 'args.wat'));
  if (withMemory) LIBRARY.push(resolve(cliDir, 'memory.wat'));

  let body = '';
  for (const libPath of LIBRARY) {
    if (!fileExists(libPath)) {
      console.error(`Warning: library module not found: ${libPath}`);
      continue;
    }
    body += readText(libPath).trimEnd() + '\n\n';
  }
  body += `;; ── User code: ${userPath} ──\n`;
  body += readText(userPath).trimEnd() + '\n';
  const fullWat = wrapModule(body, { variant: 'cli' });

  const pid = process.pid;
  const tmpWat = `/tmp/cli-build-${pid}.wat`;
  const tmpWasm = `/tmp/cli-build-${pid}.wasm`;
  writeText(tmpWat, fullWat);

  if (!compileWat(tmpWat, tmpWasm)) {
    console.error(`Assembled WAT written to ${tmpWat} for debugging`);
    process.exit(1);
  }

  const r2 = Bun.spawnSync(['bun', WASM2ELF, tmpWasm, '-o', outputPath], { stdio: 'inherit' });
  if (r2.exitCode !== 0) {
    console.error('wasm2elf failed');
    process.exit(1);
  }

  try { unlinkSync(tmpWat); } catch {}
  try { unlinkSync(tmpWasm); } catch {}
  console.error(`✓ ${outputPath}`);
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
    case 'gen-compiler':
      cmdGenCompiler(args);
      break;
    case 'gen-stages':
      cmdGenStages(args);
      break;
    case 'build':
    case 'build:full':
    case 'build-full':
      cmdBuild(args);
      break;
    case 'build:cli':
    case 'build-cli':
      cmdBuildCli(args);
      break;
    case 'build:tool':
    case 'build-tool':
      cmdBuildErTools();
      break;
    case 'help':
    default:
      console.log(`
EdgeRun Build System — consolidated build tool

Usage: bun tools/build.mjs <command> [options]

Commands:
  gen-config              Generate config.wat (globals + memory + data)
  gen-compiler [opts]     Generate JIT compiler files (out/gen/jit-*.wat)
  gen-stages [--split]    Generate pipeline stages (out/gen/pipeline-stages.wat)
  build [opts]            Build with all 3 JIT backends (out/edgerun.wat + .wasm)
  build-cli <file> [opts] Build CLI ELF from a WAT fragment
  build-tool              Build er-tools WASM module (out/tools/er-tools.wasm)
  help                    Show this help
`);
  }
}

main();
