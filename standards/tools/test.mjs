#!/usr/bin/env bun
/**
 * EdgeRun Unified Test Suite
 *
 * Instantiates edgerun.wasm and tests all major subsystems:
 *  - Pipe I/O
 *  - Frame messaging
 *  - Pipeline lifecycle (create, set_stage, run)
 *  - Mux/demux operations
 *  - UI rendering
 *
 * Usage: bun tools/test.mjs
 */

import { readFileSync } from 'fs';
import { resolve } from 'path';

const ROOT = resolve(import.meta.dirname, '..');
const PASS = '\x1b[32m';
const FAIL = '\x1b[31m';
const RST = '\x1b[0m';

let passed = 0, failed = 0;

function check(ok, msg) {
  console.log(`  ${ok ? PASS + 'PASS' : FAIL + 'FAIL'}${RST}  ${msg}`);
  if (ok) passed++; else failed++;
}

function toU32(v) { return v >>> 0; }

// ── Instantiate WASM ──
let wasm;
try {
  const bin = readFileSync(resolve(ROOT, 'edgerun.wasm'));
  const mod = new WebAssembly.Module(bin);
  const instance = new WebAssembly.Instance(mod, {});
  wasm = instance.exports;
  check(true, 'edgerun.wasm instantiated');
} catch (e) {
  check(false, `instantiate: ${e.message}`);
  process.exit(1);
}

const mem = wasm.memory;
const u8 = new Uint8Array(mem.buffer);
const u32 = new Uint32Array(mem.buffer);

// ── Test 1: Pipe I/O ──
{
  const pipe = wasm.pipe_create(1024);
  check(pipe > 0 && pipe !== 0xFFFFFFFF, `pipe_create=${pipe}`);

  const DATA_ADDR = 0x2051000;
  const encoder = new TextEncoder();
  const msg = 'Hello EdgeRun!';
  const msgBytes = encoder.encode(msg);
  u8.set(msgBytes, DATA_ADDR);

  const written = wasm.pipe_write(pipe, DATA_ADDR, msgBytes.length);
  check(written === 0, `pipe_write=${written}`);

  const avail = wasm.pipe_available(pipe);
  check(avail === msgBytes.length, `pipe_available=${avail}`);

  const SCR = 0x2040000;
  const read = wasm.pipe_read(pipe, SCR, 256);
  check(read === msgBytes.length, `pipe_read=${read}`);

  const decoded = new TextDecoder().decode(u8.slice(SCR, SCR + read));
  check(decoded === msg, `data="${decoded}"`);

  const empty = wasm.pipe_available(pipe);
  check(empty === 0, `pipe_empty=${empty}`);
}

// ── Test 2: Frame I/O ──
{
  const pipe = wasm.pipe_create(512);
  const STREAM_ID = 42;
  const DATA_ADDR = 0x2051000;

  u8.set([0x48, 0x65, 0x6C, 0x6C, 0x6F], DATA_ADDR); // "Hello"
  const fw = wasm.frame_write(pipe, STREAM_ID, DATA_ADDR, 5);
  check(fw === 0, `frame_write=${fw}`);

  const SCR = 0x2040000;
  // frame_read returns pack(status, stream_id) as i64
  const result = wasm.frame_read(pipe, SCR, 4096);
  const status = toU32(Number(result >> 32n));
  const stream_id = toU32(Number(result & 0xFFFFFFFFn));
  check(status === 0, `frame_read status=${status}`);
  check(stream_id === STREAM_ID, `frame_read stream_id=${stream_id}`);

  const payloadLen = u32[SCR >>> 2];
  const payload = new TextDecoder().decode(u8.slice(SCR + 4, SCR + 4 + payloadLen));
  check(payload === 'Hello', `payload="${payload}"`);
}

// ── Test 3: Pipeline lifecycle ──
{
  const STAGE_PASSTHROUGH = wasm.STAGE_PASSTHROUGH();
  check(STAGE_PASSTHROUGH === 0, `STAGE_PASSTHROUGH=${STAGE_PASSTHROUGH}`);

  const MUX_IDX = wasm.STAGE_MUX_STATIC();
  check(MUX_IDX === 6, `STAGE_MUX_STATIC=${MUX_IDX}`);

  const DEMUX_IDX = wasm.STAGE_DEMUX_STATIC();
  check(DEMUX_IDX === 7, `STAGE_DEMUX_STATIC=${DEMUX_IDX}`);

  // Create pipes for pipeline test
  const input = wasm.pipe_create(1024);
  const output = wasm.pipe_create(1024);
  const DATA = 0x2051000;
  u8.set([0x41, 0x42, 0x43], DATA);
  wasm.pipe_write(input, DATA, 3);

  // Create pipeline with 1 passthrough stage
  const desc = wasm.pipeline_create(1024, 1);
  check(desc > 0, `pipeline_create=${desc}`);

  wasm.pipeline_set_stage(desc, 0, STAGE_PASSTHROUGH, 0, 0);
  const SCR = 0x2040000;
  const result = wasm.pipeline_run(desc, input, output, SCR, 4096);
  check(result === 0, `pipeline_run status=${result}`);

  const avail = wasm.pipe_available(output);
  check(avail === 3, `output has ${avail} bytes`);

  const read = wasm.pipe_read(output, SCR, 256);
  const decoded = new TextDecoder().decode(u8.slice(SCR, SCR + read));
  check(decoded === 'ABC', `passthrough=="${decoded}"`);
}

// ── Test 4: Stage table ──
{
  const table = wasm.stage_table;
  check(!!table, 'stage_table exists');
  check(table.length === 64, `stage_table length=${table.length}`);

  // Index 0 should have passthrough
  const fn = table.get(0);
  check(!!fn, 'stage_table[0] is populated');
}

// ── Test 5: UI framework exports ──
{
  const needed = ['er_ui_writer_begin', 'er_ui_writer_string', 'er_ui_render',
                  'er_ui_wasm_new_card', 'er_ui_wasm_new_badge'];
  let allPresent = true;
  for (const name of needed) {
    if (typeof wasm[name] !== 'function') {
      console.log(`    MISSING: ${name}`);
      allPresent = false;
    }
  }
  check(allPresent, 'UI framework exports present');
}

// ── Test 6: Compiler/interpreter exports ──
{
  const needed = ['decode_opcodes', 'compute_end_targets', 'load', 'call', 'get_result_value', 'get_result_count'];
  let allPresent = true;
  for (const name of needed) {
    if (typeof wasm[name] !== 'function') {
      console.log(`    MISSING: ${name}`);
      allPresent = false;
    }
  }
  check(allPresent, 'interpreter exports present');
}

// ── Test 7: All STAGE_* constants ──
{
  const expected = [
    ['STAGE_PASSTHROUGH', 0],
    ['STAGE_HEX_ENCODE', 1],
    ['STAGE_HEX_DECODE', 2],
    ['STAGE_B64_ENCODE', 3],
    ['STAGE_B64_DECODE', 4],
    ['STAGE_TRANSPORT', 5],
    ['STAGE_MUX_STATIC', 6],
    ['STAGE_DEMUX_STATIC', 7],
    ['STAGE_MUX_DYNAMIC', 8],
    ['STAGE_DEMUX_DYNAMIC', 9],
    ['STAGE_WS_FRAME', 10],
    ['STAGE_WS_ENCODE', 11],
    ['STAGE_WS_DECODE', 12],
    ['STAGE_EXEC', 13],
    ['STAGE_FRAME_PACER', 15],
    ['STAGE_SHA256', 16],
    ['STAGE_HMAC_SHA256', 17],
  ];
  let allOk = true;
  for (const [name, idx] of expected) {
    if (typeof wasm[name] !== 'function') {
      console.log(`    MISSING: ${name}`);
      allOk = false;
      continue;
    }
    const val = wasm[name]();
    if (val !== idx) {
      console.log(`    ${name}: expected ${idx}, got ${val}`);
      allOk = false;
    }
  }
  check(allOk, `${expected.length} STAGE_* constants correct`);
}

// ── Summary ──
const total = passed + failed;
console.log(`\n  ${failed === 0 ? PASS + 'All' : FAIL + failed + '/' + total}${RST} ${failed === 0 ? 'passed' : 'failed'}\n`);
process.exit(failed > 0 ? 1 : 0);
