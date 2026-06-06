const fs = require('fs');
const path = require('path');

async function main() {
  console.log('=== Batch Stage Fusion Test ===\n');

  const bin = fs.readFileSync(path.join(__dirname, 'pipeline.wasm'));
  const mod = await WebAssembly.instantiate(new WebAssembly.Module(bin), {});
  const mem = new Uint8Array(mod.exports.memory.buffer);

  const e = (name) => mod.exports[name];
  const g = (name) => { const v = mod.exports[name]; return typeof v === 'function' ? v() : v; };

  // Wire encoding stages into dispatch table
  const stages = {
    passthrough: 0,
    hex_encode:  1,
    hex_decode:  2,
    b64_encode:  3,
    b64_decode:  4,
  };
  mod.exports.stage_table.set(stages.hex_encode, e('process_hex_encode'));
  mod.exports.stage_table.set(stages.hex_decode, e('process_hex_decode'));
  mod.exports.stage_table.set(stages.b64_encode, e('process_b64_encode'));
  mod.exports.stage_table.set(stages.b64_decode, e('process_b64_decode'));

  const pipe = {
    create:    e('pipe_create'),
    write:     e('pipe_write'),
    read:      e('pipe_read'),
    available: e('pipe_available'),
    snapshot:  e('pipe_snapshot'),
    restore:   e('pipe_restore'),
    reset_heap: e('pipe_reset_heap'),
  };

  const SCRATCH = 0x8F000;
  const PIPE_CAP = 4096;

  let passed = 0, failed = 0;
  function check(label, ok) {
    if (ok) { passed++; console.log(`  ✓ ${label}`); }
    else    { failed++; console.log(`  ✗ ${label}`); }
  }

  // ── Fusion Test 1: hex_encode → hex_decode ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 2);
    e('pipeline_set_stage')(desc, 0, stages.hex_encode, 0, 0);
    e('pipeline_set_stage')(desc, 1, stages.hex_decode, 0, 0);

    const msg = 'HelloFusion';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hex_encode→hex_decode: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('hex_encode→hex_decode: output length', avail === msg.length);
    if (avail === msg.length) {
      pipe.read(pipeOut, SCRATCH + 200, avail);
      let s = '';
      for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[SCRATCH + 200 + i]);
      check('hex_encode→hex_decode: content', s === msg);
    }
  }

  // ── Fusion Test 2: b64_encode → b64_decode ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 2);
    e('pipeline_set_stage')(desc, 0, stages.b64_encode, 0, 0);
    e('pipeline_set_stage')(desc, 1, stages.b64_decode, 0, 0);

    const msg = 'Base64IsFun!';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('b64_encode→b64_decode: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('b64_encode→b64_decode: output length', avail === msg.length);
    if (avail === msg.length) {
      pipe.read(pipeOut, SCRATCH + 200, avail);
      let s = '';
      for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[SCRATCH + 200 + i]);
      check('b64_encode→b64_decode: content', s === msg);
    }
  }

  // ── Fusion Test 3: hex_encode → b64_encode (mixed batch, different transforms) ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 2);
    e('pipeline_set_stage')(desc, 0, stages.hex_encode, 0, 0);
    e('pipeline_set_stage')(desc, 1, stages.b64_encode, 0, 0);

    const msg = 'ABC';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hex_encode→b64_encode: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    // hex('ABC') = '414243', b64(hex) = b64('414243') = 'NDE0MjQz'
    check('hex_encode→b64_encode: has output', avail > 0);
    if (avail > 0) {
      pipe.read(pipeOut, SCRATCH + 200, avail);
      let s = '';
      for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[SCRATCH + 200 + i]);
      check('hex_encode→b64_encode: output is b64(hex)', s === 'NDE0MjQz');
    }
  }

  // ── Fusion Test 4: Three-stage batch chain (hex → b64_encode → hex) ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 3);
    e('pipeline_set_stage')(desc, 0, stages.hex_decode, 0, 0);
    e('pipeline_set_stage')(desc, 1, stages.b64_encode, 0, 0);
    e('pipeline_set_stage')(desc, 2, stages.hex_encode, 0, 0);

    const msg = '4865784465636F6465'; // hex('HexDecode')
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hex→b64→hex 3-stage: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('hex→b64→hex 3-stage: has output', avail > 0);
    if (avail > 0) {
      pipe.read(pipeOut, SCRATCH + 200, avail);
      let s = '';
      for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[SCRATCH + 200 + i]);
      // hex_decode → b64_encode → hex_encode => hex(b64('HexDecode'))
      check('hex→b64→hex 3-stage: output', s === '534756345247566a6232526c');
    }
  }

  // ── Fusion Test 5: Mixed batch + streaming (transport stage has state) ──
  // This tests that fusion correctly stops at streaming stages
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 3);
    e('pipeline_set_stage')(desc, 0, stages.hex_encode, 0, 0);
    e('pipeline_set_stage')(desc, 1, stages.hex_decode, 0, 0);
    e('pipeline_set_stage')(desc, 2, stages.passthrough, 0, 0);

    // Passthrough is already at table index 0 (set by elem in pipeline-core)

    const msg = 'Pipeline!';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hex→hex→passthrough 3-stage batch: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('hex→hex→passthrough: output length', avail === msg.length);
    if (avail === msg.length) {
      pipe.read(pipeOut, SCRATCH + 200, avail);
      let s = '';
      for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[SCRATCH + 200 + i]);
      check('hex→hex→passthrough: content', s === msg);
    }
  }

  console.log(`\n${passed}/${passed + failed} tests passed`);
  if (failed > 0) process.exit(1);
}

main().catch(e => console.error('Error:', e.message, e.stack));
