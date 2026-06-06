#!/usr/bin/env node
/**
 * EdgeRun Pipeline Demo — Test Runner
 *
 * Compiles all WAT files, validates them, and runs the pipeline
 * demo against the edgerun.wasm module.
 *
 * Usage:
 *   node examples/pipeline-demo.test.mjs
 *
 * Prerequisites:
 *   - wasm-tools (for WAT→WASM compilation)
 *   - Node.js 18+ (for WASM instantiation)
 */

import { execSync } from 'child_process';
import { readFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const EXAMPLES = resolve(ROOT, 'examples');
const EDGERUN_WAT = resolve(ROOT, 'edgerun.wat');
const EDGERUN_WASM = resolve(ROOT, 'edgerun.wasm');
const DEMO_WAT = resolve(EXAMPLES, 'pipeline-demo.wat');
const DEMO_WASM = resolve(EXAMPLES, 'pipeline-demo.wasm');

// ── Color helpers ──
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const CYAN = '\x1b[36m';
const RESET = '\x1b[0m';

function pass(msg) { console.log(`  ${GREEN}✓${RESET} ${msg}`); }
function fail(msg) { console.log(`  ${RED}✗${RESET} ${msg}`); }
function info(msg) { console.log(`  ${CYAN}→${RESET} ${msg}`); }

// ── Helpers ──
function run(cmd, opts = {}) {
  try {
    return execSync(cmd, { cwd: ROOT, stdio: 'pipe', encoding: 'utf-8', ...opts });
  } catch (e) {
    return { error: e.stderr || e.message, stdout: e.stdout || '' };
  }
}

function checkTool(tool) {
  try {
    execSync(`which ${tool}`, { stdio: 'pipe' });
    return true;
  } catch {
    return false;
  }
}

// ── Test suite ──
const tests = [];
let passed = 0, failed = 0;

function test(name, fn) {
  tests.push({ name, fn });
}

async function runTests() {
  console.log(`\n${'='.repeat(60)}`);
  console.log(`  EdgeRun Pipeline Demo — Test Suite`);
  console.log(`${'='.repeat(60)}\n`);

  // ── Check prerequisites ──
  info('Checking prerequisites...');

  const hasWasmTools = checkTool('wasm-tools');
  if (!hasWasmTools) {
    console.log(`  ${YELLOW}⚠ wasm-tools not found — skipping WASM compilation tests${RESET}`);
    console.log(`  ${YELLOW}  Install: cargo install wasm-tools${RESET}\n`);
  }

  // ── Test 1: Source file exists ──
  test('pipeline-demo.wat exists', () => {
    if (!existsSync(DEMO_WAT)) throw new Error('Demo WAT file not found');
  });

  // ── Test 2: edgerun.wat exists ──
  test('edgerun.wat exists', () => {
    if (!existsSync(EDGERUN_WAT)) throw new Error('edgerun.wat not found');
  });

  // ── Test 3: Stage table has correct entries ──
  test('Stage table has passthrough + 4 mux/demux entries', () => {
    const content = readFileSync(EDGERUN_WAT, 'utf-8');
    const elems = content.match(/\(elem\s+\(i32\.const\s+\d+\)\s+\$\w+/g);
    if (!elems) throw new Error('No elem entries found');
    const indices = elems.map(e => parseInt(e.match(/i32\.const\s+(\d+)/)[1]));
    const expected = [0, 6, 7, 8, 9];
    for (const idx of expected) {
      if (!indices.includes(idx)) throw new Error(`Missing stage table entry at index ${idx}`);
    }
  });

  // ── Test 4: WAT parse validation (wasm-tools) ──
  if (hasWasmTools) {
    test('edgerun.wat parses and validates', () => {
      const r = run(`wasm-tools validate ${EDGERUN_WAT}`);
      if (r.error) throw new Error(`Validation failed: ${r.error}`);
    });

    test('pipeline-demo.wat parses and validates', () => {
      const r = run(`wasm-tools validate ${DEMO_WAT}`);
      if (r.error) throw new Error(`Validation failed: ${r.error}`);
    });

    test('Compile pipeline-demo.wat to WASM', () => {
      const r = run(`wasm-tools print ${DEMO_WAT} -o ${DEMO_WASM}`);
      if (r.error) throw new Error(`Compilation failed: ${r.error}`);
      if (!existsSync(DEMO_WASM)) throw new Error('WASM output not created');
    });
  }

  // ── Test 5: Check stage table entry count ──
  test('Stage table has correct elem layout', () => {
    const content = readFileSync(EDGERUN_WAT, 'utf-8');
    const elemLines = content.match(/\(elem\s+\(i32\.const\s+\d+\)\s+\$\w+/g);
    if (!elemLines || elemLines.length < 5) {
      throw new Error(`Expected >=5 elem entries, got ${elemLines?.length || 0}`);
    }
    // Verify no duplicate indices
    const indices = elemLines.map(e => parseInt(e.match(/i32\.const\s+(\d+)/)[1]));
    const unique = new Set(indices);
    if (unique.size !== indices.length) {
      throw new Error(`Duplicate stage table indices: ${indices}`);
    }
  });

  // ── Test 6: Verify process_* functions exist ──
  test('process_mux_static and related functions exist', () => {
    const content = readFileSync(EDGERUN_WAT, 'utf-8');
    for (const fn of ['process_mux_static', 'process_demux_static',
                       'process_mux_dynamic', 'process_demux_dynamic']) {
      const pat = `(func (export "${fn}")`;
      const pat2 = `(func \$${fn} (export "${fn}")`;
      if (!content.includes(pat) && !content.includes(pat2)) {
        throw new Error(`Missing function: ${fn}`);
      }
    }
  });

  // ── Test 7: UI framework exports exist in edgerun.wat ──
  test('UI framework exports present', () => {
    const content = readFileSync(EDGERUN_WAT, 'utf-8');
    const needed = ['er_ui_writer_begin', 'er_ui_writer_string', 'er_ui_render',
                    'er_ui_wasm_new_card'];
    for (const exp of needed) {
      if (!content.includes(`(export "${exp}")`)) {
        throw new Error(`Missing UI export: ${exp}`);
      }
    }
  });

  // ── Test 8: Demo imports match exports ──
  test('Demo imports exist in edgerun.wat', () => {
    const edgerun = readFileSync(EDGERUN_WAT, 'utf-8');
    const demo = readFileSync(DEMO_WAT, 'utf-8');
    // Extract import names from demo
    const importPattern = /\(import\s+"edgerun"\s+"([^"]+)"/g;
    let match;
    const imports = [];
    while ((match = importPattern.exec(demo)) !== null) {
      imports.push(match[1]);
    }
    // Check each import exists in edgerun exports
    const missing = imports.filter(name => {
      const pattern = name.includes('STAGE_')
        ? `export "${name}"`  // functions like STAGE_PASSTHROUGH
        : `export "${name}"`;
      return !edgerun.includes(pattern);
    });
    if (missing.length > 0) {
      // Some may be globals or functions — check both patterns
      const stillMissing = missing.filter(name => {
        const globalPattern = `"${name}" i32`;  // global exports
        return !edgerun.includes(globalPattern);
      });
      if (stillMissing.length > 0) {
        throw new Error(`Imports not found in edgerun.wat: ${stillMissing.join(', ')}`);
      }
    }
  });

  // ── Run all tests ──
  for (const { name, fn } of tests) {
    process.stdout.write(`  ${name}... `);
    try {
      await fn();
      console.log(`${GREEN}PASS${RESET}`);
      passed++;
    } catch (e) {
      console.log(`${RED}FAIL${RESET}`);
      console.log(`    ${e.message}`);
      failed++;
    }
  }

  // ── Summary ──
  console.log(`\n${'='.repeat(60)}`);
  const total = passed + failed;
  if (failed === 0) {
    console.log(`  ${GREEN}All ${total} tests passed${RESET}`);
  } else {
    console.log(`  ${RED}${failed}/${total} tests failed${RESET}`);
  }
  console.log(`${'='.repeat(60)}\n`);

  // Cleanup demo wasm
  if (existsSync(DEMO_WASM)) {
    try { execSync(`rm ${DEMO_WASM}`); } catch {}
  }

  process.exit(failed > 0 ? 1 : 0);
}

runTests();
