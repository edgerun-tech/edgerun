const fs = require('fs');

async function main() {
  const interp = (await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {})).instance.exports;
  while (interp.memory.buffer.byteLength < 0x310000) interp.memory.grow(1);
  const v = new DataView(interp.memory.buffer);
  const mem = new Uint8Array(interp.memory.buffer);

  // Load minimal WASM
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

  // Verify decoded ops at their actual location (opcode at +0)
  const cb = 21792;
  const decStart = v.getUint32(cb + 24, true);
  const decCount = v.getUint32(cb + 32, true);
  console.log('decoded_start:', decStart, 'decoded_count:', decCount);
  for (let i = 0; i < decCount; i++) {
    const dp = 0xA0000 + (decStart + i) * 16;
    console.log('  [' + i + '] op=0x' + mem[dp].toString(16) + ' imm0=' + v.getUint32(dp+4, true));
  }

  // Check address 0 (JS_CODE_PTR) before JIT
  console.log('\nBefore JIT:');
  console.log('  addr[0]:', v.getUint32(0, true));
  console.log('  addr[4]:', v.getUint32(4, true));
  console.log('  addr[8]:', v.getUint32(8, true));

  // Now load JIT and run
  const jitObj = await WebAssembly.instantiate(fs.readFileSync('jit.wasm'), { env: { memory: interp.memory } });
  const jit = jitObj.instance.exports;

  console.log('\nJIT code_ptr state area (0x300000):');
  console.log('  state[0] (code_ptr):', v.getUint32(0x300000, true));
  console.log('  state[4] (cache_base):', v.getUint32(0x300004, true));

  console.log('\nRunning jit_compile(0)...');
  const result = jit.jit_compile(0);
  console.log('result:', result);
  
  console.log('\nAfter JIT:');
  console.log('  addr[0] (JS_CODE_PTR):', v.getUint32(0, true), '(should be code size)');
  console.log('  JIT state[0]:', v.getUint32(0x300000, true));
  console.log('  JIT state[4]:', v.getUint32(0x300004, true));
  console.log('  JIT state[20] stack_depth:', v.getUint32(0x300000+20, true));
  console.log('  JIT state[24] max_stack:', v.getUint32(0x300000+24, true));
  console.log('  JIT state[28] label_depth:', v.getUint32(0x300000+28, true));

  // Dump first 32 bytes of output
  console.log('\nEmitted (first 32):');
  for (let i = 0; i < 32; i++) process.stdout.write(mem[0x100000 + i].toString(16).padStart(2,'0') + ' ');
  console.log();
  
  // Find epilogue
  for (let i = result - 10; i < result; i++) {
    process.stdout.write(mem[0x100000 + i].toString(16).padStart(2,'0') + ' ');
  }
  console.log();
  
  // Count ud2 instructions
  let ud2count = 0;
  for (let i = 10; i < result - 10; i += 2) {
    if (mem[0x100000 + i] === 0x0F && mem[0x100000 + i + 1] === 0x0B) ud2count++;
    else break;
  }
  console.log('\nud2 count in middle:', ud2count);
}

main().catch(e => console.error(e));
