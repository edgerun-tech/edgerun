const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

async function main() {
  console.log('=== SHA-256 / HMAC-SHA256 Pipeline Stage Tests ===\n');

  const bin = fs.readFileSync(path.join(__dirname, 'pipeline.wasm'));
  const mod = await WebAssembly.instantiate(new WebAssembly.Module(bin), {});
  const mem = new Uint8Array(mod.exports.memory.buffer);

  const e = (name) => mod.exports[name];
  const g = (name) => { const v = mod.exports[name]; return typeof v === 'function' ? v() : v; };

  const stages = { sha256: 16, hmac_sha256: 17 };
  mod.exports.stage_table.set(stages.sha256, e('process_sha256'));
  mod.exports.stage_table.set(stages.hmac_sha256, e('process_hmac_sha256'));

  const pipe = {
    create:    e('pipe_create'),
    write:     e('pipe_write'),
    read:      e('pipe_read'),
    available: e('pipe_available'),
    snapshot:  e('pipe_snapshot'),
    restore:   e('pipe_restore'),
    reset_heap: e('pipe_reset_heap'),
    alloc:     e('pipe_alloc'),
  };

  const SCRATCH = 0x8F000;
  const PIPE_CAP = 4096;

  let passed = 0, failed = 0;
  function check(label, ok) {
    if (ok) { passed++; console.log(`  ✓ ${label}`); }
    else    { failed++; console.log(`  ✗ ${label}`); }
  }

  function hex(s) {
    let h = '';
    for (let i = 0; i < s.length; i++) h += s.charCodeAt(i).toString(16).padStart(2, '0');
    return h;
  }

  // ── SHA-256 Test 1: Empty string ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);
    e('pipeline_set_stage')(desc, 0, stages.sha256, 0, 0);

    // Empty input → no write needed
    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('sha256 empty: result OK', r === 0);
    const avail = pipe.available(pipeOut);
    check('sha256 empty: no output (no input)', avail === 0);
  }

  // ── SHA-256 Test 2: "abc" ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);
    e('pipeline_set_stage')(desc, 0, stages.sha256, 0, 0);

    const msg = 'abc';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('sha256 "abc": result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('sha256 "abc": output 32 bytes', avail === 32);
    if (avail === 32) {
      pipe.read(pipeOut, SCRATCH + 200, 32);
      const expected = crypto.createHash('sha256').update(msg).digest();
      let match = true;
      for (let i = 0; i < 32; i++) {
        if (mem[SCRATCH + 200 + i] !== expected[i]) { match = false; break; }
      }
      check('sha256 "abc": content matches Node crypto', match);
    }
  }

  // ── SHA-256 Test 3: Longer message ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);
    e('pipeline_set_stage')(desc, 0, stages.sha256, 0, 0);

    const msg = 'The quick brown fox jumps over the lazy dog';
    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('sha256 "fox": result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('sha256 "fox": output 32 bytes', avail === 32);
    if (avail === 32) {
      pipe.read(pipeOut, SCRATCH + 200, 32);
      const expected = crypto.createHash('sha256').update(msg).digest();
      let match = true;
      for (let i = 0; i < 32; i++) {
        if (mem[SCRATCH + 200 + i] !== expected[i]) { match = false; break; }
      }
      check('sha256 "fox": content matches Node crypto', match);
    }
  }

  // ── SHA-256 Test 4: 1MB data ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);
    e('pipeline_set_stage')(desc, 0, stages.sha256, 0, 0);

    // Generate 1MB of deterministic data
    const size = 1024 * 1024;
    for (let i = 0; i < size; i++) mem[SCRATCH + 100 + i] = i & 0xFF;
    pipe.write(pipeIn, SCRATCH + 100, size);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('sha256 1MB: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('sha256 1MB: output 32 bytes', avail === 32);
    if (avail === 32) {
      pipe.read(pipeOut, SCRATCH + 200, 32);
      const expected = crypto.createHash('sha256').update(mem.slice(SCRATCH + 100, SCRATCH + 100 + size)).digest();
      let match = true;
      for (let i = 0; i < 32; i++) {
        if (mem[SCRATCH + 200 + i] !== expected[i]) { match = false; break; }
      }
      check('sha256 1MB: content matches Node crypto', match);
    }
  }

  // ── HMAC-SHA256 Test 1: RFC 4231 Test Case 2 ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);

    // Key: "Jefe" (4 bytes)
    const key = 'Jefe';
    const msg = 'what do ya want for nothing?';

    // Allocate config in pipe heap: [klen: i32][key: klen]
    const cfg = pipe.alloc(4 + key.length);
    mem[cfg + 0] = key.length & 0xFF;
    mem[cfg + 1] = (key.length >> 8) & 0xFF;
    mem[cfg + 2] = (key.length >> 16) & 0xFF;
    mem[cfg + 3] = (key.length >> 24) & 0xFF;
    for (let i = 0; i < key.length; i++) mem[cfg + 4 + i] = key.charCodeAt(i);

    e('pipeline_set_stage')(desc, 0, stages.hmac_sha256, cfg, 4 + key.length);

    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg.charCodeAt(i);
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hmac_sha256 RFC 4231 #2: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('hmac_sha256 RFC 4231 #2: output 32 bytes', avail === 32);
    if (avail === 32) {
      pipe.read(pipeOut, SCRATCH + 200, 32);
      const expected = crypto.createHmac('sha256', key).update(msg).digest();
      let match = true;
      for (let i = 0; i < 32; i++) {
        if (mem[SCRATCH + 200 + i] !== expected[i]) { match = false; break; }
      }
      check('hmac_sha256 RFC 4231 #2: content matches Node crypto', match);
    }
  }

  // ── HMAC-SHA256 Test 2: RFC 4231 Test Case 4 ──
  {
    pipe.reset_heap();
    const pipeIn  = pipe.create(PIPE_CAP);
    const pipeOut = pipe.create(PIPE_CAP);
    const desc = e('pipeline_create')(PIPE_CAP, 1);

    const key = Buffer.alloc(25, 0xaa); // 25 bytes of 0xaa
    const msg = Buffer.alloc(50, 0xcd); // 50 bytes of 0xcd

    const cfg = pipe.alloc(4 + key.length);
    mem[cfg + 0] = key.length & 0xFF;
    mem[cfg + 1] = (key.length >> 8) & 0xFF;
    mem[cfg + 2] = (key.length >> 16) & 0xFF;
    mem[cfg + 3] = (key.length >> 24) & 0xFF;
    for (let i = 0; i < key.length; i++) mem[cfg + 4 + i] = key[i];

    e('pipeline_set_stage')(desc, 0, stages.hmac_sha256, cfg, 4 + key.length);

    for (let i = 0; i < msg.length; i++) mem[SCRATCH + 100 + i] = msg[i];
    pipe.write(pipeIn, SCRATCH + 100, msg.length);

    const r = e('pipeline_run')(desc, pipeIn, pipeOut, SCRATCH, 8192);
    check('hmac_sha256 RFC 4231 #4: result OK', r === 0);

    const avail = pipe.available(pipeOut);
    check('hmac_sha256 RFC 4231 #4: output 32 bytes', avail === 32);
    if (avail === 32) {
      pipe.read(pipeOut, SCRATCH + 200, 32);
      const expected = crypto.createHmac('sha256', key).update(msg).digest();
      let match = true;
      for (let i = 0; i < 32; i++) {
        if (mem[SCRATCH + 200 + i] !== expected[i]) { match = false; break; }
      }
      check('hmac_sha256 RFC 4231 #4: content matches Node crypto', match);
    }
  }

  console.log(`\n${passed}/${passed + failed} tests passed`);
  if (failed > 0) process.exit(1);
}

main().catch(e => console.error('Error:', e.message, e.stack));
