const fs = require('fs');
const path = require('path');

const DIR = __dirname;
const ROOT = path.join(DIR, '..');

const CORE    = path.join(ROOT, 'io/edgerun-core.wasm');
const PIPE    = path.join(ROOT, 'io/pipe-core.wasm');
const PIPELN  = path.join(ROOT, 'io/pipeline-core.wasm');
const INTERP  = path.join(DIR, 'wasm-interpreter.wasm');
const EXEC    = path.join(DIR, 'wasm-exec-stage.wasm');

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

  const coreBin   = fs.readFileSync(CORE);
  const pipeBin   = fs.readFileSync(PIPE);
  const plBin     = fs.readFileSync(PIPELN);
  const interpBin = fs.readFileSync(INTERP);
  const execBin   = fs.readFileSync(EXEC);

  // Instantiate edgerun-core
  const core = await WebAssembly.instantiate(new WebAssembly.Module(coreBin), {});
  const mem = new Uint8Array(core.exports.memory.buffer);
  const dv  = new DataView(core.exports.memory.buffer);

  const g = (name) => {
    const v = core.exports[name];
    if (typeof v === 'function') return v();
    if (v && v.value !== undefined) return v.value;
    return v;
  };
  const e = (name) => core.exports[name];

  // Shared import object for edgerun-core consumers
  const edr = {
    memory:  e('memory'),
    pack:    e('pack'),
    STATUS_OK:           g('STATUS_OK'),
    STATUS_MORE:         g('STATUS_MORE'),
    STATUS_INPUT_SHORT:  g('STATUS_INPUT_SHORT'),
    STATUS_OUTPUT_SHORT: g('STATUS_OUTPUT_SHORT'),
    STATUS_OVERFLOW:     g('STATUS_OVERFLOW'),
    OFF_TYPES_BUF:       g('OFF_TYPES_BUF'),
    OFF_CODE_BUF:        g('OFF_CODE_BUF'),
    OFF_FUNCTIONS_BUF:   g('OFF_FUNCTIONS_BUF'),
    OFF_DECODED_OPS:     g('OFF_DECODED_OPS'),
    OFF_DECODED_COUNT:   g('OFF_DECODED_COUNT'),
    DEC_SZ:              g('DEC_SZ'),
    SZ_TYPE:             g('SZ_TYPE'),
    SZ_FUNC:             g('SZ_FUNC'),
    SZ_CODE:             g('SZ_CODE'),
  };
  const pipeIo = {
    pipe_alloc:  null, pipe_create: null,
    pipe_read:   null, pipe_write:  null,
    pipe_drain:  null, pipe_close:  null,
    pipe_available: null,
    pipe_snapshot: null, pipe_restore: null,
  };

  // Instantiate pipe-core
  const pipe = await WebAssembly.instantiate(
    new WebAssembly.Module(pipeBin), { 'edgerun-core': edr });
  pipeIo.pipe_alloc  = pipe.exports.pipe_alloc;
  pipeIo.pipe_create = pipe.exports.pipe_create;
  pipeIo.pipe_read   = pipe.exports.pipe_read;
  pipeIo.pipe_write  = pipe.exports.pipe_write;
  pipeIo.pipe_drain  = pipe.exports.pipe_drain;
  pipeIo.pipe_close  = pipe.exports.pipe_close;
  pipeIo.pipe_available = pipe.exports.pipe_available;
  pipeIo.pipe_snapshot  = pipe.exports.pipe_snapshot;
  pipeIo.pipe_restore   = pipe.exports.pipe_restore;

  // Instantiate pipeline-core
  const pl = await WebAssembly.instantiate(
    new WebAssembly.Module(plBin), {
      'edgerun-core': { memory: e('memory'), STATUS_OK: g('STATUS_OK'), STATUS_MORE: g('STATUS_MORE') },
      'pipe-core': {
        pipe_alloc:  pipeIo.pipe_alloc,
        pipe_create: pipeIo.pipe_create,
        pipe_drain:    pipeIo.pipe_drain,
        pipe_close:    pipeIo.pipe_close,
        pipe_snapshot: pipeIo.pipe_snapshot,
        pipe_restore:  pipeIo.pipe_restore,
      },
    });

  // Instantiate wasm-interpreter
  const interp = await WebAssembly.instantiate(
    new WebAssembly.Module(interpBin), { 'edgerun-core': edr });

  // Instantiate wasm-exec-stage
  const exec = await WebAssembly.instantiate(
    new WebAssembly.Module(execBin), {
      'edgerun-core':     { memory: e('memory') },
      'pipe-core':        { pipe_read: pipeIo.pipe_read, pipe_write: pipeIo.pipe_write },
      'wasm-interpreter': {
        load:              interp.exports.load,
        call:              interp.exports.call,
        get_result_value:  interp.exports.get_result_value,
        get_result_count:  interp.exports.get_result_count,
      },
    });

  // Wire process_wasm_exec into dispatch table slot 11
  pl.exports.stage_table.set(11, exec.exports.process_wasm_exec);

  console.log('All modules instantiated, dispatch table wired.\n');

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
  const SCRATCH = 0x8F100;  // scratch workspace area
  const desc = pl.exports.pipeline_create(PIPE_CAP, 1);
  pl.exports.pipeline_set_stage(desc, 0, 11, CONFIG_ADDR, 16);

  console.log('\nRunning: pipeline_run [wasm_exec] ...');
  const result = pl.exports.pipeline_run(desc, pipeIn, pipeOut, SCRATCH, 8192);
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
