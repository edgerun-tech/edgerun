#!/usr/bin/env node
/**
 * EdgeRun JIT Compiler Generator
 *
 * Reads a per-architecture KV template (JSON) and the unified compiler.wat
 * template, then generates a complete arch-specific jit-dispatch WAT file.
 *
 * Usage:
 *   node tools/gen_compiler.js templates/x86_64.json > gen/jit-dispatch-x86-64.wat
 *   node tools/gen_compiler.js templates/aarch64.json > gen/jit-dispatch-aarch64.wat
 *   node tools/gen_compiler.js templates/arm32.json > gen/jit-dispatch-arm32.wat
 */

const fs = require('fs');
const path = require('path');

function buildOpTable(ops, prefix, suffix) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  const lines = [];
  for (const [hexcode, opname] of entries) {
    const funcname = `${prefix}${opname}${suffix}`;
    const code = parseInt(hexcode, 16);
    if (code === 0x00) {
      lines.push(
        `          (if (i32.eqz (local.get $opcode))` +
        `\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`
      );
    } else {
      lines.push(
        `          (if (i32.eq (local.get $opcode) (i32.const ${hexcode}))` +
        `\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`
      );
    }
  }
  return lines.join('\n');
}

function buildSubTable(ops, prefix, suffix, tableLabel) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  const lines = [];
  for (const [hexcode, opname] of entries) {
    const funcname = `${prefix}${opname}${suffix}`;
    lines.push(
      `                (if (i32.eq (local.get $imm0) (i32.const ${hexcode}))` +
      `\n                  (then (call $${funcname} (local.get $dec_ptr)) (br $${tableLabel})))`
    );
  }
  return lines.join('\n');
}

function generate(tmplPath, compilerPath) {
  const tmpl = JSON.parse(fs.readFileSync(tmplPath, 'utf8'));
  let wat = fs.readFileSync(compilerPath, 'utf8');

  const arch = tmpl.arch;
  const suffix = `_${arch}`;
  const opPrefix = tmpl.op_prefix;
  const opSuffix = tmpl.op_suffix;
  const simdPrefix = tmpl.simd_prefix !== undefined ? tmpl.simd_prefix : opPrefix;
  const simdSuffix = tmpl.simd_suffix !== undefined ? tmpl.simd_suffix : opSuffix;

  const opTable = buildOpTable(tmpl.ops, opPrefix, opSuffix);
  const fcTable = buildSubTable(tmpl.fc_ops || {}, opPrefix, opSuffix, 'fc_done');
  const fdTable = buildSubTable(tmpl.fd_ops || {}, simdPrefix, simdSuffix, 'fd_done');

  const subs = {
    '{SUFFIX}': suffix,
    '{OP_TABLE}': opTable,
    '{FC_TABLE}': fcTable,
    '{FD_TABLE}': fdTable,
    '{PROLOGUE}': tmpl.prologue,
    '{EPILOGUE}': tmpl.epilogue,
    '{RESULT_GLOBAL}': tmpl.result_global,
    '{NEXT_OP_GLOBAL}': tmpl.next_op_global,
    '{JIT_ERROR_GLOBAL}': tmpl.jit_error_global,
    '{CODE_PTR_GLOBAL}': tmpl.code_ptr_global,
    '{LABEL_DEPTH_GLOBAL}': tmpl.label_depth_global,
    '{FUNC_OFF_TABLE}': tmpl.func_off_table,
    '{ARCH}': arch,
  };

  for (const [placeholder, value] of Object.entries(subs)) {
    wat = wat.split(placeholder).join(value);
  }

  return wat;
}

function main() {
  if (process.argv.length < 3) {
    console.error(`Usage: ${process.argv[1]} <template.json>`);
    process.exit(1);
  }

  const tmplPath = path.resolve(process.argv[2]);
  const compilerDir = path.resolve(path.dirname(process.argv[1]), '..');
  const compilerWat = path.join(compilerDir, 'compiler.wat');

  if (!fs.existsSync(compilerWat)) {
    console.error(`Error: ${compilerWat} not found`);
    process.exit(1);
  }

  const result = generate(tmplPath, compilerWat);
  console.log(result);
}

main();
