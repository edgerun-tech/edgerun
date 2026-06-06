// Benchmark: compiled native (ELF) vs interpreter vs V8 WASM
const fs = require('fs');
const { spawnSync } = require('child_process');
const ITER = 5;

// Build a WASM module: (func (result i32) ...locals... body...)
// body must end with a value on the stack
function wasmModule(locals, body, memPages) {
  const funcBody = locals > 0
    ? [0x01, ...uleb(locals), 0x7F, ...body, 0x0B]
    : [0x00, ...body, 0x0B];
  const funcEntry = [...uleb(funcBody.length), ...funcBody];
  const type = [0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F]; // () -> i32
  const funcSec = [0x03, 0x02, 0x01, 0x00];
  const codePayload = [0x01, ...funcEntry];
  const code = [0x0A, ...uleb(codePayload.length), ...codePayload];
  const exportSec = [0x07, 0x05, 0x01, 0x01, 0x66, 0x00, 0x00];
  const memLimits = uleb(memPages);
  const memSec = memPages ? [0x05, 2 + memLimits.length, 0x01, 0x00, ...memLimits] : [];
  // Sections must be in order: Type(1), Function(3), Memory(5), Export(7), Code(10)
  return new Uint8Array([0x00,0x61,0x73,0x6D,0x01,0x00,0x00,0x00, ...type, ...funcSec, ...memSec, ...exportSec, ...code]);
}

// LEB128 encode a u32 (unsigned, for section sizes and counts)
function uleb(v) {
  const r = [];
  do { let b = v & 0x7F; v >>>= 7; if (v) b |= 0x80; r.push(b); } while (v);
  return r;
}

// Signed LEB128 encode an i32 (for i32.const / i64.const immediates)
function sleb(v) {
  if (v === 0) return [0];
  const r = [];
  const neg = v < 0;
  let more = 1;
  while (more) {
    let b = v & 0x7F;
    v >>= 7;
    if ((v === 0 && !(b & 0x40)) || (v === -1 && (b & 0x40))) { more = 0; }
    else { b |= 0x80; }
    r.push(b);
  }
  return r;
}

// Build sum_sq: sum i*i for i in 0..N-1
function makeSumSq(N) {
  const lebN = sleb(N);
  const L = [0, 1]; // i, sum
  const body = [
    0x02, 0x40,  // block (void)
    0x03, 0x40,  // loop (void)
    // br_if 1 = exit block (label 0=loop, label 1=outer block)
    0x20, L[0], 0x41, ...lebN, 0x46, 0x0D, 0x01,
    0x20, L[1], 0x20, L[0], 0x20, L[0], 0x6C, 0x6A, 0x21, L[1],
    0x20, L[0], 0x41, 0x01, 0x6A, 0x21, L[0],
    0x0C, 0x00, 0x0B, 0x0B,
    0x20, L[1],
  ];
  return wasmModule(2, body);
}

// Build sum_linear: sum i for i in 0..N-1
function makeSumLin(N) {
  const lebN = sleb(N);
  const L = [0, 1];
  const body = [
    0x02, 0x40, 0x03, 0x40,
    0x20, L[0], 0x41, ...lebN, 0x46, 0x0D, 0x01,
    0x20, L[1], 0x20, L[0], 0x6A, 0x21, L[1],
    0x20, L[0], 0x41, 0x01, 0x6A, 0x21, L[0],
    0x0C, 0x00, 0x0B, 0x0B,
    0x20, L[1],
  ];
  return wasmModule(2, body);
}

// Build mult_loop: i*3 for i in 0..N-1
function makeMul3(N) {
  const lebN = sleb(N);
  const L = [0, 1];
  const body = [
    0x02, 0x40, 0x03, 0x40,
    0x20, L[0], 0x41, ...lebN, 0x46, 0x0D, 0x01,
    0x20, L[1], 0x20, L[0], 0x41, 0x03, 0x6C, 0x6A, 0x21, L[1],
    0x20, L[0], 0x41, 0x01, 0x6A, 0x21, L[0],
    0x0C, 0x00, 0x0B, 0x0B,
    0x20, L[1],
  ];
  return wasmModule(2, body);
}

// Build mem_test: write to and read from linear memory
function makeMemTest(N) {
  // Write values 0..N-1 to memory, sum them back
  const lebN = sleb(N);
  const body = [
    0x41, 0x00, 0x21, 0x01,  // i = 0
    0x02, 0x40,  0x03, 0x40,
    0x20, 0x01, 0x41, ...lebN, 0x46, 0x0D, 0x01,
    0x20, 0x01, 0x20, 0x01, 0x36, 0x02, 0x00,
    0x20, 0x01, 0x41, 0x01, 0x6A, 0x21, 0x01,
    0x0C, 0x00, 0x0B, 0x0B,
    0x41, 0x00, 0x21, 0x01,  // i = 0
    0x02, 0x40,  0x03, 0x40,
    0x20, 0x01, 0x41, ...lebN, 0x46, 0x0D, 0x01,
    0x20, 0x02, 0x20, 0x01, 0x28, 0x02, 0x00, 0x6A, 0x21, 0x02,
    0x20, 0x01, 0x41, 0x01, 0x6A, 0x21, 0x01,
    0x0C, 0x00, 0x0B, 0x0B,
    0x20, 0x02,
  ];
  return wasmModule(3, body, 64);
}

async function benchNative(wasm, name) {
  const interp = (await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {})).instance.exports;
  interp.memory.grow(0x4000000 / 0x10000 - interp.memory.buffer.byteLength / 0x10000);
  const comp = (await WebAssembly.instantiate(fs.readFileSync('compiler.wasm'), { env: { memory: interp.memory } })).instance.exports;
  const mem = new Uint8Array(interp.memory.buffer);
  
  for (let i = 0; i < wasm.length; i++) mem[0x200000 + i] = wasm[i];
  const lerr = interp.load(0x200000, wasm.length);
  if (lerr) { console.log(`  Native: load error ${lerr}`); return null; }
  
  comp.jit_compile(0);
  
  const [elfAddr, elfSize] = comp.compile_to_elf(0);
  const elf = new Uint8Array(mem.slice(elfAddr, elfAddr + elfSize));
  const path = `/tmp/bench_${name}`;
  fs.writeFileSync(path, elf);
  fs.chmodSync(path, 0o755);
  
  let result;
  for (let i = 0; i < 3; i++) { const r = spawnSync(path, { stdio: 'ignore' }); result = r.status; }
  
  const times = [];
  for (let i = 0; i < ITER; i++) {
    const start = process.hrtime.bigint();
    const r = spawnSync(path, { stdio: 'ignore' });
    const end = process.hrtime.bigint();
    if (r.status === null) { console.log('  Native: process error'); return null; }
    times.push(Number(end - start) / 1e6);
  }
  
  fs.unlinkSync(path);
  const avg = times.reduce((a,b)=>a+b,0) / times.length;
  return { times, label: 'native', avg, result };
}

async function benchInterp(wasm) {
  const interp = (await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {})).instance.exports;
  interp.memory.grow(0x40000000 / 0x10000 - interp.memory.buffer.byteLength / 0x10000);
  const mem = new Uint8Array(interp.memory.buffer);
  for (let i = 0; i < wasm.length; i++) mem[0x200000 + i] = wasm[i];
  interp.load(0x200000, wasm.length);
  
  for (let i = 0; i < 3; i++) interp.call(0, 0, 0);
  
  const times = [];
  for (let i = 0; i < ITER; i++) {
    const start = process.hrtime.bigint();
    interp.call(0, 0, 0);
    const end = process.hrtime.bigint();
    times.push(Number(end - start) / 1e6);
  }
  const avg = times.reduce((a,b)=>a+b,0) / times.length;
  const result = interp.get_result_count() > 0 ? Number(interp.get_result_value(0)) : NaN;
  return { times, label: 'interp', avg, result };
}

async function benchV8(wasm) {
  const mod = await WebAssembly.instantiate(wasm, {});
  // Find first exported function
  let fn;
  for (const v of Object.values(mod.instance.exports)) { if (typeof v === 'function') { fn = v; break; } }
  if (!fn) throw new Error('no function export');
  
  for (let i = 0; i < 3; i++) fn();
  
  const times = [];
  for (let i = 0; i < ITER; i++) {
    const start = process.hrtime.bigint();
    fn();
    const end = process.hrtime.bigint();
    times.push(Number(end - start) / 1e6);
  }
  const avg = times.reduce((a,b)=>a+b,0) / times.length;
  return { times, label: 'v8', avg, result: Number(fn()) };
}

async function main() {
  console.log('Building test modules...');
  
  const tests = [
    { name: 'sum_lin_10M',  wasm: makeSumLin(10000000) },
    { name: 'sum_lin_1M',   wasm: makeSumLin(1000000) },
    { name: 'sum_sq_1M',    wasm: makeSumSq(1000000) },
    { name: 'sum_sq_100K',  wasm: makeSumSq(100000) },
    { name: 'mul3_10M',     wasm: makeMul3(10000000) },
    { name: 'mem_100K',     wasm: makeMemTest(100000) },
  ];
  
  console.log('Running benchmarks...\n');
  const results = [];
  
  for (const {name, wasm} of tests) {
    console.log(`--- ${name} ---`);
    
    // V8
    try {
      const r = await benchV8(wasm);
      r.test = name;
      results.push(r);
      console.log(`  V8:      ${r.avg.toFixed(2)} ms  (result: ${r.result})`);
    } catch(e) { console.log(`  V8:      FAIL - ${e.message}`); }
    
    // Interpreter (skip if >1M iterations — too slow)
    if (name.endsWith('_1M') || name.endsWith('_100K')) {
      try {
        const r = await benchInterp(wasm);
        r.test = name;
        results.push(r);
        console.log(`  Interp:  ${r.avg.toFixed(2)} ms  (result: ${r.result})`);
      } catch(e) { console.log(`  Interp:  FAIL - ${e.message}`); }
    } else {
      console.log('  Interp:  SKIP (too many iterations)');
    }
    
    // Native ELF (memory ops not yet supported in ELF output)
    if (name.startsWith('mem_')) {
      console.log('  Native:  SKIP (memory ops unsupported)');
    } else {
      try {
        const r = await benchNative(wasm, name);
        if (r) {
          r.test = name;
          results.push(r);
          console.log(`  Native:  ${r.avg.toFixed(2)} ms  (result: ${r.result})`);
        }
      } catch(e) { console.log(`  Native:  FAIL - ${e.message}`); }
    }
    
    console.log();
  }
  
  // Report
  if (results.length === 0) { console.log('No results to report.'); return; }
  
  const labels = [...new Set(results.map(r => r.label))];
  const testsRan = [...new Set(results.map(r => r.test))];
  
  console.log('=== RESULTS (ms, lower is better) ===\n');
  const hdr = 'Test'.padEnd(18) + labels.map(l => l.padEnd(14)).join('') + 'Speedup vs interp';
  console.log(hdr);
  console.log('-'.repeat(hdr.length));
  
  for (const t of testsRan) {
    const row = results.filter(r => r.test === t);
    const interpRow = row.find(r => r.label === 'interp');
    const interpAvg = interpRow ? interpRow.avg : NaN;
    process.stdout.write(t.padEnd(18));
    for (const lbl of labels) {
      const r = row.find(x => x.label === lbl);
      if (r) {
        const s = interpAvg && lbl !== 'interp' ? ` (${(interpAvg/r.avg).toFixed(1)}x)` : '';
        process.stdout.write(`${r.avg.toFixed(2)}${s}`.padEnd(14));
      } else {
        process.stdout.write('N/A'.padEnd(14));
      }
    }
    // Speedup vs interp for native
    if (interpRow) {
      const nativeRow = row.find(r => r.label === 'native');
      if (nativeRow) {
        process.stdout.write(`${(interpAvg/nativeRow.avg).toFixed(1)}x (native/${(interpAvg/(row.find(r=>r.label==='v8')?.avg||Infinity)).toFixed(1)}x v8)`);
      }
    }
    process.stdout.write('\n');
  }
}

main().catch(e => console.error(e));
