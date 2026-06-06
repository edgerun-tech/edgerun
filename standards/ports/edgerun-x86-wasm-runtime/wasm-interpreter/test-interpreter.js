const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const INTERP_WAT = path.join(__dirname, 'interpreter.wat');

execSync(`wat2wasm "${INTERP_WAT}" -o /tmp/interpreter.wasm`, { stdio: 'inherit' });

// Simple module: one function returning i32.const 42
const testModules = [
  {
    name: 'i32.const 42',
    wat: `(module
      (type (func (result i32)))
      (func (type 0) (result i32) i32.const 42)
      (export "main" (func 0)))`,
    expect: { types: 1, funcs: 1, codes: 1, exports: 1, ops: 2 }
  },
  {
    name: 'local.get 0',
    wat: `(module
      (type (func (param i32) (result i32)))
      (func (type 0) (param i32) (result i32) local.get 0)
      (export "main" (func 0)))`,
    expect: { types: 1, funcs: 1, codes: 1, exports: 1, ops: 2 }
  },
  {
    name: 'full test',
    wat: `(module
      (type (func (result i32)))
      (type (func (param i32) (result i32)))
      (type (func))
      (func (type 0) (result i32) i32.const 42)
      (func (type 1) (param i32) (result i32) local.get 0)
      (func (type 2))
      (memory 1)
      (export "main" (func 0))
      (start 2)
      (data (i32.const 0) "hello"))`,
    expect: { types: 3, funcs: 3, codes: 3, exports: 1, ops: [2, 2, 1] }
  }
];

async function main() {
  const interpWasm = fs.readFileSync('/tmp/interpreter.wasm');
  const mod = await WebAssembly.compile(interpWasm);
  const inst = await WebAssembly.instantiate(mod);
  const mem = inst.exports.memory;
  const loadFn = inst.exports.load;

  const guestBase = 0x100000;
  const guestMemBase = 0x200000;

  function loadAndCall(wat, funcIdx, args) {
    fs.writeFileSync('/tmp/t.wat', wat);
    execSync('wat2wasm /tmp/t.wat -o /tmp/t.wasm', { stdio: 'pipe' });
    const wasm = fs.readFileSync('/tmp/t.wasm');
    const memEnd = Math.max(guestBase + wasm.length + 0x10000, guestMemBase + 0x30000);
    const neededPages = Math.ceil(memEnd / 65536);
    while (mem.buffer.byteLength / 65536 < neededPages) mem.grow(1);
    const view = new Uint8Array(mem.buffer);
    view.set(new Uint8Array(wasm), guestBase);
    const loadErr = loadFn(guestBase, wasm.length);
    if (loadErr !== 0) return { error: loadErr };
    // Write args into memory at scratch space (guestBase - 256)
    const i32 = new Int32Array(mem.buffer);
    const argsBase = guestBase - 256;
    for (let i = 0; i < args.length; i++) i32[argsBase/4 + i] = args[i];
    const callErr = inst.exports.call(funcIdx, argsBase, args.length);
    if (callErr !== 0) return { error: callErr };
    return { value: inst.exports.get_result_value(0), count: inst.exports.get_result_count() };
  }

  // Execution tests
  const execTests = [
    { name: 'i32.const 42', wat: testModules[0].wat, funcIdx: 0, args: [], expect: 42 },
    { name: 'local.get 0 (arg=7)', wat: testModules[1].wat, funcIdx: 0, args: [7], expect: 7 },
    { name: 'i32.add 3+5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.add) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 8 },
    { name: 'i32.sub 10-3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.sub) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 7 },
    { name: 'i32.mul 6*7', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.mul) (export "main" (func 0)))`, funcIdx: 0, args: [6, 7], expect: 42 },
    { name: 'i32.eq 5==5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.eq) (export "main" (func 0)))`, funcIdx: 0, args: [5, 5], expect: 1 },
    { name: 'i32.eq 5!=7', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.eq) (export "main" (func 0)))`, funcIdx: 0, args: [5, 7], expect: 0 },
    { name: 'i32.ne 5!=7', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.ne) (export "main" (func 0)))`, funcIdx: 0, args: [5, 7], expect: 1 },
    { name: 'i32.eqz 0', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.eqz) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 1 },
    { name: 'i32.eqz 42', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.eqz) (export "main" (func 0)))`, funcIdx: 0, args: [42], expect: 0 },
    // Block
    { name: 'block result', wat: `(module (type (func (result i32))) (func (type 0) (result i32) (block (result i32) i32.const 42)) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // If/else
    { name: 'if true', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 (if (result i32) (then (i32.const 1)) (else (i32.const 2)))) (export "main" (func 0)))`, funcIdx: 0, args: [5], expect: 1 },
    { name: 'if false', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 (if (result i32) (then (i32.const 1)) (else (i32.const 2)))) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 2 },
    // Br (skip code after br but before end)
    { name: 'br skip', wat: `(module (type (func (result i32))) (func (type 0) (result i32) (block (result i32) i32.const 1 (br 0) i32.const 99)) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1 },
    // Br_if: result value first, then condition; no branch → drop result, use fallback
    { name: 'br_if fallthrough', wat: `(module (type (func (result i32))) (func (type 0) (result i32) (block (result i32) i32.const 42 i32.const 0 (br_if 0) drop i32.const 99)) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 99 },
    // Local set/tee
    { name: 'local.set + local.get', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.const 5 i32.add local.set 0 local.get 0) (export "main" (func 0)))`, funcIdx: 0, args: [10], expect: 15 },
    { name: 'local.tee', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.const 3 i32.add local.tee 0) (export "main" (func 0)))`, funcIdx: 0, args: [7], expect: 10 },
    // Call
    { name: 'call', wat: `(module (type (func (result i32))) (type (func (param i32) (result i32))) (func (type 0) (result i32) i32.const 7 call 1 i32.const 2 i32.add) (func (type 1) (param i32) (result i32) local.get 0 i32.const 3 i32.mul) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 23 },
    // Memory
    { name: 'i32.store + i32.load', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) (i32.store (i32.const 0) (i32.const 42)) (i32.load (i32.const 0))) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // Data segment init
    { name: 'data init (load from data seg)', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) (i32.load (i32.const 0))) (export "main" (func 0)) (data (i32.const 0) "*"))`, funcIdx: 0, args: [], expect: 42 },
    // Memory size
    { name: 'memory.size 1 page', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) memory.size) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1 },
    { name: 'memory.size 2 pages', wat: `(module (memory 2) (type (func (result i32))) (func (type 0) (result i32) memory.size) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 2 },
    // Memory grow
    { name: 'memory.grow 0 (returns old size)', wat: `(module (memory 1) (func (export "main") (result i32) i32.const 0 memory.grow))`, funcIdx: 0, args: [], expect: 1 },
    { name: 'memory.grow +1 (then size=2)', wat: `(module (memory 1) (func (export "main") (result i32) i32.const 1 memory.grow drop memory.size))`, funcIdx: 0, args: [], expect: 2 },
    // Bitwise ops
    { name: 'i32.and 5&3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.and) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 1 },
    { name: 'i32.or 1|2', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.or) (export "main" (func 0)))`, funcIdx: 0, args: [1, 2], expect: 3 },
    { name: 'i32.xor 1^3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.xor) (export "main" (func 0)))`, funcIdx: 0, args: [1, 3], expect: 2 },
    { name: 'i32.shl 1<<3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.shl) (export "main" (func 0)))`, funcIdx: 0, args: [1, 3], expect: 8 },
    { name: 'i32.shr_u 8>>1', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.shr_u) (export "main" (func 0)))`, funcIdx: 0, args: [8, 1], expect: 4 },
    { name: 'i32.shr_s -8>>1', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.shr_s) (export "main" (func 0)))`, funcIdx: 0, args: [-8, 1], expect: -4 },
    // Div/rem
    { name: 'i32.div_s 10/3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.div_s) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 3 },
    { name: 'i32.div_s -10/3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.div_s) (export "main" (func 0)))`, funcIdx: 0, args: [-10, 3], expect: -3 },
    { name: 'i32.div_u 10/3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.div_u) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 3 },
    { name: 'i32.rem_s 10%3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.rem_s) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 1 },
    { name: 'i32.rem_s -10%3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.rem_s) (export "main" (func 0)))`, funcIdx: 0, args: [-10, 3], expect: -1 },
    { name: 'i32.rem_u 10%3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.rem_u) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 1 },
    // Loop
    { name: 'loop count 0 to 5', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) block loop local.get 0 i32.const 5 i32.ge_s br_if 1 local.get 0 i32.const 1 i32.add local.set 0 br 0 end end local.get 0) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 5 },
    // Return
    { name: 'return mid-function', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i32.const 42 return i32.const 0) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // Comparison ops
    { name: 'i32.lt_s 3<5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.lt_s) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 1 },
    { name: 'i32.lt_s 5<3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.lt_s) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 0 },
    { name: 'i32.lt_u 1<2', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.lt_u) (export "main" (func 0)))`, funcIdx: 0, args: [1, 2], expect: 1 },
    { name: 'i32.gt_s 5>3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.gt_s) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 1 },
    { name: 'i32.gt_u 3>5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.gt_u) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 0 },
    { name: 'i32.le_s 3<=5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.le_s) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 1 },
    { name: 'i32.le_s 5<=5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.le_s) (export "main" (func 0)))`, funcIdx: 0, args: [5, 5], expect: 1 },
    { name: 'i32.ge_s 5>=3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.ge_s) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 1 },
    { name: 'i32.ge_s 3>=5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.ge_s) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 0 },
  ];

  console.log('\n══════════ Execution Tests ══════════');
  let execPassed = 0, execFailed = 0;
  for (const et of execTests) {
    const r = loadAndCall(et.wat, et.funcIdx, et.args);
    const ok = !r.error && r.count === 1 && Number(r.value) === et.expect;
    const info = r.error ? `err=${r.error}` : `got ${Number(r.value)}`;
    console.log(`  ${ok ? 'PASS' : 'FAIL'} ${et.name}: ${info} (expected ${et.expect})`);
    if (ok) execPassed++; else execFailed++;
  }
  console.log(`  --- ${execPassed}/${execTests.length} passed ---`);

  for (const t of testModules) {
    // Compile WAT to WASM
    fs.writeFileSync('/tmp/t.wat', t.wat);
    execSync('wat2wasm /tmp/t.wat -o /tmp/t.wasm', { stdio: 'pipe' });
    const wasm = fs.readFileSync('/tmp/t.wasm');

    // Ensure memory is big enough (guest binary + guest linear memory)
    const memEnd = Math.max(guestBase + wasm.length + 0x10000, guestMemBase + 0x30000);
    const neededPages = Math.ceil(memEnd / 65536);
    while (mem.buffer.byteLength / 65536 < neededPages) mem.grow(1);

    const view = new Uint8Array(mem.buffer);
    view.set(new Uint8Array(wasm), guestBase);

    const err = loadFn(guestBase, wasm.length);
    const i32 = new Int32Array(mem.buffer);

    console.log(`\n--- ${t.name} ---`);
    console.log('Load:', err === 0 ? 'OK' : 'FAIL(' + err + ')');
    if (err !== 0) continue;

    const tc = i32[256/4];
    const fc = i32[17680/4];
    const cc = i32[21784/4];
    const ec = i32[38176/4];

    console.log(`Types: ${tc} (expected ${t.expect.types}) ${tc === t.expect.types ? 'OK' : 'FAIL'}`);
    console.log(`Funcs: ${fc} (expected ${t.expect.funcs}) ${fc === t.expect.funcs ? 'OK' : 'FAIL'}`);
    console.log(`Codes: ${cc} (expected ${t.expect.codes}) ${cc === t.expect.codes ? 'OK' : 'FAIL'}`);
    console.log(`Expts: ${ec} (expected ${t.expect.exports}) ${ec === t.expect.exports ? 'OK' : 'FAIL'}`);

    // Check decoded ops
    const decodedCount = i32[89864/4];
    if (Array.isArray(t.expect.ops)) {
      const totalOps = t.expect.ops.reduce((a,b) => a+b, 0);
      console.log(`Decoded ops: ${decodedCount} (expected ~${totalOps})`);
      // Check per-function
      for (let fi = 0; fi < t.expect.ops.length; fi++) {
        const codeBase = 21792 + fi * 64;
        const ds = i32[(codeBase + 24)/4];
        const dn = i32[(codeBase + 32)/4];
        console.log(`  Func ${fi}: decoded_start=${ds}, count=${dn} (expected ~${t.expect.ops[fi]})`);

        // Print decoded ops for first function
        if (fi === 0) {
          for (let oi = 0; oi < dn && oi < 5; oi++) {
            const opBase = 0xA0000 + (ds + oi) * 16;
            const op = new Uint8Array(mem.buffer)[opBase];
            const imm0 = i32[(opBase + 4)/4];
            const imm1 = i32[(opBase + 8)/4];
            const opNames = {
              0x00: 'unreachable', 0x01: 'nop', 0x02: 'block', 0x03: 'loop',
              0x04: 'if', 0x05: 'else', 0x0B: 'end', 0x0C: 'br', 0x0D: 'br_if',
              0x0F: 'return', 0x10: 'call', 0x1A: 'drop', 0x20: 'local.get',
              0x21: 'local.set', 0x22: 'local.tee', 0x41: 'i32.const',
              0x6A: 'i32.add', 0x6B: 'i32.sub', 0x6C: 'i32.mul',
              0x46: 'i32.eq', 0x47: 'i32.ne', 0x45: 'i32.eqz',
            };
            const name = opNames[op] || ('0x' + op.toString(16));
            console.log(`    [${oi}] ${name} imm0=${imm0} imm1=${imm1}`);
          }
        }
      }
    } else {
      console.log(`Decoded ops: ${decodedCount} (expected ~${t.expect.ops})`);
    }

    // Verify data init for full test module
    if (t.wat.includes('(data ')) {
      const u8 = new Uint8Array(mem.buffer);
      // First byte of "hello" at guest mem base + 0 should be 0x68 ('h')
      const h = u8[guestMemBase];
      console.log(`Data init: first byte = 0x${h.toString(16)} (expected 0x68) ${h === 0x68 ? 'OK' : 'FAIL'}`);
    }
  }
}

main().catch(e => console.error('FAIL:', e));
