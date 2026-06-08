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
  compileWat, stripWasm, optimizeWasm, wrapModule, stripModuleWrapper
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

// ── Load manifest and build backend map ──
const manifestJson = JSON.parse(readFileSync(rootPath('manifest.json'), 'utf-8'));
const MANIFEST = manifestJson
  .filter(e => !e.builds || e.builds.includes('full'))
  .map(e => e.path);

const backendMap = {};
for (const e of manifestJson) {
  if (e.backends) backendMap[e.path] = e.backends[0];
}

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
    const suffixed = `${name}_${suffix}`;
    if (result.includes(suffixed)) continue; // already suffixed — skip to avoid duplicate
    const escaped = name.replace(/\$/g, '\\$');
    result = result.replace(new RegExp(escaped + '(?![a-zA-Z0-9_])', 'g'), suffixed);
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

function build() {
  const outPath = resolve(ROOT, argv.find((a) => a.startsWith('--out='))?.slice(6) || 'edgerun.wat');
  const skipWasm = process.argv.includes('--no-wasm');

  console.log(`EdgeRun Build — ${new Date().toISOString()}`);
  console.log(`Output: ${outPath}\n`);
  console.log(`All 3 JIT backends: x86-64 + ARM32 + AArch64\n`);

  let body = '';
  let count = 0;

  for (const filePath of MANIFEST) {
    const fullPath = resolve(ROOT, filePath);
    if (!existsSync(fullPath)) { console.warn(`  ⚠  ${filePath} not found — skipping`); continue; }
    let content = stripModuleWrapper(readFileSync(fullPath, 'utf-8'));

    if (backendMap[filePath]) {
      const suffix = backendMap[filePath];
      if (suffix !== 'x86_64') content = cleanBrokenWAT(content);
      content = renameBackend(content, suffix);
    }

    body += `;; ── ${filePath} ──\n${content.trimEnd()}\n\n`;
    count++;
  }

  const moduleWat = wrapModule(body);
  writeFileSync(outPath, moduleWat, 'utf-8');
  console.log(`✓ ${count} fragments → ${outPath} (${moduleWat.length} bytes)`);

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
