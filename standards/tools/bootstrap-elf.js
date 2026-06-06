const fs = require('fs');
const path = require('path');

const DIR = path.join(__dirname, '..', 'ports', 'wasm-tooling', 'wasm-interpreter');
const CORE = path.join(__dirname, '..', 'system', 'runtime', 'edgerun-core.wasm');
const INTERP = path.join(DIR, 'interpreter.wasm');
const COMPILER = path.join(DIR, 'compiler-x86_64.wasm');

async function main() {
  const targetWasmPath = process.argv[2];
  const outputPath = process.argv[3] || 'a.out';
  const funcIdx = parseInt(process.argv[4] || '2', 10); // default: func 2 (first local after imports)

  if (!targetWasmPath) {
    console.error('Usage: node bootstrap-elf.js <target.wasm> [output.elf] [func_idx]');
    process.exit(1);
  }

  // 1. Load edgerun-core (shared memory + constants)
  const coreMod = await WebAssembly.instantiate(
    new WebAssembly.Module(fs.readFileSync(CORE)), {}
  );
  const core = coreMod.exports;
  const mem = new Uint8Array(core.memory.buffer);
  const dv = new DataView(core.memory.buffer);

  // Helper to get exported global values
  const g = (name) => {
    const v = core[name];
    if (typeof v === 'function') return v();
    if (v && v.value !== undefined) return v.value;
    return v;
  };

  // 2. Instantiate interpreter with shared memory
  const interpMod = new WebAssembly.Module(fs.readFileSync(INTERP));
  const interp = await WebAssembly.instantiate(interpMod, {
    'edgerun-core': {
      memory: core.memory,
      OFF_TYPES_BUF: g('OFF_TYPES_BUF'),
      OFF_CODE_BUF: g('OFF_CODE_BUF'),
      OFF_FUNCTIONS_BUF: g('OFF_FUNCTIONS_BUF'),
      OFF_DECODED_OPS: g('OFF_DECODED_OPS'),
      OFF_DECODED_COUNT: g('OFF_DECODED_COUNT'),
      DEC_SZ: g('DEC_SZ'),
      SZ_TYPE: g('SZ_TYPE'),
      SZ_FUNC: g('SZ_FUNC'),
      SZ_CODE: g('SZ_CODE'),
    }
  });

  // 3. Load target WASM binary into shared memory
  const wasmBytes = fs.readFileSync(targetWasmPath);
  const WASM_ADDR = 0x100000;
  for (let i = 0; i < wasmBytes.length; i++) {
    mem[WASM_ADDR + i] = wasmBytes[i];
  }
  console.log(`Loaded ${wasmBytes.length} bytes at 0x${WASM_ADDR.toString(16)}`);

  // 4. Decode via interpreter
  const loadResult = interp.exports.load(WASM_ADDR, wasmBytes.length);
  if (loadResult !== 0) {
    console.error(`Interpreter load() failed: ${loadResult}`);
    process.exit(1);
  }
  console.log('Module decoded OK');

  // 5. Instantiate JIT compiler with shared memory
  const compilerMod = new WebAssembly.Module(fs.readFileSync(COMPILER));
  const compiler = await WebAssembly.instantiate(compilerMod, {
    'edgerun-core': {
      memory: core.memory,
      OFF_TYPES_BUF: g('OFF_TYPES_BUF'),
      OFF_CODE_BUF: g('OFF_CODE_BUF'),
      OFF_FUNCTIONS_BUF: g('OFF_FUNCTIONS_BUF'),
      OFF_DECODED_OPS: g('OFF_DECODED_OPS'),
      OFF_DECODED_COUNT: g('OFF_DECODED_COUNT'),
      DEC_SZ: g('DEC_SZ'),
      SZ_TYPE: g('SZ_TYPE'),
      SZ_FUNC: g('SZ_FUNC'),
      SZ_CODE: g('SZ_CODE'),
    }
  });

  // 6. Compile to ELF
  const [bufAddr, totalSize] = compiler.exports.compile_to_elf(funcIdx);
  console.log(`ELF produced: buf=0x${bufAddr.toString(16)}, size=${totalSize} bytes`);

  // 7. Write ELF to file
  const elfBytes = Buffer.from(mem.slice(bufAddr, bufAddr + totalSize));
  fs.writeFileSync(outputPath, elfBytes);
  fs.chmodSync(outputPath, 0o755);
  console.log(`Written: ${outputPath} (${elfBytes.length} bytes, executable)`);
}

main().catch(e => {
  console.error('Error:', e.message, e.stack);
  process.exit(1);
});
