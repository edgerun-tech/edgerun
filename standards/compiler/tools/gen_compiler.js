#!/usr/bin/env node
// EdgeRun JIT Compiler Generator
// Usage:
//   node tools/gen_compiler.js templates/x86_64.json              # dispatch only → stdout
//   node tools/gen_compiler.js --assemble templates/x86_64.json   # full JIT → gen/jit-full-*.wat
//   node tools/gen_compiler.js --assemble all                     # all 3 architectures
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

function assemble(arch, dir, tmplPath) {
  const dispatch = generate(tmplPath, path.join(dir, 'compiler.wat'));
  const genDir = path.join(dir, 'gen');

  // Normalize arch for file names (x86_64 → x86-64)
  const fa = arch.replace(/_/g, '-');

  // Read fragment files
  const memoryMap = fs.readFileSync(path.join(dir, '..', 'runtime', 'memory-map.wat'), 'utf8');
  const emitCore = fs.readFileSync(path.join(dir, 'emit-core.wat'), 'utf8');
  const emit = fs.readFileSync(path.join(dir, `emit-${fa}.wat`), 'utf8');
  let templates = fs.readFileSync(path.join(dir, `templates-${fa}.wat`), 'utf8');
  const simd = fs.readFileSync(path.join(dir, `simd-${fa}.wat`), 'utf8');
  const wasmEmit = fs.readFileSync(path.join(dir, 'wasm-emit.wat'), 'utf8');

  // Strip (module ... ) wrapper from templates (it'\''s a fragment inside our module)
  if (templates.trimStart().startsWith('(module')) {
    templates = templates.replace(/^\(module\s*\n/, '');
    if (templates.endsWith(')\n')) templates = templates.slice(0, -2);
    else if (templates.endsWith(')')) templates = templates.slice(0, -1);
  }

  const parts = [memoryMap, dispatch, emitCore, emit, templates, simd, wasmEmit];
  const full = `(module\n${parts.join('\n\n')})\n`;

  if (!fs.existsSync(genDir)) fs.mkdirSync(genDir, { recursive: true });
  const outPath = path.join(genDir, `jit-full-${fa}.wat`);
  fs.writeFileSync(outPath, full);
  console.log(`Assembled: ${outPath} (${full.length} bytes)`);
}

function main() {
  const args = process.argv.slice(2);
  const dir = path.resolve(path.dirname(process.argv[1]), '..');
  const compiler = path.join(dir, 'compiler.wat');
  if (!fs.existsSync(compiler)) { console.error(`${compiler} not found`); process.exit(1); }

  if (args[0] === '--assemble') {
    const target = args[1];
    const archs = ['x86-64', 'aarch64', 'arm32'];
    if (target === 'all') {
      for (const arch of archs) {
        const tmplPath = path.join(dir, 'templates', `${arch.replace('-', '_')}.json`);
        if (fs.existsSync(tmplPath)) assemble(arch, dir, tmplPath);
      }
    } else {
      const tmplPath = path.resolve(target);
      if (!fs.existsSync(tmplPath)) { console.error(`Template not found: ${tmplPath}`); process.exit(1); }
      const tmpl = JSON.parse(fs.readFileSync(tmplPath, 'utf8'));
      assemble(tmpl.arch, dir, tmplPath);
    }
    return;
  }

  // Legacy mode: print dispatch to stdout
  if (args.length < 1) { console.error(`Usage: ${process.argv[1]} [--assemble <template.json>|all]`); process.exit(1); }
  const tmplPath = path.resolve(args[0]);
  process.stdout.write(generate(tmplPath, compiler));
}

main();
