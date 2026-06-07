#!/usr/bin/env node
// Pipeline stage generator — reads registry.json, writes *-stage.wat files.
// Usage: node gen-stages.js
// Stages with "generated": false are skipped (hand-written).
const fs = require('fs');
const path = require('path');

const REGISTRY = JSON.parse(fs.readFileSync(path.join(__dirname, 'registry.json'), 'utf8'));

// Generate cfg_reads local declarations and setup code
function genCfgReads(cfg_reads) {
  if (!cfg_reads || !cfg_reads.length) return { locals: '', setup: '' };
  const locs = cfg_reads.map(c =>
    `    (local \$${c.name} ${c.type || 'i32'})`
  ).join('\n');
  const setup = cfg_reads.map(c => {
    let s = '';
    if (c.default !== undefined) {
      s += `    (local.set \$${c.name} (${c.type || 'i32'}.const ${c.default}))\n`;
    }
    s += `    (if (i32.ge_u (local.get $clen) (i32.const ${c.offset + 4 || 4})) (then (local.set \$${c.name} (${c.type || 'i32'}.load${c.offset ? ` offset=${c.offset}` : ''} (local.get $cfg)))))`;
    return s;
  }).join('\n');
  return { locals: locs, setup };
}

function wrap(slot, name, locals, read_check, body, out_write, return_expr) {
  const hasRead = locals.includes('(local $read ');
  return `;; Process ${name} Stage — slot ${slot}
  (func $process_${name} (export "process_${name}")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    ${locals}${hasRead ? '' : '\n    (local $read i32)'}
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    ${read_check}
    ${body}
    ${out_write}
    ${return_expr})`;
}

const TMPLS = {
  // Core fn returns i32 status (0=ok), output is constant size
  scan(st) {
    const { slot, name, fn, out_size, min_read, args, out_buf, checks, cfg_reads, extra_locals, prep } = st;
    const mr = min_read || 1;
    const read_check = checks === 'eqz' || mr === 1
      ? '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))'
      : `(if (i32.lt_u (local.get $read) (i32.const ${mr})) (then (return (i32.const 0))))`;
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
    const ob = out_buf || 'local.get $scratch';
    const cfg = genCfgReads(cfg_reads);
    const xtra = extra_locals ? '\n    ' + extra_locals.join('\n    ') : '';
    const prep_lines = prep ? prep.map(l => `    ${l}`).join('\n') + '\n' : '';
    return wrap(slot, name,
      `(local $status i32)${cfg.locals}${xtra}`,
      read_check,
      `${cfg.setup}${prep_lines}    (local.set $status (call $${fn} (${a})))
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
    const { slot, name, fn, out_buf, args, prep, cfg_reads, extra_locals } = st;
    const ob = out_buf || 'local.get $scratch';
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch', '(i32.sub (local.get $scap) (i32.const 8))']).join(') (');
    const prep_lines = prep ? prep.map(l => `    ${l}`).join('\n') + '\n' : '';
    const cfg = genCfgReads(cfg_reads);
    const xtra = extra_locals ? '\n    ' + extra_locals.join('\n    ') : '';
    return wrap(slot, name,
      `(local $read i32) (local $result i64) (local $status i32) (local $written i32)${cfg.locals}${xtra}`,
      '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
      `${cfg.setup}${prep_lines}    (local.set $result (call $${fn} (${a})))
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (local.set $written (i32.wrap_i64 (local.get $result)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))`,
      `(drop (call $pipe_write (local.get $output) (${ob}) (local.get $written)))`,
      `local.get $written`);
  },

  // Core fn returns i32 value stored to global output buffer via i32.store/i32.store16
  store(st) {
    const { slot, name, fn, out_buf, out_size, store_type, args } = st;
    const ob = out_buf || 'global.get $SHA256_OUT_BUF';
    const stype = store_type || 'i32.store';
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read']).join(') (');
    const rv = stype === 'i32.store16' ? '(local $val i32)' : '(local $val i32)';
    return wrap(slot, name,
      rv,
      '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
      `(local.set $val (call $${fn} (${a})))
    (${stype} (${ob}) (local.get $val))`,
      `(drop (call $pipe_write (local.get $output) (${ob}) (i32.const ${out_size || 4})))`,
      `i32.const ${out_size || 4}`);
  },

  // Core fn returns written length directly; negative = error
  map(st) {
    const { slot, name, fn, args, out_buf } = st;
    const ob = out_buf || 'local.get $scratch';
    const a = (args || ['global.get $SCRATCH_BUF', 'local.get $read', 'local.get $scratch']).join(') (');
    return wrap(slot, name,
      '(local $out_len i32)',
      '(if (i32.eqz (local.get $read)) (then (return (i32.const 0))))',
      `(local.set $out_len (call $${fn} (${a})))
    (if (i32.le_s (local.get $out_len) (i32.const 0)) (then (return (i32.const 0))))`,
      `(drop (call $pipe_write (local.get $output) (${ob}) (local.get $out_len)))`,
      `local.get $out_len`);
  },

  // Hand-specified body
  raw(st) { return st.body; },
  custom(st) { return st.body; }
};

let count = 0;
for (const st of REGISTRY) {
  if (st.generated === false) continue;
  const t = TMPLS[st.pattern];
  if (!t) { console.error(`Unknown pattern "${st.pattern}" for ${st.name}, skipping`); continue; }
  const body = t(st);
  const fname = st.name.replace(/_/g, '-');
  fs.writeFileSync(path.join(__dirname, `${fname}-stage.wat`), '\n' + body + '\n');
  count++;
}
console.log(`Generated ${count} stage files`);
