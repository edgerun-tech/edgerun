const fs = require('fs');
const path = require('path');

const DIR = __dirname;
const ROOT = path.join(DIR, '..');

const CORE    = path.join(ROOT, 'io/edgerun-core.wasm');
const PIPE    = path.join(ROOT, 'io/pipe-core.wasm');
const PIPELN  = path.join(ROOT, 'io/pipeline-core.wasm');
const INTERP  = path.join(DIR, 'wasm-interpreter.wasm');
const WPC     = path.join(ROOT, 'text/wat-parse-core.wasm');
const WPS     = path.join(DIR, 'process-wat-parse.wasm');

async function main() {
  console.log('=== WAT Parse Pipeline Stage Test ===\n');

  const coreBin   = fs.readFileSync(CORE);
  const pipeBin   = fs.readFileSync(PIPE);
  const plBin     = fs.readFileSync(PIPELN);
  const interpBin = fs.readFileSync(INTERP);
  const wpcBin    = fs.readFileSync(WPC);
  const wpsBin    = fs.readFileSync(WPS);

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
    pipe_snapshot:  null, pipe_restore: null,
    pipe_available: null,
  };

  // Instantiate pipe-core
  const pipe = await WebAssembly.instantiate(
    new WebAssembly.Module(pipeBin), { 'edgerun-core': edr });
  pipeIo.pipe_alloc      = pipe.exports.pipe_alloc;
  pipeIo.pipe_create     = pipe.exports.pipe_create;
  pipeIo.pipe_read       = pipe.exports.pipe_read;
  pipeIo.pipe_write      = pipe.exports.pipe_write;
  pipeIo.pipe_drain      = pipe.exports.pipe_drain;
  pipeIo.pipe_close      = pipe.exports.pipe_close;
  pipeIo.pipe_snapshot   = pipe.exports.pipe_snapshot;
  pipeIo.pipe_restore    = pipe.exports.pipe_restore;
  pipeIo.pipe_available  = pipe.exports.pipe_available;

  // Instantiate pipeline-core
  const pl = await WebAssembly.instantiate(
    new WebAssembly.Module(plBin), {
      'edgerun-core': { memory: e('memory'), STATUS_OK: g('STATUS_OK'), STATUS_MORE: g('STATUS_MORE') },
      'pipe-core': {
        pipe_alloc:    pipeIo.pipe_alloc,
        pipe_create:   pipeIo.pipe_create,
        pipe_drain:    pipeIo.pipe_drain,
        pipe_close:    pipeIo.pipe_close,
        pipe_snapshot: pipeIo.pipe_snapshot,
        pipe_restore:  pipeIo.pipe_restore,
      },
    });

  // Instantiate wasm-interpreter (provides decode_opcodes + compute_end_targets)
  const interp = await WebAssembly.instantiate(
    new WebAssembly.Module(interpBin), { 'edgerun-core': edr });

  // Instantiate wat-parse-core
  const wpc = await WebAssembly.instantiate(
    new WebAssembly.Module(wpcBin), {
      'edgerun-core': edr,
      'wasm-interpreter': {
        decode_opcodes:      interp.exports.decode_opcodes,
        compute_end_targets: interp.exports.compute_end_targets,
      },
    });

  // Instantiate process-wat-parse pipeline stage
  const wps = await WebAssembly.instantiate(
    new WebAssembly.Module(wpsBin), {
      'edgerun-core':  { memory: e('memory') },
      'pipe-core':     { pipe_read: pipeIo.pipe_read, pipe_write: pipeIo.pipe_write },
      'wat-parse-core': { wat_parse_module: wpc.exports.wat_parse_module },
    });

  console.log('All modules instantiated.\n');

  // ── Test 1: Parse add.wat via direct stage call ──
  console.log('--- Test 1: Parse add.wat via process_wat_parse ---\n');

  // WAT source: (module (func (export "add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))
  const watSrc = '(module (func (export "add") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))';
  const watBytes = Buffer.from(watSrc, 'utf8');
  console.log(`WAT source (${watBytes.length} bytes): ${watSrc}\n`);

  // Place WAT in memory at a known address
  const WAT_ADDR = 0x200000;
  for (let i = 0; i < watBytes.length; i++) mem[WAT_ADDR + i] = watBytes[i];

  const PIPE_CAP = 4096;
  const pipeIn  = pipeIo.pipe_create(PIPE_CAP);
  const pipeOut = pipeIo.pipe_create(PIPE_CAP);

  let r = pipeIo.pipe_write(pipeIn, WAT_ADDR, watBytes.length);
  console.assert(r === 0, `pipe_write: ${r} (expected 0)`);
  console.log('WAT text written to input pipe.\n');

  // Scratch area
  const SCRATCH = 0x8F100;

  // Clear state counters
  dv.setInt32(g('OFF_TYPE_COUNT'), 0, true);
  dv.setInt32(g('OFF_FUNCTION_COUNT'), 0, true);
  dv.setInt32(g('OFF_CODE_COUNT'), 0, true);
  dv.setInt32(g('OFF_EXPORT_COUNT'), 0, true);
  dv.setInt32(g('OFF_GLOBAL_COUNT'), 0, true);
  dv.setInt32(g('OFF_TABLE_HAS'), 0, true);
  dv.setInt32(g('OFF_MEM_MIN'), 0, true);
  dv.setInt32(g('OFF_START_FUNC'), -1, true);
  dv.setInt32(g('OFF_DATA_COUNT'), 0, true);
  dv.setInt32(g('OFF_ELEM_COUNT'), 0, true);

  // Call process_wat_parse directly (as a stage function)
  const stageResult = wps.exports.process_wat_parse(pipeIn, pipeOut, 0, 0, SCRATCH, 8192, 0);
  console.log(`Stage result: ${stageResult}  (expected 4 = OK with 4 bytes written)\n`);

  if (stageResult < 0) {
    console.error(`Stage returned error ${stageResult}`);
    process.exit(1);
  }

  // Verify output pipe
  const outAvail = pipeIo.pipe_available(pipeOut);
  console.log(`Output pipe: ${outAvail} bytes available (expected 4)`);

  if (outAvail >= 4) {
    pipeIo.pipe_read(pipeOut, SCRATCH, 4);
    const status = dv.getInt32(SCRATCH, true);
    console.log(`Output status: ${status} (expected 0 = OK)\n`);
  }

  // ── Verify interpreter state ──
  console.log('--- State verification ---\n');

  const typeCount   = dv.getInt32(g('OFF_TYPE_COUNT'), true);
  const funcCount   = dv.getInt32(g('OFF_FUNCTION_COUNT'), true);
  const codeCount   = dv.getInt32(g('OFF_CODE_COUNT'), true);
  const exportCount = dv.getInt32(g('OFF_EXPORT_COUNT'), true);
  const memMin      = dv.getInt32(g('OFF_MEM_MIN'), true);
  const dataCount   = dv.getInt32(g('OFF_DATA_COUNT'), true);

  console.log(`type_count:     ${typeCount}     (expected 1)`);
  console.log(`function_count: ${funcCount}     (expected 1)`);
  console.log(`code_count:     ${codeCount}     (expected 1)`);
  console.log(`export_count:   ${exportCount}   (expected 1)`);
  console.log(`mem_min:        ${memMin}        (expected 0 — no memory declared)`);
  console.log(`data_count:     ${dataCount}     (expected 0 — no data declared)`);

  let pass = true;
  if (typeCount !== 1)   { console.error(`  FAIL: type_count`); pass = false; }
  if (funcCount !== 1)   { console.error(`  FAIL: function_count`); pass = false; }
  if (codeCount !== 1)   { console.error(`  FAIL: code_count`); pass = false; }
  if (exportCount !== 1) { console.error(`  FAIL: export_count`); pass = false; }
  if (memMin !== 0)      { console.error(`  FAIL: mem_min`); pass = false; }
  if (dataCount !== 0)   { console.error(`  FAIL: data_count`); pass = false; }

  // Verify type record at offset 0
  const TYPE_SZ   = g('SZ_TYPE');
  const typesBuf  = g('OFF_TYPES_BUF');
  const typeParamCount = dv.getInt32(typesBuf + 4, true); // param_count at +4
  const typeResult     = dv.getInt8(typesBuf + 64);       // result type at +64
  console.log(`\ntype[0].param_count: ${typeParamCount} (expected 2)`);
  console.log(`type[0].result:      0x${typeResult.toString(16)} (expected 0x7F = i32)`);
  if (typeParamCount !== 2) { console.error(`  FAIL: param_count`); pass = false; }
  if (typeResult !== 0x7F)  { console.error(`  FAIL: result type`); pass = false; }

  // Verify function record
  const funcsBuf = g('OFF_FUNCTIONS_BUF');
  const funcTypeIdx = dv.getInt32(funcsBuf + 0, true); // type_index at +0
  console.log(`\nfunc[0].type_index: ${funcTypeIdx} (expected 0)`);
  if (funcTypeIdx !== 0) { console.error(`  FAIL: func type_index`); pass = false; }

  // Verify code record
  const CODE_SZ  = g('SZ_CODE');
  const codeBuf  = g('OFF_CODE_BUF');
  const codeSize = dv.getInt32(codeBuf + 8, true); // body_size at +8
  console.log(`\ncode[0].body_size: ${codeSize} bytes (expected ~3)`);
  if (codeSize < 1) { console.error(`  FAIL: code body_size`); pass = false; }

  // Verify export
  const exportsBuf = g('OFF_EXPORTS_BUF');
  const expNameLen = dv.getInt32(exportsBuf + 8, true);
  const expKind    = dv.getInt8(exportsBuf + 16);
  const expIndex   = dv.getInt32(exportsBuf + 24, true);
  console.log(`\nexport[0].name_len: ${expNameLen} (expected 3 = "add")`);
  console.log(`export[0].kind:      0x${expKind.toString(16)} (expected 0x00 = func)`);
  console.log(`export[0].index:     ${expIndex} (expected 0)`);
  if (expNameLen !== 3) { console.error(`  FAIL: export name_len`); pass = false; }
  if (expKind !== 0x00) { console.error(`  FAIL: export kind`); pass = false; }
  if (expIndex !== 0)   { console.error(`  FAIL: export index`); pass = false; }

  // Verify export name
  const expNamePtr = dv.getInt32(exportsBuf + 0, true);
  const exportName = Buffer.from(mem.slice(expNamePtr, expNamePtr + expNameLen)).toString('utf8');
  console.log(`\nexport[0].name: "${exportName}" (expected "add")`);
  if (exportName !== 'add') { console.error(`  FAIL: export name string`); pass = false; }

  // Verify decoded opcodes
  const decodedCount = dv.getInt32(g('OFF_DECODED_COUNT'), true);
  console.log(`\ndecoded_count: ${decodedCount} (expected > 0)`);
  if (decodedCount <= 0) { console.error(`  FAIL: decoded_count`); pass = false; }

  console.log(`\n${pass ? '=== ALL TESTS PASSED ===' : '=== SOME TESTS FAILED ==='}`);
  process.exit(pass ? 0 : 1);
}

main().catch(e => console.error('Error:', e.message, e.stack));
