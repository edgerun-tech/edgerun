const fs = require('fs');

async function main() {
  const interp = (await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {})).instance.exports;
  const jitObj = await WebAssembly.instantiate(fs.readFileSync('jit.wasm'), { env: { memory: interp.memory } });
  const jit = jitObj.instance.exports;

  // Grow memory
  while (interp.memory.buffer.byteLength < 0x310000) interp.memory.grow(1);
  const v = new DataView(interp.memory.buffer);
  const mem = new Uint8Array(interp.memory.buffer);

  // Load minimal WASM: (module (func (result i32) i32.const 42))
  const wasm = new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2A, 0x0B,
  ]);
  const ptr = 0x200000;
  wasm.forEach((b,i) => mem[ptr+i] = b);
  let err = interp.load(ptr, wasm.length);
  console.log('load err:', err, 'dbg:', mem[0x0F0000]);

  // Read code_buf for func 0
  const cb = 21792;
  console.log('\ncode_buf[0]:');
  for (let i = 0; i < 16; i++) {
    console.log('  +' + (i*4) + ': ' + v.getUint32(cb + i*4, true));
  }

  // Read decoded ops
  const decStart = v.getUint32(cb + 24, true);
  const decCount = v.getUint32(cb + 32, true);
  console.log('\ndecoded_start:', decStart, 'decoded_count:', decCount);

  const decOps = 0xA0000;
  console.log('\ndecoded ops:');
  for (let i = 0; i < decCount; i++) {
    const dp = decOps + (decStart + i) * 16;
    console.log('  [' + i + '] offset=' + v.getUint32(dp, true) + ' next=' + v.getUint32(dp+4, true) +
      ' op=0x' + mem[dp+8].toString(16).padStart(2,'0') +
      ' imm0=' + v.getUint32(dp+12, true));
  }

  // Run JIT compile
  const offsetBefore = v.getUint32(89880, true);  // JIT_OFFSET global
  console.log('\nJIT_OFFSET before:', offsetBefore);
  
  const startTime = Date.now();
  const result = jit.jit_compile(0);
  const elapsed = Date.now() - startTime;
  
  const offsetAfter = v.getUint32(89880, true);
  console.log('JIT_OFFSET after:', offsetAfter);
  console.log('result (code size):', result);
  console.log('compile time:', elapsed, 'ms');

  // Dump emitted code
  const codePtr = 0x100000;
  console.log('\nEmitted bytes at 0x100000:');
  const len = Math.min(result, 64);
  const bytes = [];
  for (let i = 0; i < len; i++) bytes.push(mem[codePtr + i].toString(16).padStart(2,'0'));
  console.log('  ' + bytes.join(' '));
  if (result > 64) {
    const tail = [];
    for (let i = Math.max(64, result - 16); i < result; i++) tail.push(mem[codePtr + i].toString(16).padStart(2,'0'));
    console.log('  ... ' + tail.join(' '));
  }

  // Dump JIT state from memory
  console.log('\nJIT state:');
  const stateBase = 0x300000;
  console.log('  code_ptr: ' + v.getUint32(stateBase, true));
  console.log('  label_depth: ' + v.getUint32(stateBase + 4, true));
  console.log('  fixup_count: ' + v.getUint32(stateBase + 8, true));
  console.log('  return_emitted: ' + v.getUint32(stateBase + 12, true));
  console.log('  stack_depth: ' + v.getUint32(stateBase + 16, true));
  console.log('  max_stack: ' + v.getUint32(stateBase + 20, true));
  console.log('  cache_base: ' + v.getUint32(stateBase + 24, true));
  console.log('  cache_end: ' + v.getUint32(stateBase + 28, true));
  console.log('  func_idx: ' + v.getUint32(stateBase + 32, true));
  console.log('  result_count: ' + v.getUint32(stateBase + 36, true));
}

main().catch(e => console.error(e));
