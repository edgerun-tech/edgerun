#!/usr/bin/env bun
import { execSync } from 'child_process';
import { readFileSync, statSync, existsSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const PASS = '\x1b[32m';
const FAIL = '\x1b[31m';
const RST = '\x1b[0m';
let passed = 0, failed = 0;

function check(ok, msg) { console.log(`  ${ok ? PASS + 'PASS' : FAIL + 'FAIL'}${RST}  ${msg}`); if (ok) passed++; else failed++; }
function run(cmd) { try { return execSync(cmd, { cwd: ROOT, stdio: 'pipe', encoding: 'utf-8' }); } catch { return null; } }

console.log(`\n  EdgeRun Validate — unified build check\n`);
check(run('which wasm-tools'), 'wasm-tools found');

// Only validate the unified build output — the single WASM binary is the only artifact
const r1 = run(`wasm-tools validate ${resolve(ROOT, 'edgerun.wat')}`);
check(r1 !== null, 'edgerun.wat validates');

const wasmOk = run(`wasm-tools validate ${resolve(ROOT, 'edgerun.wasm')}`);
check(wasmOk !== null, 'edgerun.wasm validates');

const wSize = existsSync(resolve(ROOT, 'edgerun.wasm')) ? statSync(resolve(ROOT, 'edgerun.wasm')).size : 0;
const sSize = existsSync(resolve(ROOT, 'edgerun-stripped.wasm')) ? statSync(resolve(ROOT, 'edgerun-stripped.wasm')).size : 0;
check(wSize > 100000, `edgerun.wasm: ${(wSize / 1024).toFixed(0)} KB`);

// Stage table entries
const content = readFileSync(resolve(ROOT, 'edgerun.wat'), 'utf-8');
const elem = content.match(/\(elem\s+\(i32\.const\s+\d+\)\s+\$\w+/g) || [];
check(elem.length >= 5, `stage table: ${elem.length} entries`);
for (const e of elem) console.log(`    ${e}`);

if (sSize > 0) console.log(`  stripped: ${(sSize / 1024).toFixed(0)} KB`);

console.log(`\n  ${failed === 0 ? PASS + 'All' : FAIL + failed + '/' + (passed + failed)}${RST} ${failed === 0 ? 'passed' : 'failed'}\n`);
process.exit(failed > 0 ? 1 : 0);
