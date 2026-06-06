const fs = require('fs');
const path = require('path');

const DIR = __dirname;
const PIPELINE = path.join(DIR, 'pipeline.wasm');

async function main() {
  console.log('=== Pipeline × WAT exec (process_exec auto-detect) ===\n');

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

  mod.exports.stage_table.set(13, mod.exports.process_exec);

  const PIPE_CAP = 4096;
  const SCRATCH = 0x8F100;
  let passed = 0, failed = 0;
  function check(label, ok) {
    if (ok) { passed++; console.log(`  \u2713 ${label}`); }
    else    { failed++; console.log(`  \u2717 ${label}`); }
  }

  // ── Test 1: direct load_wat call ──
  {
    const watSrc = '(module\n  (func (export "add") (param i32 i32) (result i32)\n    local.get 0\n    local.get 1\n    i32.add\n  )\n)';
    const watBytes = Buffer.from(watSrc, 'utf8');
    console.log(`Test 1: direct load_wat ...\n`);

    const ADDR = 0x100000;
    for (let i = 0; i < watBytes.length; i++) mem[ADDR + i] = watBytes[i];
    const direct = e('load_wat')(ADDR, watBytes.length);
    check('direct load_wat OK', direct === 0);
    if (direct !== 0) {
      console.log(`  load_wat returned ${direct}`);
      console.log(`  WAT_DBG = 0x${dv.getUint32(0x8C020, true).toString(16)}`);
      console.log(`  0x8C048 = 0x${dv.getUint32(0x8C048, true).toString(16)}`);  // body parser dbg
      console.log(`  0x8C060 = 0x${dv.getUint32(0x8C060, true).toString(16)}`);  // body pos
      console.log(`  0x8C080 = 0x${dv.getUint32(0x8C080, true).toString(16)}`);  // body_off
    }
  }

  // ── Test 2: WAT via pipeline ──
  {
    const watSrc2 = '(module\n  (func (export "add") (param i32 i32) (result i32)\n    local.get 0\n    local.get 1\n    i32.add\n  )\n)';
    const watBytes2 = Buffer.from(watSrc2, 'utf8');
    console.log(`\nTest 2: WAT pipeline add(3,7)=10 (${watBytes2.length}B)\n`);

    const ADDR = 0x100000;
    for (let i = 0; i < watBytes2.length; i++) mem[ADDR + i] = watBytes2[i];

    const pipeIn  = e('pipe_create')(PIPE_CAP);
    const pipeOut = e('pipe_create')(PIPE_CAP);
    e('pipe_write')(pipeIn, ADDR, watBytes2.length);

    const CONFIG = 0x8F000;
    dv.setInt32(CONFIG + 0, 0, true);
    dv.setInt32(CONFIG + 4, 2, true);
    dv.setInt32(CONFIG + 8, 3, true);
    dv.setInt32(CONFIG + 12, 7, true);

    const desc = mod.exports.pipeline_create(PIPE_CAP, 1);
    mod.exports.pipeline_set_stage(desc, 0, 13, CONFIG, 16);

    const result = mod.exports.pipeline_run(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('pipeline_run OK', result === 0);
    if (result !== 0) console.log(`  error ${result}, WAT_DBG=0x${dv.getUint32(0x8C020, true).toString(16)}`);

    const avail = e('pipe_available')(pipeOut);
    check('output 4 bytes', avail === 4);

    if (avail >= 4) {
      e('pipe_read')(pipeOut, SCRATCH, 4);
      const val = dv.getInt32(SCRATCH, true);
      check(`result=${val}`, val === 10);
    }
  }

  // ── Test 3: WASM backward compat ──
  {
    const wasmBytes = new Uint8Array([
      0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x07, 0x01, 0x60, 0x02, 0x7F, 0x7F, 0x01, 0x7F,
      0x03, 0x02, 0x01, 0x00,
      0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
      0x0A, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6A, 0x0B,
    ]);
    console.log(`\nTest 3: WASM binary add(10,20)=30 (${wasmBytes.length}B)\n`);

    const ADDR = 0x100000;
    for (let i = 0; i < wasmBytes.length; i++) mem[ADDR + i] = wasmBytes[i];

    const pipeIn  = e('pipe_create')(PIPE_CAP);
    const pipeOut = e('pipe_create')(PIPE_CAP);
    e('pipe_write')(pipeIn, ADDR, wasmBytes.length);

    const CONFIG = 0x8F000;
    dv.setInt32(CONFIG + 0, 0, true);
    dv.setInt32(CONFIG + 4, 2, true);
    dv.setInt32(CONFIG + 8, 10, true);
    dv.setInt32(CONFIG + 12, 20, true);

    const desc = mod.exports.pipeline_create(PIPE_CAP, 1);
    mod.exports.pipeline_set_stage(desc, 0, 13, CONFIG, 16);

    const result = mod.exports.pipeline_run(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('pipeline_run OK', result === 0);

    const avail = e('pipe_available')(pipeOut);
    check('output 4 bytes', avail === 4);

    if (avail >= 4) {
      e('pipe_read')(pipeOut, SCRATCH, 4);
      const val = dv.getInt32(SCRATCH, true);
      check(`result=${val}`, val === 30);
    }
  }

  console.log(`\n${passed}/${passed + failed} tests passed`);
  if (failed > 0) process.exit(1);
}

main().catch(e => console.error('Error:', e.message, e.stack));
