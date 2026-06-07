#!/usr/bin/env node
const fs = require('fs');
const path = require('path');

function buildOpTable(ops, prefix, suffix) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  return entries.map(([hexcode, opname]) => {
    const funcname = `${prefix}${opname}${suffix}`;
    const code = parseInt(hexcode, 16);
    if (code === 0x00)
      return `          (if (i32.eqz (local.get $opcode))\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`;
    return `          (if (i32.eq (local.get $opcode) (i32.const ${hexcode}))\n            (then (call $${funcname} (local.get $dec_ptr)) (br $dispatch_done)))`;
  }).join('\n');
}

function buildSubTable(ops, prefix, suffix, label) {
  const entries = Object.entries(ops).sort((a, b) => parseInt(a[0]) - parseInt(b[0]));
  return entries.map(([hexcode, opname]) => {
    const funcname = `${prefix}${opname}${suffix}`;
    return `                (if (i32.eq (local.get $imm0) (i32.const ${hexcode}))\n                  (then (call $${funcname} (local.get $dec_ptr)) (br $${label})))`;
  }).join('\n');
}

function generate(tmplPath, compilerPath) {
  const tmpl = JSON.parse(fs.readFileSync(tmplPath, 'utf8'));
  let wat = fs.readFileSync(compilerPath, 'utf8');
  const arch = tmpl.arch;
  const opPrefix = tmpl.op_prefix;
  const opSuffix = tmpl.op_suffix;
  const simdPrefix = tmpl.simd_prefix !== undefined ? tmpl.simd_prefix : opPrefix;
  const simdSuffix = tmpl.simd_suffix !== undefined ? tmpl.simd_suffix : opSuffix;
  const opTable = buildOpTable(tmpl.ops, opPrefix, opSuffix);
  const fcTable = buildSubTable(tmpl.fc_ops || {}, opPrefix, opSuffix, 'fc_done');
  const fdTable = buildSubTable(tmpl.fd_ops || {}, simdPrefix, simdSuffix, 'fd_done');
  const subs = {
    '{SUFFIX}': `_${arch}`,
    '{ARCH}': arch,
    '{OP_TABLE}': opTable,
    '{FC_TABLE}': fcTable,
    '{FD_TABLE}': fdTable,
    '{PROLOGUE}': tmpl.prologue,
    '{EPILOGUE}': tmpl.epilogue,
    '{EMIT_BYTE}': tmpl.emit_byte,
    '{EMIT_DWORD}': tmpl.emit_dword,
    '{EMIT_ELF_STUB}': tmpl.emit_elf_stub,
    '{EMIT_ELF64_EHDR}': tmpl.emit_elf64_ehdr,
    '{EMIT_ELF64_PHDR}': tmpl.emit_elf64_phdr,
    '{FIXUP_CALLS}': tmpl.fixup_calls,
    '{COPY_COMPILED_CODE}': tmpl.copy_compiled_code,
    '{RESULT_GLOBAL}': tmpl.result_global,
    '{NEXT_OP_GLOBAL}': tmpl.next_op_global,
    '{JIT_ERROR_GLOBAL}': tmpl.jit_error_global,
    '{CODE_PTR_GLOBAL}': tmpl.code_ptr_global,
    '{LABEL_DEPTH_GLOBAL}': tmpl.label_depth_global,
    '{FUNC_OFF_TABLE}': tmpl.func_off_table,
  };
  for (const [k, v] of Object.entries(subs)) wat = wat.split(k).join(v);
  return wat;
}

function main() {
  if (process.argv.length < 3) { console.error(`Usage: ${process.argv[1]} <template.json>`); process.exit(1); }
  const tmplPath = path.resolve(process.argv[2]);
  const dir = path.resolve(path.dirname(process.argv[1]), '..');
  const compiler = path.join(dir, 'compiler.wat');
  if (!fs.existsSync(compiler)) { console.error(`${compiler} not found`); process.exit(1); }
  console.log(generate(tmplPath, compiler));
}

main();
