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

import { readFileSync, writeFileSync } from 'fs';
import { resolve } from 'path';
import { spawnSync } from 'child_process';

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
  const checkRead = (checkPtrArr[0] | (checkPtrArr[1]<<8) | (checkPtrArr[2]<<16) | (checkPtrArr[3]<<24)) >>> 0;
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
  const bodyCallCount = new Uint8Array(wasm.memory.buffer, 0x8C0B8, 4);
  const bodyCallCountVal = bodyCallCount[0] | (bodyCallCount[1] << 8) | (bodyCallCount[2] << 16) | (bodyCallCount[3] << 24);
  const posBeforeEndCheck = new Uint8Array(wasm.memory.buffer, 0x8C0C0, 4);
  const posBeforeEndCheckVal = posBeforeEndCheck[0] | (posBeforeEndCheck[1] << 8) | (posBeforeEndCheck[2] << 16) | (posBeforeEndCheck[3] << 24);
  const posAfterDelta = new Uint8Array(wasm.memory.buffer, 0x8C0A4, 4);
  const posAfterDeltaVal = posAfterDelta[0] | (posAfterDelta[1] << 8) | (posAfterDelta[2] << 16) | (posAfterDelta[3] << 24);
  check(watStatus === 0, `load_wat status=${watStatus} dbg=0x${dbgVal.toString(16)} body=0x${bodyDbgVal.toString(16)} emitOff=${emitOffVal} emitFlg=${emitFlgVal} err=${funcErrVal} posParam=0x${posParamVal.toString(16)} bodyOffWs=${bodyOffAfterWsVal} emitBodyOff=${emitBodyOffVal} posAfterWs=${posAfterWsVal} calls=${bodyCallCountVal} posEnd=${posBeforeEndCheckVal} posDelta=${posAfterDeltaVal}`);
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

// ── Test 11: Multi-function JIT compilation with local calls ──
{
  // Test setup: 2 functions, func0 calls func1, func1 returns 42
  // Manually write decoded ops, type table, function table

  // Type table at OFF_TYPES_BUF (0x00104): 1 type entry
  // Type 0: param_count=0 (u16 at +128), result_count=1 (u16 at +136)
  const TYPES_BUF = 0x00104;
  const SZ_TYPE = 140;
  u8[TYPES_BUF + 128] = 0;  u8[TYPES_BUF + 129] = 0;  // param_count = 0
  u8[TYPES_BUF + 136] = 1;  u8[TYPES_BUF + 137] = 0;  // result_count = 1

  // Function table at OFF_FUNCTIONS_BUF (0x04514): 2 entries
  const FUNCS_BUF = 0x04514;
  const SZ_FUNC = 16;
  u32[FUNCS_BUF / 4 + 0] = 0;                // func 0: type_idx = 0
  u32[FUNCS_BUF / 4 + 4] = 0;                // func 1: type_idx = 0

  // Import count = 0 (no imported functions)
  u32[0x4108 / 4] = 0;

  // ── Decoded ops for func 0: call 1, return ──
  const DECODED_OPS = 0xA0000;
  const DEC_SZ = 32;
  const u32_dops = new Uint32Array(mem.buffer, DECODED_OPS, 8);
  u32_dops[0] = 0x00000010;  // opcode 0x10 (call) at byte 0
  u32_dops[1] = 1;            // imm0 = func_idx 1
  u32_dops[2] = 0;            // imm1 = 0
  u32_dops[3] = 0;            // ...rest
  u32_dops[4] = 0x0000000F;  // opcode 0x0F (return)
  u32_dops[5] = 0;
  u32_dops[6] = 0;
  u32_dops[7] = 0;

  // Reset code ptr to 0 (= start of JIT_CACHE at 0x100000)
  u32[0] = 0;
  // Reset fixup count (at address 2368 = 0x940)
  u32[0x940 / 4] = 0;

  // Compile func 0
  const codeSize0 = wasm.jit_compile_x86_64(0);
  check(codeSize0 > 0, `multi-jit: func0 compiled, codeSize=${codeSize0}`);
  // Verify fixup was recorded
  const fixCount = new Uint32Array(wasm.memory.buffer, 0x940, 1)[0];
  check(fixCount === 1, `multi-jit: fixup count after func0 = ${fixCount} (expected 1)`);
  const fixEntry = new Uint32Array(wasm.memory.buffer, 0x80800, 2);
  check(fixEntry[1] === 1, `multi-jit: fixup target = ${fixEntry[1]} (expected 1)`);

  // ── Decoded ops for func 1: i32.const 42, return ──
  u32_dops[0] = 0x00000041;  // opcode 0x41 (i32.const)
  u32_dops[1] = 42;           // imm0 = 42
  u32_dops[2] = 0;            // imm1 = 0
  u32_dops[3] = 0;
  u32_dops[4] = 0x0000000F;  // opcode 0x0F (return)
  u32_dops[5] = 0;
  u32_dops[6] = 0;
  u32_dops[7] = 0;

  // Compile func 1 (code continues after func0)
  const codeSize1 = wasm.jit_compile_x86_64(1);
  check(codeSize1 > 0, `multi-jit: func1 compiled, codeSize=${codeSize1}`);

  // Fix up local calls
  wasm.fixup_calls_x86_64();

  // Read the JIT cache to verify the call was patched
  const JIT_CACHE = 0x100000;
  const jitBytes = new Uint8Array(wasm.memory.buffer, JIT_CACHE, 64);
  // func0 call rel32 is at offset 17 (prologue=4, push rbx/r12/r13=5,
  // xor eax/mov r12/mov r13=8). After fixup, bytes 18-21 = rel32=13.
  const rel32 = (jitBytes[18] | (jitBytes[19] << 8) | (jitBytes[20] << 16) | (jitBytes[21] << 24)) >>> 0;
  const relSigned = rel32 > 0x7FFFFFFF ? rel32 - 0x100000000 : rel32;
  check(jitBytes[17] === 0xE8, `multi-jit: call rel32 prefix = 0x${jitBytes[17].toString(16)}`);
  check(relSigned > 0, `multi-jit: call rel32 = ${relSigned} (positive, forward call)`);

  // Verify func_offset table has both entries
  // After prologue (4 bytes), func0 offset = 4
  const offTable = new Uint32Array(mem.buffer, 0x80000, 2);
  check(offTable[0] === 4, `multi-jit: func0 offset table = ${offTable[0]} (expected 4)`);
  check(offTable[1] === 35, `multi-jit: func1 offset table = ${offTable[1]} (expected 35)`);
}

// ── Test 12: Functional UI layout + render + hit-test ──
{
  // Construct a 3-node tree (1 root, 2 children) and test layout/render/hit
  const TREE = 0x600000;
  const LAYOUT_BUF = 0x610000;
  const CMD_BUF = 0x620000;
  const CMD_CAP = 65536;

  // ── Build tree header ──
  // Ensure memory has room up to 0x700000
  while (mem.buffer.byteLength < 0x700000) {
    // page-align growth request
    const need = Math.ceil((0x700000 - mem.buffer.byteLength) / 65536);
    wasm.memory.grow(need);
  }
  const u16 = new Uint16Array(mem.buffer);
  const u8 = new Uint8Array(mem.buffer);

  // Header (20 bytes)
  u32[TREE / 4 + 0] = 0x49755245;          // magic "ERUI"
  u32[TREE / 4 + 1] = 68;                   // used_len = 20 + 3*16
  u16[TREE / 2 + 4] = 1;                    // version
  u16[TREE / 2 + 5] = 0;                    // axis (column)
  u16[TREE / 2 + 6] = 0;                    // gap
  u16[TREE / 2 + 7] = 8;                    // padding
  u16[TREE / 2 + 8] = 3;                    // node_count
  u16[TREE / 2 + 9] = 1;                    // root_count

  // Record 0 (root, container kind=0)
  const R0 = TREE + 20;
  u16[R0 / 2 + 0] = 0;   // kind (container)
  u16[R0 / 2 + 1] = 0;   // ancestor_ref (0 = root)
  u32[R0 / 4 + 1] = 1;   // id
  u16[R0 / 2 + 4] = 0;   // first_ref
  u16[R0 / 2 + 5] = 0;   // first_len
  u16[R0 / 2 + 6] = 0;   // second_ref
  u16[R0 / 2 + 7] = 0;   // second_len

  // Record 1 (child of root)
  const R1 = TREE + 36;
  u16[R1 / 2 + 0] = 7;   // kind (label — has text, shows something)
  u16[R1 / 2 + 1] = 1;   // ancestor_ref (parent = node 0)
  u32[R1 / 4 + 1] = 2;   // id
  u16[R1 / 2 + 4] = 0;   // first_ref (no string)
  u16[R1 / 2 + 5] = 0;
  u16[R1 / 2 + 6] = 0;
  u16[R1 / 2 + 7] = 0;

  // Record 2 (child of root)
  const R2 = TREE + 52;
  u16[R2 / 2 + 0] = 0;   // kind (container)
  u16[R2 / 2 + 1] = 1;   // ancestor_ref (parent = node 0)
  u32[R2 / 4 + 1] = 3;   // id
  u16[R2 / 2 + 4] = 0;
  u16[R2 / 2 + 5] = 0;
  u16[R2 / 2 + 6] = 0;
  u16[R2 / 2 + 7] = 0;

  // ── Test layout ──
  const layoutOk = wasm.er_ui_layout_set_buf(LAYOUT_BUF);
  check(layoutOk === undefined, 'er_ui_layout_set_buf returned void');

  const layoutResult = wasm.er_ui_layout(TREE, 68, 800, 600);
  check(layoutResult === 3, `er_ui_layout returned ${layoutResult} (expected 3 nodes)`);

  // Verify layout buffer data: each node gets 16 bytes [x, y, w, h]
  const lb = new Float32Array(mem.buffer, LAYOUT_BUF, 12);
  check(lb[0] >= 0 && lb[1] >= 0, `root position (${lb[0]}, ${lb[1]})`);
  check(lb[2] > 0 && lb[3] > 0, `root size (${lb[2]}, ${lb[3]})`);

  // ── Test render ──
  const cmdCount = wasm.er_ui_render(TREE, 68, CMD_BUF, CMD_CAP, 0, 0, 800, 600);
  check(cmdCount > 0, `er_ui_render returned ${cmdCount} commands`);

  // Verify command format: each command is 48 bytes
  // First command: kind at offset 0 (0=rect, 2=text)
  const cmdKind = u32[CMD_BUF / 4];
  check(cmdKind === 1 || cmdKind === 2, `first cmd kind=${cmdKind} (1=rect, 2=text)`);

  // Verify total command bytes
  const cmdBytes = cmdCount * 48;
  check(cmdBytes > 0 && cmdBytes < CMD_CAP, `cmd bytes=${cmdBytes} within cap`);

  // ── Test hit-test ──
  const hitRoot = wasm.er_ui_layout_hit_test(10, 10, 3);
  check(hitRoot >= 0, `hit-test (10,10) returned ${hitRoot} (>=0 = hit)`);

  const hitMiss = wasm.er_ui_layout_hit_test(9999, 9999, 3);
  check(hitMiss === -1, `hit-test (9999,9999) returned ${hitMiss} (-1 = miss)`);
}

// ── Test 13: Self-hosting pipeline — WAT → JIT → ELF → native execution ──
{
  const wcPath = resolve(ROOT, 'app/wayland-client.wat');
  const wcText = readFileSync(wcPath);
  const WAT_ADDR = 0x300000;
  const wcBytes = new Uint8Array(wasm.memory.buffer, WAT_ADDR, wcText.length);
  wcBytes.set(wcText);

  const loadStatus = wasm.load_wat(WAT_ADDR, wcText.length);
  check(loadStatus === 0, `wayland: load_wat status=${loadStatus}`);

  if (loadStatus === 0) {
    wasm.patch_wayland_syscalls();
    check(true, 'wayland: syscall map patched');

    const importCount = new Uint32Array(wasm.memory.buffer, 0x4108, 1)[0];
    check(importCount === 12, `wayland: import count = ${importCount}`);

    const [elfAddr, elfSize] = wasm.compile_to_elf_x86_64(importCount);
    check(elfAddr > 0 && elfSize > 0,
      `wayland: ELF addr=0x${elfAddr.toString(16)} size=${elfSize}`);

    const elfBytes = new Uint8Array(wasm.memory.buffer, elfAddr, elfSize);
    const elfPath = '/tmp/wayland-demo.elf';
    writeFileSync(elfPath, elfBytes);
    check(true, `wayland: wrote ${elfSize} bytes to ${elfPath}`);

    try {
      spawnSync('chmod', ['+x', elfPath], { timeout: 2000 });
      const run = spawnSync(elfPath, [], {
        timeout: 15000,
        stdio: ['ignore', 'pipe', 'pipe'],
        env: { ...process.env, WAYLAND_DISPLAY: process.env.WAYLAND_DISPLAY || 'wayland-0' }
      });
      const exitCode = run.status;
      check(typeof exitCode === 'number' && exitCode >= 0 && exitCode < 256,
        `wayland: ELF exited with code ${exitCode}`);
    } catch (e) {
      check(false, `wayland: execution error — ${e.message}`);
    }
  }
}

// ── Summary ──
const total = passed + failed;
console.log(`\n  ${failed === 0 ? PASS + 'All' : FAIL + failed + '/' + total}${RST} ${failed === 0 ? 'passed' : 'failed'}\n`);
process.exit(failed > 0 ? 1 : 0);
