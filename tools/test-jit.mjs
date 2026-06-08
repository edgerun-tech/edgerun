#!/usr/bin/env node
import { jitCompile, decodeWasm } from '../compiler/jit.mjs';
import { readFileSync, writeFileSync, existsSync } from 'fs';

function leb(v) { const b = []; do { let byte = v & 0x7f; v >>>= 7; if (v) byte |= 0x80; b.push(byte); } while (v); return b; }
function lebS(v) { const b = []; let m = 1; while (m) { let byte = v & 0x7f; v >>= 7; if ((v === 0 && !(byte & 0x40)) || (v === -1 && (byte & 0x40))) m = 0; else byte |= 0x80; b.push(byte); } return b; }

function buildWasm(funcs) {
  const tSection = [];
  tSection.push(...leb(funcs.length));
  for (const f of funcs) {
    tSection.push(0x60, ...leb(f.params.length));
    for (const p of f.params) tSection.push(p);
    tSection.push(...leb(f.results.length));
    for (const r of f.results) tSection.push(r);
  }
  const s1 = [1, ...leb(tSection.length), ...tSection];
  const fBody = [...leb(funcs.length)];
  for (let i = 0; i < funcs.length; i++) fBody.push(...leb(i));
  const s3 = [3, ...leb(fBody.length), ...fBody];
  const expSection = [...leb(1)];
  const expName = 'main';
  expSection.push(...leb(expName.length));
  for (let i = 0; i < expName.length; i++) expSection.push(expName.charCodeAt(i));
  expSection.push(0x00, ...leb(0));
  const s7 = [7, ...leb(expSection.length), ...expSection];
  const bodies = [];
  for (const f of funcs) {
    const body = [];
    const g = {}; for (const t of f.locals) g[t] = (g[t] || 0) + 1;
    body.push(...leb(Object.keys(g).length));
    for (const [t, c] of Object.entries(g)) body.push(...leb(c), +t);
    for (const b of f.body) { if (b instanceof Array) body.push(...b); else body.push(b); }
    bodies.push([...leb(body.length), ...body]);
  }
  const cSection = [...leb(bodies.length)];
  for (const b of bodies) cSection.push(...b);
  const s10 = [10, ...leb(cSection.length), ...cSection];
  return new Uint8Array([0, 0x61, 0x73, 0x6D, 1, 0, 0, 0, ...s1, ...s3, ...s7, ...s10]);
}

function hex(buf) { return [...buf].map(b => b.toString(16).padStart(2, '0')).join(' '); }

function test(desc, func) {
  try {
    const wasm = buildWasm([func]);
    const elf = jitCompile(wasm);
    const ok = elf[0] === 0x7f && elf[1] === 0x45 && elf[2] === 0x4c && elf[3] === 0x46;
    const textOff = 256;
    const hasCode = elf.length > textOff + 4 && elf.slice(textOff, textOff + 4).some(b => b !== 0);
    console.log(`${ok && hasCode ? '✓' : '✗'} ${desc}: ${elf.length}b ELF${hasCode ? '' : ' (EMPTY!)'}`);
    if (!ok) console.log(`  bad magic: ${hex(elf.slice(0, 4))}`);
    if (hasCode) {
      const text = elf.slice(textOff, Math.min(elf.length, textOff + 32));
      console.log(`  text: ${hex(text)}`);
    }
    return ok && hasCode;
  } catch(e) {
    console.log(`✗ ${desc}: ${e.message}`);
    return false;
  }
}

console.log('── Testing JIT via edgerun.wasm ──\n');

let allOk = true;

allOk &= test('i32.const 42; return', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(42), 0x0B],
});

allOk &= test('i32.const 1; i32.const 2; i32.add', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(1), 0x41, ...lebS(2), 0x6A, 0x0B],
});

allOk &= test('i64.const 1000; i64.const 2000; i64.add', {
  params: [], results: [0x7E], locals: [],
  body: [0x42, ...lebS(1000), ...lebS(0), 0x42, ...lebS(2000), ...lebS(0), 0x7C, 0x0B],
});

allOk &= test('local.get 0; local.get 1 (params)', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B],
});

allOk &= test('local.set after local.get', {
  params: [0x7F], results: [0x7F], locals: [0x7F],
  body: [0x20, 0x00, 0x21, 0x01, 0x20, 0x00, 0x0B],
});

allOk &= test('i32.const 0; i32.const 1; i32.eq', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(0), 0x41, ...lebS(1), 0x46, 0x0B],
});

allOk &= test('local.get 0; local.get 1; i32.sub', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x6B, 0x0B],
});

allOk &= test('local.get 0; local.get 1; i32.mul', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x6C, 0x0B],
});

allOk &= test('local.get 0; local.get 1; i32.and', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x71, 0x0B],
});

allOk &= test('local.get 0; local.get 1; i32.or', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x72, 0x0B],
});

allOk &= test('local.get 0; local.get 1; i32.shl', {
  params: [0x7F, 0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x20, 0x01, 0x74, 0x0B],
});

allOk &= test('i32.const 10; i32.const 3; i32.div_s', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(10), 0x41, ...lebS(3), 0x6D, 0x0B],
});

allOk &= test('i32.const 10; i32.const 3; i32.rem_u', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(10), 0x41, ...lebS(3), 0x70, 0x0B],
});

allOk &= test('drop: i32.const 7; drop; i32.const 42; return', {
  params: [], results: [0x7F], locals: [],
  body: [0x41, ...lebS(7), 0x1A, 0x41, ...lebS(42), 0x0B],
});

allOk &= test('i32.eqz on local.get 0', {
  params: [0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x45, 0x0B],
});

allOk &= test('block + br', {
  params: [], results: [], locals: [],
  body: [0x02, 0x40, 0x0C, 0x00, 0x0B, 0x0B],
});

allOk &= test('if/else', {
  params: [0x7F], results: [0x7F], locals: [],
  body: [0x20, 0x00, 0x04, 0x7F, 0x41, ...lebS(1), 0x05, 0x41, ...lebS(2), 0x0B, 0x0B],
});

allOk &= test('local.tee', {
  params: [0x7F], results: [0x7F], locals: [0x7F],
  body: [0x20, 0x00, 0x22, 0x01, 0x0B],
});

allOk &= test('Local with locals (not params)', {
  params: [], results: [0x7F], locals: [0x7F, 0x7F],
  body: [0x41, ...lebS(10), 0x21, 0x00, 0x41, ...lebS(20), 0x21, 0x01, 0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B],
});

console.log(`\n${allOk ? 'All passed' : 'Some failed'}`);

// Test with a real WASM file
console.log('\n── Real file test ──');
for (const path of ['/tmp/test.wasm']) {
  try {
    const bytes = readFileSync(path);
    const elf = jitCompile(bytes);
    const textOff = 256;
    const hasCode = elf.length > textOff + 4;
    console.log(`✓ ${path}: ${elf.length}b ELF${hasCode ? ', has code section' : ', NO code'}`);
  } catch(e) {}
}
