const fs = require('fs');
const path = require('path');

const corePath = path.join(__dirname, '../../../system/runtime/edgerun-core.wasm');
const interpreterPath = path.join(__dirname, 'interpreter.wasm');

async function main() {
  // Instantiate edgerun-core first to get the shared memory
  const coreWasm = fs.readFileSync(corePath);
  const coreMod = new WebAssembly.Module(coreWasm);
  const core = await WebAssembly.instantiate(coreMod, {});
  
  const memory = core.exports.memory;
  const mem = new Uint8Array(memory.buffer);

  const g = (name) => {
    const v = core.exports[name];
    if (typeof v === 'object' && v && v.value !== undefined) return v.value;
    if (typeof v === 'function') return v();
    return v;
  };
  
  const interpWasm = fs.readFileSync(interpreterPath);
  const interpMod = new WebAssembly.Module(interpWasm);
  const interp = await WebAssembly.instantiate(interpMod, {
    'edgerun-core': {
      memory: memory,
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
  
  console.log('Interpreter exports:', Object.keys(interp.exports).join(', '));
  
  // Check if load_wat exists
  if (!interp.exports.load_wat) {
    console.log('ERROR: load_wat not exported');
    return;
  }

  // Test 1: Minimal module
  const wat = `(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add
  )
)`;
  
  const watBuf = new TextEncoder().encode(wat);
  const watPtr = 0x100000;
  for (let i = 0; i < watBuf.length; i++) {
    mem[watPtr + i] = watBuf[i];
  }
  mem[watPtr + watBuf.length] = 0;
  
  console.log('\nWAT (' + watBuf.length + ' bytes):');
  console.log(new TextDecoder().decode(mem.subarray(watPtr, watPtr + watBuf.length)));
  
  const result = interp.exports.load_wat(watPtr, watBuf.length);
  console.log('load_wat result:', result, result === 0 ? '(OK)' : '(ERROR ' + result + ')');
  
  // Read module counters at known offsets
  // Looking at $wat_parse_module to find the memory locations
  // OFF_TYPE_COUNT, OFF_FUNCTION_COUNT, etc should be at 0x10004, 0x10010, 0x1001C
  // but let's dump what's there
  const dv = new DataView(memory.buffer);
  console.log('\nModule state at known locations (little-endian u32):');
  for (let off = 0x10000; off < 0x10040; off += 4) {
    const val = dv.getUint32(off, true);
    if (val !== 0) {
      console.log(`  [0x${off.toString(16)}] = ${val}`);
    }
  }
  
  // Check after restoration - wasm ptr should be back at 0
  // OFF_WASM_PTR is at... let me find it
}

main().catch(e => console.error('Error:', e.message, e.stack));
