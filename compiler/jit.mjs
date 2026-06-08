import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dir = dirname(fileURLToPath(import.meta.url));

let _inst = null;
let _mem = null;
let _dv = null;

function stubSyscall(name) {
  return (...args) => { throw Error(`syscall ${name} not available in JIT mode`); };
}

function inst() {
  if (!_inst) {
    const wasm = readFileSync(resolve(__dir, '..', 'out', 'edgerun.wasm'));
    const mod = new WebAssembly.Module(wasm);
    const importObj = {
      'wasi_snapshot_preview1': {
        fd_write: stubSyscall('fd_write'),
        fd_read: stubSyscall('fd_read'),
        proc_exit: stubSyscall('proc_exit'),
        args_sizes_get: stubSyscall('args_sizes_get'),
        args_get: stubSyscall('args_get'),
      },
      'linux': {
        read: stubSyscall('read'),
        write: stubSyscall('write'),
        open: stubSyscall('open'),
        close: stubSyscall('close'),
        poll: stubSyscall('poll'),
        mmap: stubSyscall('mmap'),
        munmap: stubSyscall('munmap'),
        socket: stubSyscall('socket'),
        connect: stubSyscall('connect'),
        sendmsg: stubSyscall('sendmsg'),
        memfd_create: stubSyscall('memfd_create'),
        ftruncate: stubSyscall('ftruncate'),
      }
    };
    _inst = new WebAssembly.Instance(mod, importObj);
    _mem = new Uint8Array(_inst.exports.memory.buffer);
    _dv = new DataView(_inst.exports.memory.buffer);
  }
  return _inst;
}

function mem() { inst(); return _mem; }
function dv() { inst(); return _dv; }

// ── LEB128 helpers ────────────────────────────────────────────
function lebU(v) { const b = []; do { let byte = v & 0x7f; v >>>= 7; if (v) byte |= 0x80; b.push(byte); } while (v); return b; }
function lebS(v) { const b = []; let m = 1; while (m) { let byte = v & 0x7f; v >>= 7; if ((v === 0 && !(byte & 0x40)) || (v === -1 && (byte & 0x40))) m = 0; else byte |= 0x80; b.push(byte); } return b; }

// ── WASM binary parsing ───────────────────────────────────────
function parseSections(buf) {
  const u8 = new Uint8Array(buf);
  const dv = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  let pos = 0;
  if (dv.getUint32(pos, true) !== 0x6d736100) throw Error('not wasm');
  pos += 8;

  const types = [];
  const imports = [];
  let funcTypeIdxs = [];
  const codes = [];
  const exports = [];
  let startFunc = -1;

  while (pos < buf.length) {
    const secId = u8[pos++];
    const secSize = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
    const secEnd = pos + secSize;

    switch (secId) {
      case 1: {
        const count = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
        for (let i = 0; i < count; i++) {
          if (u8[pos++] !== 0x60) throw Error('expected functype');
          const pn = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          const params = []; for (let j = 0; j < pn; j++) params.push(u8[pos++]);
          const rn = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          const results = []; for (let j = 0; j < rn; j++) results.push(u8[pos++]);
          types.push({ params, results });
        }
        break;
      }
      case 2: {
        const count = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
        for (let i = 0; i < count; i++) {
          const ml = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          let mn = ''; for (let j = 0; j < ml; j++) mn += String.fromCharCode(u8[pos++]);
          const nl = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          let fn = ''; for (let j = 0; j < nl; j++) fn += String.fromCharCode(u8[pos++]);
          const kind = u8[pos++];
          let ti = 0; if (kind === 0) ti = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          imports.push({ kind, modName: mn, fnName: fn, typeIdx: ti });
        }
        break;
      }
      case 3: {
        const count = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
        funcTypeIdxs = [];
        for (let i = 0; i < count; i++) funcTypeIdxs.push((() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })());
        break;
      }
      case 7: {
        const count = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
        for (let i = 0; i < count; i++) {
          const nl = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          let n = ''; for (let j = 0; j < nl; j++) n += String.fromCharCode(u8[pos++]);
          const kind = u8[pos++];
          const idx = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          exports.push({ name: n, kind, idx });
        }
        break;
      }
      case 8: startFunc = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })(); break;
      case 10: {
        const count = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
        for (let i = 0; i < count; i++) {
          const bodySize = (() => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; })();
          codes.push({ bodyStart: pos, bodyEnd: pos + bodySize });
          pos += bodySize;
        }
        break;
      }
      default: pos = secEnd;
    }
  }

  const funcImports = imports.filter(i => i.kind === 0);
  let mainIdx = -1;
  for (const ex of exports) {
    if ((ex.name === '_start' || ex.name === 'main') && ex.kind === 0) mainIdx = ex.idx;
  }
  if (mainIdx < 0 && startFunc >= 0) mainIdx = startFunc;

  return { types, imports: funcImports, funcTypeIdxs, codes, exports, startFunc, mainIdx };
}

// ── Opcode decode ─────────────────────────────────────────────
function decodeOps(code, buf) {
  const u8 = new Uint8Array(buf);
  let pos = code.bodyStart;

  const leb = () => { let v = 0, s = 0, b; do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80); return v; };
  const lebS = () => {
    let v = 0, s = 0, b;
    do { b = u8[pos++]; v |= (b & 0x7f) << s; s += 7; } while (b & 0x80);
    if (s < 32 && (b & 0x40)) v |= -(1 << s);
    return v;
  };

  const nc = leb();
  let localCount = 0;
  for (let i = 0; i < nc; i++) { const c = leb(); leb(); localCount += c; }

  const ops = [];
  while (pos < code.bodyEnd) {
    const opcode = u8[pos++];
    let imm0 = 0, imm1 = 0;
    switch (opcode) {
      case 0x00: case 0x01: case 0x05: case 0x0B: case 0x0F: case 0x1A: case 0x1B: case 0x1C: break;
      case 0x02: case 0x03: case 0x04: imm0 = lebS(); break;
      case 0x0C: case 0x0D: imm0 = leb(); break;
      case 0x0E: imm0 = leb(); imm1 = leb(); break;
      case 0x10: imm0 = leb(); break;
      case 0x11: imm0 = leb(); leb(); break;
      case 0x20: case 0x21: case 0x22: case 0x23: case 0x24: imm0 = leb(); break;
      case 0x25: case 0x26: imm0 = leb(); break;
      case 0x28: case 0x29: case 0x2A: case 0x2B: case 0x2C: case 0x2D: case 0x2E: case 0x2F:
      case 0x30: case 0x31: case 0x32: case 0x33: case 0x34: case 0x35:
      case 0x36: case 0x37: case 0x38: case 0x39: case 0x3A: case 0x3B: case 0x3C: case 0x3D: case 0x3E:
        leb(); imm0 = leb(); break;
      case 0x3F: case 0x40: break;
      case 0x41: imm0 = lebS(); break;
      case 0x42: imm0 = lebS(); imm1 = lebS(); break;
      case 0x43: imm0 = new DataView(buf.buffer, buf.byteOffset, buf.byteLength).getUint32(pos, true); pos += 4; break;
      case 0x44: {
        const dv2 = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
        imm0 = dv2.getUint32(pos, true); imm1 = dv2.getUint32(pos + 4, true); pos += 8; break;
      }
      case 0xFC: case 0xFD: leb(); break;
      default: break;
    }
    ops.push({ opcode, imm0, imm1 });
  }
  return { ops, localCount };
}

// ── Populate JIT buffers ──────────────────────────────────────
const SYSCALL_MAP = {
  fd_write: 20, fd_read: 0, fd_close: 3, fd_seek: 8,
  proc_exit: 60, environ_sizes_get: 61, environ_get: 62,
  args_sizes_get: 63, args_get: 64,
};

function populateJIT(buf, parsed) {
  const e = inst().exports;
  const u8 = mem();
  const v = dv();
  const { types, imports, funcTypeIdxs, codes, exports, startFunc, mainIdx } = parsed;

  // Types
  v.setUint32(e.OFF_TYPE_COUNT, types.length, true);
  for (let i = 0; i < types.length; i++) {
    const t = types[i];
    const base = e.OFF_TYPES_BUF + i * e.SZ_TYPE;
    for (let j = 0; j < t.params.length && j < 128; j++) u8[base + j] = t.params[j];
    v.setUint16(base + 128, t.params.length, true);
    for (let j = 0; j < t.results.length && j < 128; j++) u8[base + 132 + j] = t.results[j];
    v.setUint16(base + 136, t.results.length, true);
  }

  // Imports count
  const importCount = imports.length;
  v.setUint32(e.OFF_IMPORT_COUNT, importCount, true);

  // Functions
  const numCodeFuncs = funcTypeIdxs.length;
  const totalFuncs = importCount + numCodeFuncs;
  v.setUint32(e.OFF_FUNC_COUNT, totalFuncs, true);

  for (let i = 0; i < importCount; i++) {
    const base = e.OFF_FUNCTIONS_BUF + i * e.SZ_FUNC;
    v.setUint32(base, imports[i].typeIdx, true);
    const sysno = (() => {
      const key = imports[i].fnName.replace(/^[a-z]+_/, '');
      if (SYSCALL_MAP[imports[i].fnName] !== undefined) return SYSCALL_MAP[imports[i].fnName];
      if (SYSCALL_MAP[key] !== undefined) return SYSCALL_MAP[key];
      return -1;
    })();
    v.setUint32(base + 12, Math.max(sysno, 0), true);
  }
  for (let i = 0; i < numCodeFuncs; i++) {
    const base = e.OFF_FUNCTIONS_BUF + (importCount + i) * e.SZ_FUNC;
    v.setUint32(base, funcTypeIdxs[i], true);
  }

  // Code records + decoded ops
  v.setUint32(e.OFF_CODE_COUNT, codes.length, true);
  let decodedOpIdx = 0;

  for (let ci = 0; ci < codes.length; ci++) {
    const code = codes[ci];
    const { ops, localCount } = decodeOps(code, buf);
    const base = e.OFF_CODE_BUF + ci * e.SZ_CODE;
    v.setUint32(base, code.bodyStart, true);
    v.setUint32(base + 8, code.bodyEnd - code.bodyStart, true);
    v.setUint32(base + 16, localCount, true);
    v.setUint32(base + 24, decodedOpIdx, true);
    v.setUint32(base + 32, ops.length, true);

    for (let i = 0; i < ops.length; i++) {
      const opBase = e.OFF_DECODED_OPS + decodedOpIdx * e.DEC_SZ;
      const op = ops[i];
      v.setUint32(opBase, op.opcode, true);
      v.setUint32(opBase + 4, op.imm0, true);
      v.setUint32(opBase + 8, op.imm1, true);

      if (op.opcode === 0x10) {
        const realFuncIdx = op.imm0;
        if (realFuncIdx < importCount) {
          const sysno = imports[realFuncIdx]?.fnName ? (() => {
            const key = imports[realFuncIdx].fnName.replace(/^[a-z]+_/, '');
            if (SYSCALL_MAP[imports[realFuncIdx].fnName] !== undefined) return SYSCALL_MAP[imports[realFuncIdx].fnName];
            if (SYSCALL_MAP[key] !== undefined) return SYSCALL_MAP[key];
            return 0;
          })() : 0;
          v.setUint32(opBase + 20, sysno, true);
        }
      }
      decodedOpIdx++;
    }
    // terminator
    v.setUint32(e.OFF_DECODED_OPS + decodedOpIdx * e.DEC_SZ, 0, true);
  }

  // Start function
  const startIdx = mainIdx >= 0 ? mainIdx : (importCount > 0 ? importCount : 0);
  v.setUint32(e.OFF_START_FUNC, startIdx, true);

  // Syscall map
  for (let i = 0; i < importCount; i++) {
    const key = imports[i].fnName.replace(/^[a-z]+_/, '');
    let sysno = -1;
    if (SYSCALL_MAP[imports[i].fnName] !== undefined) sysno = SYSCALL_MAP[imports[i].fnName];
    else if (SYSCALL_MAP[key] !== undefined) sysno = SYSCALL_MAP[key];
    v.setUint32(e.OFF_SYSCALL_MAP + i * 4, sysno >= 0 ? sysno : 0, true);
  }

  return { importCount, totalFuncs, decodedOpCount: decodedOpIdx };
}

// ── Reset JIT state ───────────────────────────────────────────
function resetJIT() {
  const e = _inst.exports;
  const JIT_SCRATCH = e.JIT_SCRATCH_BASE;

  const v = dv();
  const u8 = mem();
  v.setUint32(JIT_SCRATCH + 0, 0, true);     // JS_CODE_PTR
  v.setUint32(JIT_SCRATCH + 28, 0, true);    // JS_LABEL_DEPTH
  v.setUint32(JIT_SCRATCH + 32, 0, true);    // JS_RETURN_EMITTED
  u8.fill(0, JIT_SCRATCH + 64, JIT_SCRATCH + 1088);        // JS_LABEL_OFFSETS
  u8.fill(0, JIT_SCRATCH + 1088, JIT_SCRATCH + 1344);      // JS_LABEL_KINDS
  u8.fill(0, JIT_SCRATCH + 1344, JIT_SCRATCH + 2368);      // JS_LABEL_IF_JZ
  v.setUint32(JIT_SCRATCH + 2368, 0, true);  // JS_FIXUP_COUNT
  u8.fill(0, JIT_SCRATCH + 2372, JIT_SCRATCH + 3396);      // JS_FIXUP_LABEL
  u8.fill(0, JIT_SCRATCH + 3396, JIT_SCRATCH + 4420);      // JS_FIXUP_OFFSET
  v.setUint32(JIT_SCRATCH + 4424, 0, true);  // JS_CALL_FIXUP_COUNT
}

// ── Public API ────────────────────────────────────────────────

export function jitCompile(wasmBytes) {
  inst();
  const buf = new Uint8Array(wasmBytes);

  const parsed = parseSections(buf);
  populateJIT(buf, parsed);
  resetJIT();

  const e = _inst.exports;
  const [elfAddr, elfSize] = e.compile_all_to_elf_x86_64();
  if (elfSize <= 0 || elfSize > 100_000_000) throw Error(`bad elf size: ${elfSize}`);

  return new Uint8Array(_mem.buffer, elfAddr, elfSize);
}

export function jitCompileTo(outPath, wasmBytes) {
  const elf = jitCompile(wasmBytes);
  writeFileSync(outPath, elf);
  return elf;
}

export function decodeWasm(wasmBytes) {
  const buf = new Uint8Array(wasmBytes);
  return parseSections(buf);
}
