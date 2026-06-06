const fs = require('fs');
const path = require('path');

const DIR = __dirname;
const PIPELINE = path.join(DIR, 'pipeline.wasm');

async function main() {
  console.log('=== Pipe Core Extended Tests ===\n');

  const bin = fs.readFileSync(PIPELINE);
  const mod = await WebAssembly.instantiate(new WebAssembly.Module(bin), {});
  const mem = new Uint8Array(mod.exports.memory.buffer);
  const dv  = new DataView(mod.exports.memory.buffer);

  const e = (name) => mod.exports[name];

  const pipeIo = {
    create:   e('pipe_create'),
    set_mode: e('pipe_set_mode'),
    write:    e('pipe_write'),
    read:     e('pipe_read'),
    read_ptr: e('pipe_read_ptr'),
    advance:  e('pipe_advance'),
    available: e('pipe_available'),
    space:    e('pipe_space'),
    state:    e('pipe_state'),
    close:    e('pipe_close'),
    reset:    e('pipe_reset'),
    snapshot: e('pipe_snapshot'),
    restore:  e('pipe_restore'),
    reset_heap: e('pipe_reset_heap'),
  };

  const SCR = 0x8F000;
  let passed = 0, failed = 0;

  function check(label, ok) {
    if (ok) { passed++; console.log(`  ✓ ${label}`); }
    else    { failed++; console.log(`  ✗ ${label}`); }
  }

  // ── Test 1: Linear pipe basic write/read ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(64);
    check('linear: create', p > 0);
    check('linear: available empty', pipeIo.available(p) === 0);
    check('linear: space full', pipeIo.space(p) === 64);

    // Write "Hello"
    for (let i = 0; i < 5; i++) mem[SCR + i] = 'Hello'.charCodeAt(i);
    let r = pipeIo.write(p, SCR, 5);
    check('linear: write ok', r === 0);
    check('linear: available 5', pipeIo.available(p) === 5);
    check('linear: space 59', pipeIo.space(p) === 59);

    // Read back
    r = pipeIo.read(p, SCR + 100, 64);
    check('linear: read 5', r === 5);
    let s = '';
    for (let i = 0; i < 5; i++) s += String.fromCharCode(mem[SCR + 100 + i]);
    check('linear: content "Hello"', s === 'Hello');
    check('linear: available 0 after drain', pipeIo.available(p) === 0);
  }

  // ── Test 2: Circular pipe basic ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(16);
    pipeIo.set_mode(p, 2); // PIPE_MODE_CIRCULAR = 2
    check('circular: create', p > 0);
    check('circular: mode set', (pipeIo.state(p) & 2) !== 0);

    // Write 10 bytes
    for (let i = 0; i < 10; i++) mem[SCR + i] = 0xAA + i;
    let r = pipeIo.write(p, SCR, 10);
    check('circular: write 10 ok', r === 0);
    check('circular: available 10', pipeIo.available(p) === 10);
    check('circular: space 6', pipeIo.space(p) === 6);

    // Read 5 bytes
    r = pipeIo.read(p, SCR + 200, 5);
    check('circular: read 5', r === 5);
    check('circular: available 5 remaining', pipeIo.available(p) === 5);
    for (let i = 0; i < 5; i++) {
      check(`circular: byte ${i} = 0x${(0xAA + i).toString(16)}`, mem[SCR + 200 + i] === 0xAA + i);
    }
  }

  // ── Test 3: Circular pipe wrap-around ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(8);
    pipeIo.set_mode(p, 2);
    check('wrap: create', p > 0);

    // Write 6 bytes (space remaining: 2)
    for (let i = 0; i < 6; i++) mem[SCR + i] = i + 10;
    let r = pipeIo.write(p, SCR, 6);
    check('wrap: write 6 ok', r === 0);
    check('wrap: available 6', pipeIo.available(p) === 6);

    // Read 4 bytes (moves rd forward, leaves 2 bytes at positions 4-5)
    r = pipeIo.read(p, SCR + 100, 4);
    check('wrap: read 4', r === 4);
    check('wrap: available 2', pipeIo.available(p) === 2);

    // Write 4 more bytes — should wrap (positions 6,7 wrap to 0,1)
    for (let i = 0; i < 4; i++) mem[SCR + 20 + i] = 100 + i;
    r = pipeIo.write(p, SCR + 20, 4);
    check('wrap: write 4 (will wrap) ok', r === 0);
    check('wrap: available 6', pipeIo.available(p) === 6);

    // Read all 6 — first 2 are old data (10+4=14, 10+5=15), next 4 are new (100,101,102,103)
    r = pipeIo.read(p, SCR + 300, 16);
    check('wrap: read all 6', r === 6);
    check('wrap: byte0=14', mem[SCR + 300] === 14);
    check('wrap: byte1=15', mem[SCR + 301] === 15);
    check('wrap: byte2=100', mem[SCR + 302] === 100);
    check('wrap: byte3=101', mem[SCR + 303] === 101);
    check('wrap: byte4=102', mem[SCR + 304] === 102);
    check('wrap: byte5=103', mem[SCR + 305] === 103);
  }

  // ── Test 4: Circular pipe overflow ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(8);
    pipeIo.set_mode(p, 2);
    for (let i = 0; i < 8; i++) mem[SCR + i] = i;
    let r = pipeIo.write(p, SCR, 8);
    check('overflow: fill to capacity', r === 0);
    r = pipeIo.write(p, SCR, 1);
    check('overflow: write 1 more fails', r === 4); // STATUS_OVERFLOW
  }

  // ── Test 5: Zero-copy pipe_read_ptr + pipe_advance (linear) ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(64);
    for (let i = 0; i < 7; i++) mem[SCR + i] = 'network'.charCodeAt(i);
    pipeIo.write(p, SCR, 7);
    check('read_ptr: write ok', pipeIo.available(p) === 7);

    const lenAddr = SCR + 100;
    const ptr = pipeIo.read_ptr(p, lenAddr);
    const avail = dv.getInt32(lenAddr, true);
    check('read_ptr: avail 7', avail === 7);
    let s = '';
    for (let i = 0; i < avail; i++) s += String.fromCharCode(mem[ptr + i]);
    check('read_ptr: content "network"', s === 'network');

    pipeIo.advance(p, 4);
    check('read_ptr: available 3 after advance 4', pipeIo.available(p) === 3);

    // Read remaining with pipe_read
    let r = pipeIo.read(p, SCR + 200, 8);
    check('read_ptr: read remaining 3', r === 3);
    s = '';
    for (let i = 0; i < 3; i++) s += String.fromCharCode(mem[SCR + 200 + i]);
    check('read_ptr: remaining "ork"', s === 'ork');
  }

  // ── Test 6: Zero-copy pipe_read_ptr (circular, non-wrapping) ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(16);
    pipeIo.set_mode(p, 2);
    for (let i = 0; i < 10; i++) mem[SCR + i] = 200 + i;
    pipeIo.write(p, SCR, 10);
    check('read_ptr_circ: write 10', pipeIo.available(p) === 10);

    const lenAddr = SCR + 100;
    const ptr = pipeIo.read_ptr(p, lenAddr);
    const avail = dv.getInt32(lenAddr, true);
    check('read_ptr_circ: avail 10', avail === 10);
    check('read_ptr_circ: ptr points to pipe data', ptr >= 0x40000);
    for (let i = 0; i < 5; i++) {
      check(`read_ptr_circ: byte ${i} = ${200 + i}`, mem[ptr + i] === 200 + i);
    }

    pipeIo.advance(p, 10);
    check('read_ptr_circ: empty after advance', pipeIo.available(p) === 0);
  }

  // ── Test 7: Close preserves mode ──
  {
    pipeIo.reset_heap();
    const p = pipeIo.create(16);
    pipeIo.set_mode(p, 2);
    pipeIo.close(p);
    check('close_circ: state = 3 (closed|circular)', pipeIo.state(p) === 3);

    pipeIo.reset_heap();
    const p2 = pipeIo.create(16);
    pipeIo.close(p2);
    check('close_linear: state = 1 (closed)', pipeIo.state(p2) === 1);
  }

  console.log(`\n${passed}/${passed + failed} tests passed`);
  if (failed > 0) process.exit(1);
}

main().catch(e => console.error('Error:', e.message, e.stack));
