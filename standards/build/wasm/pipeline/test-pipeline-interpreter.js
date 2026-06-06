const fs = require('fs');
const path = require('path');

const DIR = __dirname;
const PIPELINE = path.join(DIR, 'pipeline.wasm');

function makeAddWasm() {
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x07, 0x01, 0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
    0x0A, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B,
  ]);
}

async function main() {
  console.log('=== Pipeline × WASM Interpreter Integration Test ===\n');

  const bin = fs.readFileSync(PIPELINE);

  const mod = await WebAssembly.instantiate(new WebAssembly.Module(bin), {});
  const mem = new Uint8Array(mod.exports.memory.buffer);
  const dv  = new DataView(mod.exports.memory.buffer);

  const g = (name) => {
    const v = mod.exports[name];
    if (typeof v === 'function') return v();
    if (v && v.value !== undefined) return v.value;
    return v;
  };
  const e = (name) => mod.exports[name];

  console.log(`Pipeline module loaded: ${Object.keys(mod.exports).length} exports`);

  const pipeIo = {
    pipe_alloc:    e('pipe_alloc'),
    pipe_create:   e('pipe_create'),
    pipe_read:     e('pipe_read'),
    pipe_write:    e('pipe_write'),
    pipe_drain:    e('pipe_drain'),
    pipe_close:    e('pipe_close'),
    pipe_available: e('pipe_available'),
    pipe_snapshot:  e('pipe_snapshot'),
    pipe_restore:   e('pipe_restore'),
  };

  // Wire process_wasm_exec into dispatch table slot 11
  mod.exports.stage_table.set(11, mod.exports.process_wasm_exec);

  console.log('\nDispatch table wired.\n');

  // ── Test ──
  const wasmBytes = makeAddWasm();
  console.log(`WASM module: ${wasmBytes.length} bytes (add(3,7) → 10)`);

  const WASM_ADDR = 0x100000;
  for (let i = 0; i < wasmBytes.length; i++) mem[WASM_ADDR + i] = wasmBytes[i];

  const PIPE_CAP = 4096;
  const pipeIn  = pipeIo.pipe_create(PIPE_CAP);
  const pipeOut = pipeIo.pipe_create(PIPE_CAP);

  let r = pipeIo.pipe_write(pipeIn, WASM_ADDR, wasmBytes.length);
  console.assert(r === 0, `pipe_write: ${r}`);

  // Set up config: [func_idx=0, arg_count=2, arg0=3, arg1=7]
  const CONFIG_ADDR = 0x8F000;
  dv.setInt32(CONFIG_ADDR + 0, 0, true);   // func_idx = 0
  dv.setInt32(CONFIG_ADDR + 4, 2, true);   // arg_count = 2
  dv.setInt32(CONFIG_ADDR + 8, 3, true);   // arg0 = 3
  dv.setInt32(CONFIG_ADDR + 12, 7, true);  // arg1 = 7

  // Create pipeline: 1 stage (wasm_exec, config at CONFIG_ADDR, clen=16)
  const SCRATCH = 0x8F100;
  const desc = mod.exports.pipeline_create(PIPE_CAP, 1);
  mod.exports.pipeline_set_stage(desc, 0, 11, CONFIG_ADDR, 16);

  console.log('\nRunning: pipeline_run [wasm_exec] ...');
  const result = mod.exports.pipeline_run(desc, pipeIn, pipeOut, SCRATCH, 8192);
  console.log(`Result: ${result}  (0 = OK, <0 = error)`);

  if (result !== 0) {
    console.error(`Pipeline failed (error ${result})`);
    return;
  }

  const avail = pipeIo.pipe_available(pipeOut);
  console.log(`Output pipe: ${avail} bytes`);

  if (avail >= 4) {
    pipeIo.pipe_read(pipeOut, SCRATCH, 4);
    const val = dv.getInt32(SCRATCH, true);
    console.log(`\n✓ add(3, 7) = ${val}  (expected 10)`);
    console.log(val === 10 ? '\n=== TEST PASSED ===' : '\n=== TEST FAILED ===');
  } else {
    console.log('\n=== TEST FAILED (no output) ===');
  }
}

main().catch(e => console.error('Error:', e.message, e.stack));
