#!/usr/bin/env bun
import { readFileSync, writeFileSync, existsSync } from 'fs';
import { execSync } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const JIT_WASM = join(__dirname, '..', '..', 'out', 'gen', 'jit-full-x86-64.wasm');

const OFF_TYPE_COUNT = 256;
const OFF_TYPES_BUF = 0x104;
const SZ_TYPE = 256;
const OFF_IMPORT_COUNT = 16648;
const OFF_FUNCTION_COUNT = 17680;
const OFF_FUNCTIONS_BUF = 0x4514;
const OFF_CODE_COUNT = 21784;
const OFF_CODE_BUF = 0x551C;
const SZ_CODE = 64;
const OFF_EXPORT_COUNT = 38176;
const OFF_EXPORTS_BUF = 38184;
const SZ_EXPORT = 32;
const OFF_START_FUNC = 43316;
const OFF_DECODED_OPS = 0xA0000;
const DEC_SZ = 32;
const JIT_CACHE = 0x100000;
const RETURN_EMITTED = 32;
const LABEL_DEPTH = 28;
const FIXUP_COUNT = 2368;
const TEXT_VA = 0x400000;
const BSS_SIZE = 0x100000;
const PAGE_SIZE = 0x1000;

const SYSCALL_MAP = {
  fd_write: 20, fd_read: 0, fd_close: 3, fd_seek: 8,
  proc_exit: 60, environ_sizes_get: 61, environ_get: 62,
  args_sizes_get: 63, args_get: 64,
};

function syscallForImport(modName, fnName) {
  const key = fnName.replace(/^[a-z]+_/, '');
  if (SYSCALL_MAP[fnName] !== undefined) return SYSCALL_MAP[fnName];
  if (SYSCALL_MAP[key] !== undefined) return SYSCALL_MAP[key];
  const lower = fnName.toLowerCase();
  if (lower.includes('exit')) return 60;
  if (lower.includes('write')) return 1;
  if (lower.includes('read')) return 0;
  return -1;
}

function readStr(buf, start, len) {
  let s = '';
  for (let i = 0; i < len; i++) s += String.fromCharCode(buf[start + i]);
  return s;
}

function parseWasm(buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  let pos = 0;

  if (dv.getUint32(pos, true) !== 0x6d736100) throw new Error('Not a WASM file');
  pos += 4;
  if (dv.getUint32(pos, true) !== 1) throw new Error('Unsupported WASM version');
  pos += 4;

  function leb() {
    let v = 0, s = 0, b;
    do { b = dv.getUint8(pos++); v |= (b & 0x7f) << s; s += 7; } while (b & 0x80);
    return v;
  }

  const types = [];
  const imports = [];
  let funcTypeIdxs = [];
  const codes = [];
  const exports = [];
  let dataCount = 0;
  let startFunc = -1;
  let memPages = 0;
  const dataSegments = [];

  while (pos < buf.length) {
    const secId = dv.getUint8(pos++);
    const secSize = leb();
    const secEnd = pos + secSize;
    const secStart = pos;

    switch (secId) {
      case 1: {
        const count = leb();
        for (let i = 0; i < count; i++) {
          if (dv.getUint8(pos++) !== 0x60) throw new Error('Expected functype');
          const pn = leb();
          const params = []; for (let j = 0; j < pn; j++) params.push(dv.getUint8(pos++));
          const rn = leb();
          const results = []; for (let j = 0; j < rn; j++) results.push(dv.getUint8(pos++));
          types.push({ params, results });
        }
        break;
      }
      case 2: {
        const count = leb();
        for (let i = 0; i < count; i++) {
          const modLen = leb();
          const modName = readStr(buf, pos, modLen); pos += modLen;
          const nameLen = leb();
          const fnName = readStr(buf, pos, nameLen); pos += nameLen;
          const kind = dv.getUint8(pos++);
          let typeIdx = 0;
          if (kind === 0) typeIdx = leb();
          imports.push({ kind, modName, fnName, typeIdx });
        }
        break;
      }
      case 3: {
        const count = leb();
        funcTypeIdxs = [];
        for (let i = 0; i < count; i++) funcTypeIdxs.push(leb());
        break;
      }
      case 5: {
        const count = leb();
        for (let i = 0; i < count; i++) {
          const limits = leb();
          const initial = leb();
          let max = initial;
          if (limits & 0x01) max = leb();
          if (i === 0) memPages = initial;
        }
        break;
      }
      case 7: {
        const count = leb();
        for (let i = 0; i < count; i++) {
          const nameLen = leb();
          const name = readStr(buf, pos, nameLen); pos += nameLen;
          const kind = dv.getUint8(pos++);
          const idx = leb();
          exports.push({ name, kind, idx });
        }
        break;
      }
      case 8: { startFunc = leb(); break; }
      case 9: pos = secEnd; break;
      case 10: {
        const count = leb();
        for (let i = 0; i < count; i++) {
          const bodySize = leb();
          codes.push({ bodyStart: pos, bodyEnd: pos + bodySize });
          pos += bodySize;
        }
        break;
      }
      case 11: {
        dataCount = leb();
        const parseInitExpr = () => {
          let off = 0;
          while (pos < secEnd && dv.getUint8(pos) !== 0x0b) {
            const op = dv.getUint8(pos++);
            if (op === 0x41) { off = (() => { let v = 0, s = 0, b; do { b = dv.getUint8(pos++); v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); if (s < 32 && (b & 0x40)) v |= -(1 << s); return v; })(); }
            else if (op === 0x10 || op === 0x23) { (() => { let v = 0, s = 0, b; do { b = dv.getUint8(pos++); v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })(); }
          }
          if (pos < secEnd) pos++;
          return off;
        };
        for (let i = 0; i < dataCount; i++) {
          const mode = leb();
          let offset = 0;
          if (mode === 0) {
            offset = parseInitExpr();
          } else if (mode === 1) {
            parseInitExpr();
          } else if (mode === 2) {
            leb(); offset = parseInitExpr();
          }
          const dataLen = leb();
          dataSegments.push({ offset, data: buf.slice(pos, pos + dataLen) });
          pos += dataLen;
        }
        break;
      }
      case 12: case 13: case 6: case 4:
      default: pos = secEnd; break;
    }
  }

  const numFuncImports = imports.filter(i => i.kind === 0).length;
  const funcCount = funcTypeIdxs.length;
  let mainFuncIdx = -1;
  for (const ex of exports) {
    if (ex.name === 'main' && ex.kind === 0) {
      mainFuncIdx = ex.idx;
      break;
    }
  }
  const codeIdx = mainFuncIdx >= 0 ? mainFuncIdx - numFuncImports : -1;

  return { types, imports, funcTypeIdxs, codes, exports, dataCount, startFunc,
           mainFuncIdx, codeIdx, memPages, dataSegments, codeBuf: buf, funcCount, numFuncImports };
}

function decodeOps(code, buf) {
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  let pos = code.bodyStart;
  const ops = [];

  function leb() {
    let v = 0, s = 0, b;
    do { b = dv.getUint8(pos++); v |= (b & 0x7f) << s; s += 7; } while (b & 0x80);
    return v;
  }
  function leb_signed() {
    let v = 0, s = 0, b;
    do { b = dv.getUint8(pos++); v |= (b & 0x7f) << s; s += 7; } while (b & 0x80);
    if (s < 32 && (b & 0x40)) v |= - (1 << s);
    return v;
  }

  // parse locals
  const localGroupCount = leb();
  let localCount = 0;
  for (let i = 0; i < localGroupCount; i++) {
    const count = leb();
    const type = dv.getUint8(pos++);
    localCount += count;
  }

  while (pos < code.bodyEnd) {
    const opcode = dv.getUint8(pos++);
    let imm0 = 0, imm1 = 0;

    switch (opcode) {
      case 0x00: break; // unreachable
      case 0x01: break; // nop
      case 0x02: case 0x03: case 0x04: imm0 = leb_signed(); break; // block/loop/if
      case 0x05: break; // else
      case 0x0B: break; // end
      case 0x0C: imm0 = leb(); break; // br
      case 0x0D: imm0 = leb(); break; // br_if
      case 0x0E: { imm0 = leb(); const t = leb(); imm1 = t; break; } // br_table
      case 0x0F: break; // return
      case 0x10: imm0 = leb(); break; // call
      case 0x11: imm0 = leb(); pos++; break; // call_indirect
      case 0x1A: break; // drop
      case 0x1B: break; // select
      case 0x20: case 0x21: case 0x22: imm0 = leb(); break; // local.get/set/tee
      case 0x23: case 0x24: imm0 = leb(); break; // global.get/set
      case 0x25: case 0x26: imm0 = leb(); break; // table.get/set
      case 0x28: case 0x29: case 0x2A: case 0x2B:
      case 0x2C: case 0x2D: case 0x2E: case 0x2F:
      case 0x30: case 0x31: case 0x32: case 0x33:
      case 0x34: case 0x35:
      case 0x36: case 0x37: case 0x38: case 0x39:
      case 0x3A: case 0x3B: case 0x3C: case 0x3D: case 0x3E: {
        leb(); imm0 = leb(); break; // align + offset
      }
      case 0x3F: case 0x40: break; // current_memory / grow_memory
      case 0x41: imm0 = leb_signed(); break;
      case 0x42: imm1 = leb_signed(); imm0 = Number(BigInt.asIntN(64, BigInt(imm1))); break; // i64.const
      case 0x43: imm0 = dv.getUint32(pos, true); pos += 4; break; // f32.const
      case 0x44: imm0 = dv.getUint32(pos, true); imm1 = dv.getUint32(pos + 4, true); pos += 8; break; // f64.const
      case 0x45: case 0x46: case 0x47: case 0x48: case 0x49: case 0x4A: case 0x4B: case 0x4C: case 0x4D: case 0x4E: case 0x4F:
      case 0x50: case 0x51: case 0x52: case 0x53: case 0x54: case 0x55: case 0x56: case 0x57: case 0x58: case 0x59: case 0x5A: case 0x5B:
      case 0x5C: case 0x5D: case 0x5E: case 0x5F:
      case 0x60: case 0x61: case 0x62: case 0x63: case 0x64: case 0x65: case 0x66: case 0x67: case 0x68: case 0x69: case 0x6A: case 0x6B:
      case 0x6C: case 0x6D: case 0x6E: case 0x6F:
      case 0x70: case 0x71: case 0x72: case 0x73: case 0x74: case 0x75: case 0x76: case 0x77: case 0x78: case 0x79: case 0x7A: case 0x7B:
      case 0x7C: case 0x7D: case 0x7E: case 0x7F:
      case 0x80: case 0x81: case 0x82: case 0x83: case 0x84: case 0x85: case 0x86: case 0x87: case 0x88: case 0x89: case 0x8A: case 0x8B:
      case 0x8C: case 0x8D: case 0x8E: case 0x8F:
      case 0x90: case 0x91: case 0x92: case 0x93: case 0x94: case 0x95: case 0x96: case 0x97: case 0x98: case 0x99: case 0x9A: case 0x9B:
      case 0x9C: case 0x9D: case 0x9E: case 0x9F:
      case 0xA0: case 0xA1: case 0xA2: case 0xA3: case 0xA4: case 0xA5: case 0xA6: case 0xA7: case 0xA8: case 0xA9: case 0xAA: case 0xAB:
      case 0xAC: case 0xAD: case 0xAE: case 0xAF:
      case 0xB0: case 0xB1: case 0xB2: case 0xB3: case 0xB4: case 0xB5: case 0xB6: case 0xB7: case 0xB8: case 0xB9: case 0xBA: case 0xBB:
      case 0xBC: case 0xBD: case 0xBE: case 0xBF: break;
      case 0xFC: case 0xFD: {
        // prefix opcodes - skip subopcode + remaining immediates
        if (opcode === 0xFC) { leb(); }
        else { leb(); }
        // for most prefix opcodes, no more immediates
        // but some have additional immediates - we skip to bodyEnd
        break;
      }
      default: break;
    }
    ops.push({ opcode, imm0, imm1 });
  }
  return { ops, localCount };
}

function writeELFFromWasm(inputPath, outputPath) {
  let wasmPath = inputPath;
  if (inputPath.endsWith('.wat')) {
    wasmPath = inputPath.replace(/\.wat$/, '.wasm');
    try {
      execSync(`wat2wasm "${inputPath}" -o "${wasmPath}"`, { stdio: 'pipe' });
    } catch (e) {
      throw new Error(`wat2wasm failed: ${e.stderr?.toString() || e.message}`);
    }
  }

  const buf = readFileSync(wasmPath);
  const mod = parseWasm(buf);

  if (mod.mainFuncIdx < 0 || mod.codeIdx < 0 || mod.codeIdx >= mod.codes.length)
    throw new Error('No exported "main" function found in code section');

  if (!existsSync(JIT_WASM))
    throw new Error('JIT WASM not found. Run: bun run gen');

  const jitWasm = readFileSync(JIT_WASM);
  const jitMod = new WebAssembly.Module(jitWasm);
  const inst = new WebAssembly.Instance(jitMod, {});
  const e = inst.exports;
  const mem = new Uint8Array(e.memory.buffer);
  const v = new DataView(e.memory.buffer);

  // ── Populate types ──
  v.setUint32(OFF_TYPE_COUNT, mod.types.length, true);
  for (let i = 0; i < mod.types.length; i++) {
    const t = mod.types[i];
    const base = OFF_TYPES_BUF + i * SZ_TYPE;
    for (let j = 0; j < t.params.length && j < 128; j++) mem[base + j] = t.params[j];
    v.setUint16(base + 128, t.params.length, true);
    for (let j = 0; j < t.results.length && j < 128; j++) mem[base + 132 + j] = t.results[j];
    v.setUint16(base + 136, t.results.length, true);
  }

  // ── Populate imports ──
  const funcImports = mod.imports.filter(i => i.kind === 0);
  v.setUint32(OFF_IMPORT_COUNT, funcImports.length, true);

  // ── Populate functions ──
  const totalFuncCount = funcImports.length + mod.funcTypeIdxs.length;
  v.setUint32(OFF_FUNCTION_COUNT, totalFuncCount, true);
  for (let i = 0; i < funcImports.length; i++) {
    v.setUint32(OFF_FUNCTIONS_BUF + i * 16, funcImports[i].typeIdx, true);
  }
  for (let i = 0; i < mod.funcTypeIdxs.length; i++) {
    v.setUint32(OFF_FUNCTIONS_BUF + (funcImports.length + i) * 16, mod.funcTypeIdxs[i], true);
  }

  // ── Populate code records ──
  v.setUint32(OFF_CODE_COUNT, mod.codes.length, true);
  let codeBodyBase = 0x10000;
  for (let i = 0; i < mod.codes.length; i++) {
    const code = mod.codes[i];
    const base = OFF_CODE_BUF + i * SZ_CODE;
    v.setUint32(base + 0, codeBodyBase, true);
    v.setUint32(base + 8, code.bodyEnd - code.bodyStart, true);
    for (let j = 0; j < code.bodyEnd - code.bodyStart; j++) {
      mem[codeBodyBase + j] = buf[code.bodyStart + j];
    }
    codeBodyBase += (code.bodyEnd - code.bodyStart) + 0x100;
  }

  // ── Populate exports ──
  v.setUint32(OFF_EXPORT_COUNT, mod.exports.length, true);
  for (let i = 0; i < mod.exports.length; i++) {
    const ex = mod.exports[i];
    const base = OFF_EXPORTS_BUF + i * SZ_EXPORT;
    for (let j = 0; j < ex.name.length && j < 16; j++) mem[base + j] = ex.name.charCodeAt(j);
    mem[base + 16] = ex.kind;
    v.setUint32(base + 20, ex.idx, true);
  }

  v.setUint32(OFF_START_FUNC, mod.mainFuncIdx, true);

  // ── Build syscall map ──
  const importSyscalls = funcImports.map((imp, idx) => {
    if (imp.kind !== 0) return -1;
    const sysno = syscallForImport(imp.modName, imp.fnName);
    if (sysno < 0) console.error(`Warning: no syscall mapping for import ${imp.modName}.${imp.fnName}`);
    return sysno;
  });
  for (let i = 0; i < importSyscalls.length; i++) {
    v.setUint32(0x90000 + i * 4, importSyscalls[i] >= 0 ? importSyscalls[i] : 0, true);
  }
  console.error(`Syscall map[${importSyscalls.length}]: ${Array.from({length: importSyscalls.length}, (_, i) => v.getUint32(0x90000 + i*4, true)).join(',')}`);

  // ── Copy WASM binary ──
  v.setUint32(64, 0x20000, true);
  for (let i = 0; i < buf.length; i++) mem[0x20000 + i] = buf[i];

  // ── Decode ops for each function ──
  let decodedOpOffset = 0;
  for (let fi = 0; fi < mod.codes.length; fi++) {
    const code = mod.codes[fi];
    const { ops, localCount } = decodeOps(code, buf);
    const startIdx = decodedOpOffset;

    for (let i = 0; i < ops.length; i++) {
      const base = OFF_DECODED_OPS + decodedOpOffset * DEC_SZ;
      const op = ops[i];
      v.setUint32(base + 0, op.opcode, true);
      v.setUint32(base + 4, op.imm0, true);
      v.setUint32(base + 8, op.imm1, true);

      if (op.opcode === 0x10) {
        const realFuncIdx = op.imm0;
        if (realFuncIdx < funcImports.length) {
          v.setUint32(base + 20, importSyscalls[realFuncIdx] >= 0 ? importSyscalls[realFuncIdx] : 0, true);
        }
      }
      decodedOpOffset++;
    }
    ops.push({ opcode: 0, imm0: 0 });

    // Update code record with decoded op info
    const codeBase = OFF_CODE_BUF + fi * SZ_CODE;
    v.setUint32(codeBase + 16, localCount, true);
    v.setUint32(codeBase + 24, startIdx, true);
    v.setUint32(codeBase + 32, ops.length, true);
  }
  v.setUint32(OFF_DECODED_OPS + decodedOpOffset * DEC_SZ, 0, true);

  // ── Reset JIT state ──
  v.setUint32(JIT_CACHE, 0, true);
  v.setUint32(LABEL_DEPTH, 0, true);
  v.setUint32(RETURN_EMITTED, 0, true);
  v.setUint32(FIXUP_COUNT, 0, true);

  // ── Compile ──
  console.error(`Before compile: syscall_map[0]=${v.getUint32(0x90000, true)}`);
  const result = e.compile_all_to_elf_x86_64();
  console.error(`After compile: syscall_map[0]=${v.getUint32(0x90000, true) ? 'OK' : 'ZERO'}`);
  try {
    const buf2 = new DataView(e.memory.buffer);
    console.error(`After compile syscall_map: ${Array.from({length:5}, (_, i) => buf2.getUint32(0x90000 + i*4, true)).join(',')}`);
    const jitCache = new Uint8Array(e.memory.buffer, 0x100000, 256);
    let hex = '';
    for (let i = 0; i < 256; i++) hex += jitCache[i].toString(16).padStart(2,'0');
    console.error(`JIT cache[0x100000..0x1000FF]: ${hex}`);
    // Dump more of JIT cache
    const jitCache2k = new Uint8Array(e.memory.buffer, 0x100000, 2048);
    let hex2 = '';
    for (let i = 0; i < 2048; i++) hex2 += jitCache2k[i].toString(16).padStart(2,'0');
    console.error(`JIT cache[0x100000..0x1007FF]: ${hex2}`);
    // Dump decoded ops for function 0 (first non-import)
    const decodedOps = new Uint8Array(e.memory.buffer, 0xA0000, 320);
    let doHex = '';
    for (let i = 0; i < 320; i++) doHex += decodedOps[i].toString(16).padStart(2,'0');
    console.error(`decoded_ops[0..319]: ${doHex}`);
    // Dump code entries for all functions
    const funcImportCount = buf2.getUint32(0x4108, true);
    // Dump func_off_table
    const funcOffTable = [];
    for (let fi = 0; fi < 14; fi++) {
      funcOffTable.push(buf2.getUint32(0x80000 + fi * 4, true));
    }
    console.error(`func_off_table[0..13]: ${funcOffTable.join(',')}`);
    // JS_CODE_PTR after all functions compiled
    console.error(`JS_CODE_PTR (at addr 0) = ${buf2.getUint32(0, true)}`);
    console.error(`JS_FIXUP_COUNT (at 2368) = ${buf2.getUint32(2368, true)}, JS_CALL_FIXUP_COUNT (at 4424) = ${buf2.getUint32(4424, true)}`);
    // Dump call fixup table (count is at 0x300940, but fixup_calls resets it to 0,
    // so just scan the table for non-zero entries)
    console.error(`call fixup table scan:`);
    for (let fi = 0; fi < 30; fi++) {
      const codeOff = buf2.getUint32(0x80800 + fi * 8, true);
      const target = buf2.getUint32(0x80800 + fi * 8 + 4, true);
      if (codeOff !== 0 || target !== 0) {
        const targetOff = funcOffTable[target] !== undefined ? funcOffTable[target] : 'uninit';
        console.error(`  fixup[${fi}]: code_off=0x${codeOff.toString(16)} target_func=${target} target_off=0x${typeof targetOff === 'number' ? targetOff.toString(16) : targetOff} rel32=${typeof targetOff === 'number' ? (targetOff - codeOff - 5).toString(16) : '?'}`);
      }
    }
    // Dump the compiled JIT cache bytes around the call fixup sites
    for (let fi = 0; fi < 9; fi++) {
      const base = 0x551C + fi * 64;
      const bodyAddr = buf2.getUint32(base + 0, true);
      const bodyLen = buf2.getUint32(base + 8, true);
      const localCount = buf2.getUint32(base + 16, true);
      const dStart = buf2.getUint32(base + 24, true);
      const dLen = buf2.getUint32(base + 32, true);
      console.error(`func[${fi+funcImportCount}]: bodyAddr=0x${bodyAddr.toString(16)} bodyLen=${bodyLen} localCount=${localCount} decoded_start=${dStart} decoded_len=${dLen}`);
      // Dump decoded ops for this function
      for (let j = 0; j < dLen && j < 20; j++) {
        const opBase = 0xA0000 + (dStart + j) * 32;
        const opcode = buf2.getUint32(opBase, true);
        const imm0 = buf2.getUint32(opBase + 4, true);
        const imm1 = buf2.getUint32(opBase + 8, true);
        const syscall = buf2.getUint32(opBase + 20, true);
        console.error(`  op[${j}]: opcode=0x${opcode.toString(16)} imm0=${imm0} imm1=${imm1} syscall=${syscall}${opcode===0x10?(imm0<funcImportCount?' (IMPORT)':''):''}`);
      }
    }
  } catch(e2) { console.error('detached:', e2.message); }
  const elfPtr = result[0];
  let elfSize = result[1];
  if (elfSize <= 0 || elfSize > 10000000) throw new Error(`Bad ELF size: ${elfSize}`);

  // ── Patch fd_write (SYS_writev=20) calls ──
  // The compiled code is at ELF_CODE_OFF = 256 within the ELF.
  // JIT uses 32-bit disp for ALL loads, so each mov r64,[rsp+N] is 8 bytes,
  // and add rsp,imm is 7 bytes (81 /0 + imm32). Total sequence:
  //   b8 14 00 00 00             mov eax, 20              (5)
  //   48 8b bc 24 00 00 00 00    mov rdi, [rsp]           (8)
  //   48 8b b4 24 08 00 00 00    mov rsi, [rsp+8]         (8)
  //   48 8b 94 24 10 00 00 00    mov rdx, [rsp+16]        (8)
  //   4c 8b 94 24 18 00 00 00    mov r10, [rsp+24]        (8)
  //   4c 8b 84 24 20 00 00 00    mov r8, [rsp+32]         (8)
  //   4c 8b 8c 24 28 00 00 00    mov r9, [rsp+40]         (8)
  //   0f 05                      syscall                  (2)
  //   48 81 c4 30 00 00 00       add rsp, 48              (7)
  //   (the JIT's call template reads params from the WASM stack but
  //    never pops them — our replacement must also pop the 4 WASM params
  //    to avoid stack corruption in the function epilogue.)
  //                                   total = 62 + 1(push) = 63
  // We replace 63 bytes with: call wrapper + add rsp, 80 + push rax + NOP fill
  const ELF_CODE_OFF = 256;
  const FDWRITE_PATTERN = [0xb8, 0x14, 0x00, 0x00, 0x00];
  const SEQ_SIZE = 63;

  // Generate fd_write wrapper x86-64 code
  // The call template stores 6 qwords on the stack (sub rsp, 48).
  // After "call wrapper", the return address is pushed, then "push rbp; mov rbp, rsp".
  // Stack layout from rbp:
  //   [rbp+16] = struct[0] = fd           (integer, no mem_ptr)
  //   [rbp+24] = struct[1] = iovs         (already has mem_ptr added by call template)
  //   [rbp+32] = struct[2] = iovs_len     (integer, no mem_ptr)
  //   [rbp+40] = struct[3] = nwritten     (already has mem_ptr added)
  //   [rbp+48] = struct[4] = 0            (unused)
  //   [rbp+56] = struct[5] = 0            (unused)
  // We read from struct slots and convert WASM iovec {u32,u32} → Linux {u64,u64}.
  let wrapper = [];
  let wrapperSize = 0;
  function genFdWriteWrapper() {
    const a = [];
    function e(...bytes) { a.push(...bytes); }
    e(
      0x55,                                        // push rbp
      0x48, 0x89, 0xe5,                            // mov rbp, rsp
      0x48, 0x83, 0xec, 0x10,                      // sub rsp, 16 (space for host iovec)
      // load fd → rdi
      0x48, 0x8b, 0x45, 0x10,                      // mov rax, [rbp+16] (fd)
      0x48, 0x89, 0xc7,                            // mov rdi, rax
      // load iovs → rsi (already has mem_ptr, points to WASM iovs in guest mem)
      0x48, 0x8b, 0x45, 0x18,                      // mov rax, [rbp+24] (iovs)
      0x48, 0x89, 0xc6,                            // mov rsi, rax
      // load iovs_len → rdx
      0x48, 0x8b, 0x45, 0x20,                      // mov rax, [rbp+32] (iovs_len)
      0x48, 0x89, 0xc2,                            // mov rdx, rax
      // load nwritten → r10 (already has mem_ptr)
      0x48, 0x8b, 0x45, 0x28,                      // mov rax, [rbp+40] (nwritten)
      0x49, 0x89, 0xc2,                            // mov r10, rax
      // if count == 0, skip conversion
      0x85, 0xd2,                                  // test edx, edx
    );
    const jzOff = a.length;
    e(0x74, 0x00);                                 // jz rel8 placeholder
    e(
      // convert first iov entry
      0x8b, 0x06,                                  // mov eax, [rsi] (WASM buf u32)
      0x4c, 0x01, 0xf0,                            // add rax, r14 (absolute address)
      0x48, 0x89, 0x45, 0xf0,                      // mov [rbp-16], rax (host iov_base)
      0x8b, 0x46, 0x04,                            // mov eax, [rsi+4] (WASM buf_len u32)
      0x48, 0x89, 0x45, 0xf8,                      // mov [rbp-8], rax (host iov_len)
      0x48, 0x8d, 0x75, 0xf0,                      // lea rsi, [rbp-16] (host iovec ptr)
      0xba, 0x01, 0x00, 0x00, 0x00,                // mov edx, 1 (one iovec entry)
    );
    const syscallOff = a.length;                   // skip target for jz
    e(
      0xb8, 0x14, 0x00, 0x00, 0x00,                // mov eax, 20 (SYS_writev)
      0x0f, 0x05,                                  // syscall
      0x41, 0x89, 0x02,                            // mov [r10], eax (result → *nwritten)
      0xc9,                                        // leave
      0xc3,                                        // ret
    );
    // Patch jz rel8
    a[jzOff + 1] = syscallOff - (jzOff + 2);
    return a;
  }

  // Find fd_write calls in compiled code
  const compiledStart = elfPtr + ELF_CODE_OFF;
  const compiledEnd = elfPtr + elfSize;
  const fdWriteCalls = [];
  for (let off = 0; off < elfSize - ELF_CODE_OFF - SEQ_SIZE; off++) {
    const base = compiledStart + off;
    let match = true;
    for (let j = 0; j < FDWRITE_PATTERN.length; j++) {
      if (mem[base + j] !== FDWRITE_PATTERN[j]) { match = false; break; }
    }
    if (match) fdWriteCalls.push(off);
  }

  if (fdWriteCalls.length > 0) {
    wrapper = genFdWriteWrapper();
    wrapperSize = wrapper.length;

    // Place wrapper after current ELF, extend buffer
    const wrapperOff = elfSize;
    while (mem.length < elfPtr + elfSize + wrapperSize + 1024) {
      mem.fill(0, mem.length, mem.length + 65536);
    }
    for (let j = 0; j < wrapperSize; j++) mem[elfPtr + wrapperOff + j] = wrapper[j];
    const newElfSize = elfSize + wrapperSize;

    // Patch each call site: call rel32 + add rsp, 48 + NOP fill
    const TEXT_VA = 0x400000;
    for (const callOff of fdWriteCalls) {
      const callSiteVA = TEXT_VA + ELF_CODE_OFF + callOff;
      const wrapperVA = TEXT_VA + wrapperOff;
      const rel32 = wrapperVA - (callSiteVA + 5);
      const relBuf = new Uint8Array(4);
      new DataView(relBuf.buffer).setInt32(0, rel32, true);
      const patchBase = compiledStart + callOff;
      mem[patchBase] = 0xe8;
      for (let j = 0; j < 4; j++) mem[patchBase + 1 + j] = relBuf[j];
      mem[patchBase + 5] = 0x48;
      mem[patchBase + 6] = 0x83;
      mem[patchBase + 7] = 0xc4;
      mem[patchBase + 8] = 0x50;  // add rsp, 80 (48 struct + 32 pop 4 WASM params)
      mem[patchBase + 9] = 0x50;  // push rax (return value onto clean WASM stack)
      for (let j = 10; j < SEQ_SIZE; j++) mem[patchBase + j] = 0x90;
    }

    // Update p_filesz and p_memsz in program header
    const phv2 = new DataView(mem.buffer, elfPtr + 64, 56);
    const oldFilesz = Number(phv2.getBigUint64(32, true));
    if (newElfSize > oldFilesz) {
      phv2.setBigUint64(32, BigInt(newElfSize), true);
      const oldMemsz = Number(phv2.getBigUint64(40, true));
      if (newElfSize > oldMemsz) {
        phv2.setBigUint64(40, BigInt(newElfSize), true);
      }
    }
    elfSize = newElfSize;
    console.error(`Patched ${fdWriteCalls.length} fd_write call(s), ELF extended by ${wrapperSize} bytes`);
  }

  // Re-read program header to get current p_filesz/p_memsz
  const elfHeader = mem.slice(elfPtr, elfPtr + 120);
  const pv = new DataView(elfHeader.buffer, elfHeader.byteOffset, elfHeader.byteLength);
  const origFilesz = Number(pv.getBigUint64(96, true));
  const origMemsz = Number(pv.getBigUint64(104, true));

  if (mod.dataSegments.length > 0 || mod.memPages > 1) {
    const pageAlign = (x) => (x + PAGE_SIZE - 1) & ~(PAGE_SIZE - 1);

    const guestMemOff = pageAlign(origFilesz) + 0x80;
    if (mod.dataSegments.length > 0) console.error(`DATA PATH: origFilesz=${origFilesz}, guestMemOff=${guestMemOff}`);
    const memSize = Math.max(mod.memPages || 1, 1) * 65536;
    const dataEnd = guestMemOff + memSize;

    // Ensure buffer is large enough
    while (mem.length < elfPtr + dataEnd) {
      mem.fill(0, mem.length, mem.length + 65536);
    }

    for (const seg of mod.dataSegments) {
      const segOff = guestMemOff + seg.offset;
      for (let j = 0; j < seg.data.length; j++) {
        mem[elfPtr + segOff + j] = seg.data[j];
      }
    }

    // Update p_filesz and p_memsz in the ELF output buffer
    // p_filesz must cover the data, p_memsz must be at least as large as the
    // original BSS region so the stub's syscall map (mem_ptr + 0x90000) stays mapped.
    const phdrBuf = mem.slice(elfPtr + 64, elfPtr + 120);
    const phv = new DataView(phdrBuf.buffer, phdrBuf.byteOffset, phdrBuf.byteLength);
    phv.setBigUint64(32, BigInt(Math.max(origFilesz, dataEnd)), true);
    phv.setBigUint64(40, BigInt(Math.max(origMemsz, pageAlign(guestMemOff + memSize))), true);
    // Write back patched phdr
    for (let i = 0; i < 56; i++) mem[elfPtr + 64 + i] = phdrBuf[i];

    const elfSize2 = dataEnd;
    const elfData = mem.slice(elfPtr, elfPtr + elfSize2);
    writeFileSync(outputPath, elfData);
    console.error(`Wrote ${elfData.length}-byte ELF to ${outputPath}`);
    return elfData;
  }

  console.error(`DEBUG: else path (no data/mem), dataSegments=${mod.dataSegments.length}, memPages=${mod.memPages}, elfSize=${elfSize}`);
  const elfData = mem.slice(elfPtr, elfPtr + elfSize);
  writeFileSync(outputPath, elfData);
  console.error(`Wrote ${elfData.length}-byte ELF to ${outputPath}`);
  return elfData;
}

// ── CLI ──
const args = process.argv.slice(2);
if (args.length < 1) {
  console.error(`Usage: wasm2elf <input.wasm|.wat> [-o <output.elf>]

Compiles a WASM/WAT module to a Linux x86-64 ELF binary.

The module must export a function named "main" that returns i32.
Supports WASM/WAT with imported functions, memory, data sections,
basic arithmetic, locals, and control flow.
`);
  process.exit(1);
}

const inputPath = args[0];
const oi = args.indexOf('-o');
const oi2 = args.indexOf('--output');
let outputPath;
if (oi !== -1 && oi + 1 < args.length) outputPath = args[oi + 1];
else if (oi2 !== -1 && oi2 + 1 < args.length) outputPath = args[oi2 + 1];
else outputPath = inputPath.replace(/\.(wat|wasm)$/, '') + '.elf';

if (!existsSync(inputPath)) {
  console.error(`Error: input file not found: ${inputPath}`);
  process.exit(1);
}

try {
  const elf = writeELFFromWasm(inputPath, outputPath);
  if (elf[0] === 0x7f && elf[1] === 0x45 && elf[2] === 0x4c && elf[3] === 0x46) {
    console.error('✓ Valid ELF magic');
  }
} catch (e) {
  console.error(`Error: ${e.message}`);
  process.exit(1);
}
