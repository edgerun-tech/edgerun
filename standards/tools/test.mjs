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
  const instance = new WebAssembly.Instance(mod, {
    host: {
      sock_open: () => -1,
      sock_send: () => -1,
      sock_recv: () => -1,
      sock_close: () => {},
    },
    linux: {
      poll: () => -1,
      mmap: (addr, len, prot, flags, fd, off) => 0,
      munmap: () => 0,
      socket: () => -1,
      connect: () => -1,
      sendmsg: () => -1,
      memfd_create: () => -1,
      ftruncate: () => -1,
    },
  });
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

  const DATA_ADDR = 0x1000000;
  const encoder = new TextEncoder();
  const msg = 'Hello EdgeRun!';
  const msgBytes = encoder.encode(msg);
  u8.set(msgBytes, DATA_ADDR);

  const written = wasm.pipe_write(pipe, DATA_ADDR, msgBytes.length);
  check(written === 0, `pipe_write=${written}`);

  const avail = wasm.pipe_available(pipe);
  check(avail === msgBytes.length, `pipe_available=${avail}`);

  const SCR = 0x1004000;
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
  const DATA_ADDR = 0x1000010;

  u8.set([0x48, 0x65, 0x6C, 0x6C, 0x6F], DATA_ADDR); // "Hello"
  const fw = wasm.frame_write(pipe, STREAM_ID, DATA_ADDR, 5);
  check(fw === 0, `frame_write=${fw}`);

  const SCR = 0x1004000;
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
  const DATA = 0x1000020;
  u8.set([0x41, 0x42, 0x43], DATA);
  wasm.pipe_write(input, DATA, 3);

  // Create pipeline with 1 passthrough stage
  const desc = wasm.pipeline_create(1024, 1);
  check(desc > 0, `pipeline_create=${desc}`);

  wasm.pipeline_set_stage(desc, 0, STAGE_PASSTHROUGH, 0, 0);
  const SCR = 0x1004000;
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
  check(table.length === 144, `stage_table length=${table.length}`);

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

// ── Test 8: Metadata introspection ──
{
  const modCount = wasm.metadata_module_count();
  check(modCount > 0, `metadata_module_count=${modCount}`);

  const funcCount = wasm.metadata_function_count();
  check(funcCount > 0, `metadata_function_count=${funcCount}`);

  const globalCount = wasm.metadata_global_count();
  check(globalCount > 0, `metadata_global_count=${globalCount}`);

  // Verify we can find a known function
  const DATA = 0x1000030;
  const enc = new TextEncoder();
  u8.set(enc.encode('pack'), DATA);
  const fIdx = wasm.metadata_find_function(DATA, 4);
  check(fIdx >= 0, `metadata_find_function('pack')=${fIdx}`);

  // Verify we can find a known global
  u8.set(enc.encode('INT_ERR'), DATA);
  const gIdx = wasm.metadata_find_global(DATA, 7);
  check(gIdx >= 0, `metadata_find_global('INT_ERR')=${gIdx}`);

  // List functions
  const LIST = 0x1001000;
  const written = wasm.metadata_list_functions(LIST, 65536);
  check(written > 0, `metadata_list_functions wrote ${written} bytes`);

  // Verify source line for a known function
  const packIdx = fIdx;
  const packLine = wasm.metadata_function_source_line(packIdx);
  check(packLine >= 60 && packLine <= 66, `metadata_function_source_line(pack)=${packLine}`);

  // Verify find_export works for exported functions
  u8.set(enc.encode('memcpy'), DATA);
  const expResult = wasm.metadata_find_export(DATA, 6);
  const expKind = Number(expResult >> 32n);
  const expIdx = Number(expResult & 0xffffffffn);
  check(expKind === 1 && expIdx >= 0, `metadata_find_export('memcpy')=pack(${expKind},${expIdx})`);

  // Unknown export returns pack(0, -1)
  u8.set(enc.encode('nonexistent'), DATA);
  const badExp = wasm.metadata_find_export(DATA, 11);
  const badIdx = Number(badExp & 0xffffffffn);
  const badKind = Number(badExp >> 32n);
  check(badKind === 0 && badIdx === 4294967295, `metadata_find_export('nonexistent')=pack(${badKind},${badIdx})`);

  // Verify module_functions for module 0 (module-header.wat)
  const MODBUF = 0x1002000;
  const mFuncCount = wasm.metadata_module_functions(0, MODBUF, 256);
  check(mFuncCount > 0, `metadata_module_functions(0) returned ${mFuncCount} functions`);

  // Verify search_functions finds functions matching a pattern
  u8.set(enc.encode('sha256'), DATA);
  const SRCHBUF = 0x1003000;
  const matchCount = wasm.metadata_search_functions(DATA, 6, SRCHBUF, 256);
  check(matchCount >= 3, `metadata_search_functions('sha256') found ${matchCount} matches`);
}

// ── Test 9: WASM emitter round-trip ──
{
  // Minimal module: (func (export "f") (result i32) i32.const 42)
  // Encodes to 34 bytes: magic(8) + type(7) + func(4) + export(7) + code(8)
  const SRC = 0x500000;
  const OUT = 0x520000;
  const moduleBytes = new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00,
    0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2A, 0x0B,
  ]);

  u8.set(moduleBytes, SRC);
  const loadStatus = wasm.load(SRC, moduleBytes.length);
  check(loadStatus === 0, `load minimal module: status=${loadStatus}`);

  const emitResult = wasm.emit_wasm(OUT, 4096);
  const emitStatus = Number(emitResult >> 32n);
  const emitSize = Number(emitResult & 0xFFFFFFFFn);
  check(emitStatus === 0, `emit_wasm status=${emitStatus}`);
  check(emitSize === moduleBytes.length, `emit_wasm size ${emitSize} == ${moduleBytes.length}`);

  // Verify magic header
  check(
    u8[OUT + 0] === 0x00 && u8[OUT + 1] === 0x61 &&
    u8[OUT + 2] === 0x73 && u8[OUT + 3] === 0x6D,
    'emitted magic \\0asm'
  );
  check(
    u8[OUT + 4] === 0x01 && u8[OUT + 5] === 0x00 &&
    u8[OUT + 6] === 0x00 && u8[OUT + 7] === 0x00,
    'emitted version 1'
  );
}

// ── Test 10: WAT load + emit round-trip ──
{
  // Minimal WAT: (func (export "f") (result i32) i32.const 42)
  const WAT = 0x540000;
  const OUT = 0x560000;
  // Direct memory consistency check before load_wat
  const memCheck = new Uint8Array(wasm.memory.buffer, WAT, 4);
  const origBytes = [memCheck[0], memCheck[1], memCheck[2], memCheck[3]];
  // Write 0xAABBCCDD to 0x8C000 manually
  const checkPtrArr = new Uint8Array(wasm.memory.buffer, 0x8C000, 4);
  checkPtrArr[0] = 0xDD; checkPtrArr[1] = 0xCC; checkPtrArr[2] = 0xBB; checkPtrArr[3] = 0xAA;
  const checkRead = checkPtrArr[0] | (checkPtrArr[1]<<8) | (checkPtrArr[2]<<16) | (checkPtrArr[3]<<24);
  check(checkRead === 0xAABBCCDD, `memory[0x8C000] before load_wat=0x${checkRead.toString(16)}`);

  const watBytes = new TextEncoder().encode('(module (func (export "f") (result i32) i32.const 42))');
  u8.set(watBytes, WAT);
  // Verify WAT was written correctly
  const watCheck = new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, WAT, watBytes.length));
  check(watCheck === new TextDecoder().decode(watBytes), `WAT source check: "${watCheck.substring(0, 20)}..."`);
  // Check byte at the start
  const firstByte = new Uint8Array(wasm.memory.buffer, WAT, 1)[0];
  check(firstByte === 0x28, `first byte=0x${firstByte.toString(16)}`);
  const watStatus = wasm.load_wat(WAT, watBytes.length);
  const dbg = new Uint8Array(wasm.memory.buffer, 0x8C020, 4);
  const dbgVal = dbg[0] | (dbg[1] << 8) | (dbg[2] << 16) | (dbg[3] << 24);
  // Debug markers from $wat_parse_body at 0x8C048
  const bodyDbg = new Uint8Array(wasm.memory.buffer, 0x8C048, 4);
  const bodyDbgVal = bodyDbg[0] | (bodyDbg[1] << 8) | (bodyDbg[2] << 16) | (bodyDbg[3] << 24);
  // emit_byte failure debug at 0x8C06C
  const emitOff = new Uint8Array(wasm.memory.buffer, 0x8C06C, 4);
  const emitOffVal = emitOff[0] | (emitOff[1] << 8) | (emitOff[2] << 16) | (emitOff[3] << 24);
  const emitFlg = new Uint8Array(wasm.memory.buffer, 0x8C070, 4);
  const emitFlgVal = emitFlg[0] | (emitFlg[1] << 8) | (emitFlg[2] << 16) | (emitFlg[3] << 24);
  // Error code from $wat_parse_func_decl at 0x8C01C (OFF_WAT_TMP)
  const funcErr = new Uint8Array(wasm.memory.buffer, 0x8C01C, 4);
  const funcErrVal = funcErr[0] | (funcErr[1] << 8) | (funcErr[2] << 16) | (funcErr[3] << 24);
  // Additional debug: pos param (0x8C060), body_off after skip_ws (0x8C084), emit body_off (0x8C090)
  const posParam = new Uint8Array(wasm.memory.buffer, 0x8C060, 4);
  const posParamVal = posParam[0] | (posParam[1] << 8) | (posParam[2] << 16) | (posParam[3] << 24);
  const bodyOffAfterWs = new Uint8Array(wasm.memory.buffer, 0x8C084, 4);
  const bodyOffAfterWsVal = bodyOffAfterWs[0] | (bodyOffAfterWs[1] << 8) | (bodyOffAfterWs[2] << 16) | (bodyOffAfterWs[3] << 24);
  const emitBodyOff = new Uint8Array(wasm.memory.buffer, 0x8C090, 4);
  const emitBodyOffVal = emitBodyOff[0] | (emitBodyOff[1] << 8) | (emitBodyOff[2] << 16) | (emitBodyOff[3] << 24);
  const posAfterWs = new Uint8Array(wasm.memory.buffer, 0x8C094, 4);
  const posAfterWsVal = posAfterWs[0] | (posAfterWs[1] << 8) | (posAfterWs[2] << 16) | (posAfterWs[3] << 24);
  check(watStatus === 0, `load_wat status=${watStatus} dbg=0x${dbgVal.toString(16)} body=0x${bodyDbgVal.toString(16)} emitOff=${emitOffVal} emitFlg=${emitFlgVal} err=${funcErrVal} posParam=0x${posParamVal.toString(16)} bodyOffWs=${bodyOffAfterWsVal} emitBodyOff=${emitBodyOffVal} posAfterWs=${posAfterWsVal}`);
  // Check what was stored at OFF_WAT_PTR after load_wat
  const watPtr = new Uint8Array(wasm.memory.buffer, 0x8C000, 4);
  const watPtrVal = watPtr[0] | (watPtr[1] << 8) | (watPtr[2] << 16) | (watPtr[3] << 24);
  // Also check the saved ptr
  const savPtr = new Uint8Array(wasm.memory.buffer, 0x8C010, 4);
  const savPtrVal = savPtr[0] | (savPtr[1] << 8) | (savPtr[2] << 16) | (savPtr[3] << 24);
  check(watPtrVal === WAT, `OFF_WAT_PTR=0x${watPtrVal.toString(16)} sav=0x${savPtrVal.toString(16)}`);

  const emitResult = wasm.emit_wasm(OUT, 4096);
  const emitStatus = Number(emitResult >> 32n);
  const emitSize = Number(emitResult & 0xFFFFFFFFn);
  check(emitStatus === 0, `emit_wasm from WAT status=${emitStatus}`);
  check(emitSize > 0, `emit_wasm from WAT size=${emitSize}`);
}

// ── Summary ──
const total = passed + failed;
console.log(`\n  ${failed === 0 ? PASS + 'All' : FAIL + failed + '/' + total}${RST} ${failed === 0 ? 'passed' : 'failed'}\n`);
process.exit(failed > 0 ? 1 : 0);
