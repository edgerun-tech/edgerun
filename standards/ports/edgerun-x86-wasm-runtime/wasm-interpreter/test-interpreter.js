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
    const count = inst.exports.get_result_count();
    const values = [];
    for (let i = 0; i < count; i++) values.push(Number(inst.exports.get_result_value(i)));
    return { value: values[0] ?? 0, count, values };
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
    // i32.clz/ctz/popcnt
    { name: 'i32.clz 1 (leading zeros in i32=31)', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.clz) (export "main" (func 0)))`, funcIdx: 0, args: [1], expect: 31 },
    { name: 'i32.clz 0 = 32', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.clz) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 32 },
    { name: 'i32.ctz 8 (trailing zeros=3)', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.ctz) (export "main" (func 0)))`, funcIdx: 0, args: [8], expect: 3 },
    { name: 'i32.popcnt 7 = 3', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.popcnt) (export "main" (func 0)))`, funcIdx: 0, args: [7], expect: 3 },
    { name: 'i32.popcnt 0 = 0', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.popcnt) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 0 },
    // i32.rotl/rotr
    { name: 'i32.rotl 1<<3 (0x1 rol 3 = 0x8)', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.rotl) (export "main" (func 0)))`, funcIdx: 0, args: [1, 3], expect: 8 },
    { name: 'i32.rotr 8>>3 (0x8 ror 3 = 0x1)', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 local.get 1 i32.rotr) (export "main" (func 0)))`, funcIdx: 0, args: [8, 3], expect: 1 },
    // i32.extend8_s / i32.extend16_s
    { name: 'i32.extend8_s 0x80 -> -128', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.extend8_s) (export "main" (func 0)))`, funcIdx: 0, args: [0x80], expect: -128 },
    { name: 'i32.extend16_s 0x8000 -> -32768', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i32.extend16_s) (export "main" (func 0)))`, funcIdx: 0, args: [0x8000], expect: -32768 },
    // select
    { name: 'select true (pick first)', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i32.const 10 i32.const 20 i32.const 1 select) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 10 },
    { name: 'select false (pick second)', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i32.const 10 i32.const 20 i32.const 0 select) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 20 },
    // global.get / global.set
    { name: 'global.get i32 (42)', wat: `(module (global (mut i32) (i32.const 42)) (type (func (result i32))) (func (type 0) (result i32) global.get 0) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    { name: 'global.set then get', wat: `(module (global (mut i32) (i32.const 0)) (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 global.set 0 global.get 0) (export "main" (func 0)))`, funcIdx: 0, args: [99], expect: 99 },
    // i64.const + wrap/extend
    { name: 'i64.const 0x1234', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i64.const 0x1234 i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0x1234 },
    { name: 'i64.extend_i32_s (-1)', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [-1], expect: -1 },
    { name: 'i64.extend_i32_u (0xFFFFFFFF = 0xFFFFFFFF)', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_u i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [-1], expect: -1 },
    // i64 arithmetic
    { name: 'i64.add 10+5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.add i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [10, 5], expect: 15 },
    { name: 'i64.sub 10-3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.sub i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 7 },
    { name: 'i64.mul 6*7', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.mul i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [6, 7], expect: 42 },
    { name: 'i64.div_s 10/3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.div_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [10, 3], expect: 3 },
    { name: 'i64.and 5&3 = 1', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.and i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 1 },
    { name: 'i64.or 1|2 = 3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.or i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [1, 2], expect: 3 },
    { name: 'i64.xor 1^3 = 2', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.xor i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [1, 3], expect: 2 },
    { name: 'i64.shl 1<<3 = 8', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.shl i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [1, 3], expect: 8 },
    { name: 'i64.eq 5==5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.eq) (export "main" (func 0)))`, funcIdx: 0, args: [5, 5], expect: 1 },
    { name: 'i64.ne 5!=7', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.ne) (export "main" (func 0)))`, funcIdx: 0, args: [5, 7], expect: 1 },
    { name: 'i64.gt_s 5>3', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.gt_s) (export "main" (func 0)))`, funcIdx: 0, args: [5, 3], expect: 1 },
    { name: 'i64.lt_s 3<5', wat: `(module (type (func (param i32 i32) (result i32))) (func (type 0) (param i32 i32) (result i32) local.get 0 i64.extend_i32_s local.get 1 i64.extend_i32_s i64.lt_s) (export "main" (func 0)))`, funcIdx: 0, args: [3, 5], expect: 1 },
    // i32.load8_s/u, i32.load16_s/u, i32.store8/16
    { name: 'i32.load8_u 0xFF = 255', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0xFF i32.store8 i32.const 0 i32.load8_u) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0xFF },
    { name: 'i32.load8_s 0x80 = -128', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0x80 i32.store8 i32.const 0 i32.load8_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: -128 },
    { name: 'i32.load16_u 0x7FFF = 32767', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0x7FFF i32.store16 i32.const 0 i32.load16_u) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0x7FFF },
    { name: 'i32.load16_s 0x8000 = -32768', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0x8000 i32.store16 i32.const 0 i32.load16_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: -32768 },
    // f32.const
    { name: 'f32.reinterpret_i32 1.0 -> 0x3F800000', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 1.0 i32.reinterpret_f32) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0x3F800000 },
    // f64.extend8_s / 16_s / 32_s
    { name: 'i64.extend8_s 0xFF', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.extend8_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [0xFF], expect: -1 },
    { name: 'i64.extend32_s 0xFFFFFFFF = -1', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.extend32_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [-1], expect: -1 },
    // f32.eq basic
    { name: 'f32.eq 1.0 1.0', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 1 f32.const 1 f32.eq) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1 },
    // i64.eqz
    { name: 'i64.eqz 0', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.eqz) (export "main" (func 0)))`, funcIdx: 0, args: [0], expect: 1 },
    { name: 'i64.eqz 42', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.eqz) (export "main" (func 0)))`, funcIdx: 0, args: [42], expect: 0 },
    // i64.clz/ctz/popcnt
    { name: 'i64.clz 1', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.clz i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [1], expect: 63 },
    { name: 'i64.ctz 8', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.ctz i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [8], expect: 3 },
    { name: 'i64.popcnt 7', wat: `(module (type (func (param i32) (result i32))) (func (type 0) (param i32) (result i32) local.get 0 i64.extend_i32_s i64.popcnt i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [7], expect: 3 },
    // i32.trunc_f32_s
    { name: 'i32.trunc_f32_s 3.14', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 3.14 i32.trunc_f32_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 3 },
    // i32.trunc_f64_s
    { name: 'i32.trunc_f64_s 3.99', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f64.const 3.99 i32.trunc_f64_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 3 },
    // f32.convert_i32_s
    { name: 'f32.convert_i32_s 42 -> i32.reinterpret_f32', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i32.const 42 f32.convert_i32_s i32.reinterpret_f32) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0x42280000 },
    // f64.convert_i64_s
    { name: 'f64.convert_i64_s 42 -> i64.reinterpret_f64 -> i32.wrap_i64', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i64.const 42 f64.convert_i64_s i64.reinterpret_f64 i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0 },
    // f32.demote_f64
    { name: 'f32.demote_f64 3.14 -> i32.trunc_f32_s', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f64.const 3.14 f32.demote_f64 i32.trunc_f32_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 3 },
    // f64.promote_f32
    { name: 'f64.promote_f32 1.0 -> f64.eq', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 1 f64.promote_f32 f64.const 1 f64.eq) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1 },
    // f32.reinterpret_i32
    { name: 'f32.reinterpret_i32 0x3F800000', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i32.const 0x3F800000 f32.reinterpret_i32 i32.reinterpret_f32) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0x3F800000 },
    // f64.reinterpret_i64
    { name: 'f64.reinterpret_i64 0 -> i32.wrap_i64', wat: `(module (type (func (result i32))) (func (type 0) (result i32) i64.const 0 f64.reinterpret_i64 i64.reinterpret_f64 i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0 },
    // br_table
    { name: 'br_table default (selector out of range)', wat: `(module (func (export "main") (result i32) (block $l0 (block $l1 (br_table $l0 (i32.const 99)) (return (i32.const 10))) (return (i32.const 20))) (i32.const 30)))`, funcIdx: 0, args: [], expect: 30 },
    // call_indirect
    { name: 'call_indirect', wat: `(module (type (func (result i32))) (table funcref (elem $f)) (func $f (result i32) (i32.const 42)) (func (export "main") (result i32) i32.const 0 call_indirect (type 0)))`, funcIdx: 1, args: [], expect: 42 },
    { name: 'call_indirect with args', wat: `(module (type (func (param i32 i32) (result i32))) (table funcref (elem $add)) (func $add (type 0) local.get 0 local.get 1 i32.add) (func (export "main") (type 0) local.get 0 local.get 1 i32.const 0 call_indirect (type 0)))`, funcIdx: 1, args: [10, 32], expect: 42 },
    // memory.copy (bulk-memory)
    { name: 'memory.copy 42', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 42 i32.store i32.const 4 i32.const 0 i32.const 4 memory.copy i32.const 4 i32.load) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // memory.fill
    { name: 'memory.fill FF', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0xFF i32.const 4 memory.fill i32.const 0 i32.load8_u) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0xFF },
    // memory.init
    { name: 'memory.init 42', wat: `(module (memory 1) (data "\\2a\\00\\00\\00") (type (func (result i32))) (func (type 0) (result i32) i32.const 0 i32.const 0 i32.const 4 memory.init 0 data.drop 0 i32.const 0 i32.load) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // saturating trunc ops
    { name: 'i32.trunc_sat_f32_s 3.14 = 3', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 3.14 i32.trunc_sat_f32_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 3 },
    { name: 'i32.trunc_sat_f32_s overflow -> MAX', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 2147483648.0 i32.trunc_sat_f32_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 2147483647 },
    { name: 'i32.trunc_sat_f32_s NaN -> 0', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const nan i32.trunc_sat_f32_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0 },
    { name: 'i32.trunc_sat_f32_u neg -> 0', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const -1.0 i32.trunc_sat_f32_u) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 0 },
    { name: 'i32.trunc_sat_f32_u normal', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 100.5 i32.trunc_sat_f32_u) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 100 },
    { name: 'i64.trunc_sat_f32_s 42', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f32.const 42.0 i64.trunc_sat_f32_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    { name: 'i32.trunc_sat_f64_s 3.99', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f64.const 3.99 i32.trunc_sat_f64_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 3 },
    { name: 'i32.trunc_sat_f64_s overflow -> MAX', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f64.const 2147483648.0 i32.trunc_sat_f64_s) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 2147483647 },
    { name: 'i64.trunc_sat_f64_s 42', wat: `(module (type (func (result i32))) (func (type 0) (result i32) f64.const 42.0 i64.trunc_sat_f64_s i32.wrap_i64) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 42 },
    // Float load/store
    { name: 'f32.store + f32.load', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 f32.const 42.5 f32.store i32.const 0 f32.load i32.reinterpret_f32) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1110048768 },
    { name: 'f64.store + f64.load', wat: `(module (memory 1) (type (func (result i32))) (func (type 0) (result i32) i32.const 0 f64.const 3.14 f64.store i32.const 0 f64.load f64.const 3.14 f64.eq) (export "main" (func 0)))`, funcIdx: 0, args: [], expect: 1 },
    // table.get / table.set
    { name: 'table.set then table.get', wat: `(module (table 1 funcref) (type (func (result i32))) (func $a (type 0) (result i32) i32.const 99) (func $b (export "main") (type 0) (result i32) i32.const 0 ref.func 1 table.set 0 i32.const 0 table.get 0 ref.is_null i32.eqz))`, funcIdx: 1, args: [], expect: 1 },
    // Extended table ops
    { name: 'table.size', wat: `(module (table 2 10 funcref) (type (func (result i32))) (func (export "main") (type 0) (result i32) table.size 0))`, funcIdx: 0, args: [], expect: 2 },
    { name: 'table.grow then size', wat: `(module (table 1 10 funcref) (type (func (result i32))) (func (export "main") (type 0) (result i32) ref.null func i32.const 3 table.grow 0 drop table.size 0))`, funcIdx: 0, args: [], expect: 4 },
    { name: 'table.fill then table.get', wat: `(module (table 2 10 funcref) (type (func (result i32))) (func (export "main") (type 0) (result i32) i32.const 0 ref.null func i32.const 2 table.fill 0 i32.const 0 table.get 0 ref.is_null))`, funcIdx: 0, args: [], expect: 1 },
    { name: 'table.copy', wat: `(module (table 3 10 funcref) (type (func (result i32))) (func $a (type 0) (result i32) i32.const 99) (elem (i32.const 0) func $a) (func (export "main") (type 0) (result i32) i32.const 2 i32.const 0 i32.const 1 table.copy 0 0 i32.const 2 table.get 0 ref.is_null i32.eqz))`, funcIdx: 1, args: [], expect: 1 },
    // Multi-value return
    { name: 'multi-return two i32s', wat: `(module (type (func (result i32 i32))) (func (type 0) (result i32 i32) i32.const 10 i32.const 20) (export "main" (func 0)))`, funcIdx: 0, args: [], multi: [10, 20] },
    { name: 'multi-return return mid-function', wat: `(module (type (func (result i32 i32))) (func (type 0) (result i32 i32) i32.const 30 i32.const 40 return unreachable) (export "main" (func 0)))`, funcIdx: 0, args: [], multi: [30, 40] },
    { name: 'call multi-return function', wat: `(module (type (func (result i32 i32))) (type (func (result i32))) (func $f2 (type 0) (result i32 i32) i32.const 100 i32.const 200) (func $main (export "main") (type 1) (result i32) call 0 drop) (export "main2" (func 0)))`, funcIdx: 1, args: [], expect: 100 },
    { name: 'call_indirect multi-return', wat: `(module (type (func (result i32 i32))) (table 1 funcref) (func $f (type 0) (result i32 i32) i32.const 50 i32.const 60) (elem (i32.const 0) func $f) (type (func (result i32))) (func $main (export "main") (type 1) (result i32) i32.const 0 call_indirect (type 0) drop) (export "main2" (func 0)))`, funcIdx: 1, args: [], expect: 50 },
    { name: 'block multi-value', wat: `(module (type (func (result i32 i32))) (func (type 0) (result i32 i32) block (result i32 i32) i32.const 7 i32.const 8 end) (export "main" (func 0)))`, funcIdx: 0, args: [], multi: [7, 8] },
  ];

  console.log('\n══════════ Execution Tests ══════════');
  let execPassed = 0, execFailed = 0;
  for (const et of execTests) {
    const r = loadAndCall(et.wat, et.funcIdx, et.args);
    let ok;
    if (et.multi) {
      ok = !r.error && r.count === et.multi.length && et.multi.every((v, i) => Number(r.values[i]) === v);
    } else {
      ok = !r.error && r.count === 1 && Number(r.value) === et.expect;
    }
    const info = r.error ? `err=${r.error}` : (et.multi ? `got [${r.values}]` : `got ${Number(r.value)}`);
    const expected = et.multi ? `expected [${et.multi}]` : `expected ${et.expect}`;
    console.log(`  ${ok ? 'PASS' : 'FAIL'} ${et.name}: ${info} (${expected})`);
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
