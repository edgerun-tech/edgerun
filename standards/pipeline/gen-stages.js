#!/usr/bin/env node
// Pipeline stage generator — reads registry.json, writes *-stage.wat files.
// Usage: node gen-stages.js
// Stages with "generated": false are skipped (hand-written).
const fs = require('fs');
const path = require('path');

const REGISTRY = JSON.parse(fs.readFileSync(path.join(__dirname, 'registry.json'), 'utf8'));

function wrap(slot, name, locals, read_check, body, out_write, return_expr) {
  return `;; Process ${name} Stage — slot ${slot}
  (func $process_${name} (export "process_${name}")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    ${locals}
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    ${read_check}
    ${body}
    ${out_write}
    ${return_expr})`;
}

const TMPLS = {
  // Core fn returns i32 status (0=ok), output is constant size
  scan(st) {
    const { slot, name, fn, out_size, min_read, args, out_buf, checks } = st;
    const mr = min_read || 1;
    const read_check = checks === 'eqz' || mr === 1
      ? '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))'
      : `(if (i32.lt_u (local.get $read) (i32.const ${mr})) (then (return (i32.const 0))))`;
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
    const ob = out_buf || 'local.get $scratch';
    return wrap(slot, name,
      '(local $status i32)',
      read_check,
      `(local.set $status (call $${fn} (${a})))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))`,
      `(drop (call $pipe_write (local.get $output) (${ob}) (i32.const ${out_size})))`,
      `i32.const ${out_size}`);
  },

  // Core fn returns written length, uses stage_write_output helper
  transform(st) {
    const { slot, name, fn, args } = st;
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
    const re = '(return (call $stage_write_output (local.get $output) (local.get $scratch)\n      (call $' + fn + ' (' + a + '))))';
    return wrap(slot, name,
      '',
      `(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))`,
      '',
      '',
      re);
  },

  // Core fn returns i64 packed (status=hi32, written=lo32)
  decode(st) {
    const { slot, name, fn, out_buf, args, prep } = st;
    const ob = out_buf || 'local.get $scratch';
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch', '(i32.sub (local.get $scap) (i32.const 8))']).join(') (');
    const prep_lines = prep ? prep.map(l => `    ${l}`).join('\n') + '\n' : '';
    return wrap(slot, name,
      '(local $read i32) (local $result i64) (local $status i32) (local $written i32)',
      '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
      `${prep_lines}    (local.set $result (call $${fn} (${a})))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $written (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))`,
      `(drop (call $pipe_write (local.get $output) (${ob}) (local.get $written)))`,
      `local.get $written`);
  },

  // Hand-specified body
  raw(st) { return st.body; },
  custom(st) { return st.body; }
};

let count = 0;
for (const st of REGISTRY) {
  if (st.generated === false) continue;
  const body = TMPLS[st.pattern](st);
  const fname = st.name.replace(/_/g, '-');
  fs.writeFileSync(path.join(__dirname, `${fname}-stage.wat`), '\n' + body + '\n');
  count++;
}
console.log(`Generated ${count} stage files`);
