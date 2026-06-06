// Test: JIT compiler emits correct x86_64 machine code
// We load the interpreter, parse a simple WASM module,
// then compile via JIT and verify the emitted bytes.

const fs = require('fs');

async function main() {
  // Load interpreter (it creates its own memory)
  const interpObj = await WebAssembly.instantiate(
    fs.readFileSync('interpreter.wasm'),
    {}
  );
  const interp = interpObj.instance.exports;

  // Grow memory to cover JIT cache (0x100000) and JIT state (0x300000)
  const needed = Math.ceil((0x310000) / 65536);  // need 49+ pages
  while (interp.memory.buffer.byteLength < 0x310000) {
    interp.memory.grow(1);
  }

  // Load compiler, sharing interpreter's memory
  const compObj = await WebAssembly.instantiate(
    fs.readFileSync('compiler.wasm'),
    { env: { memory: interp.memory } }
  );
  const comp = compObj.instance.exports;

  const mem = new Uint8Array(interp.memory.buffer);

  function hex(bytes, len) {
    return Array.from({length: len}, (_, i) => bytes[i].toString(16).padStart(2, '0')).join(' ');
  }

  function loadWasm(wasmBytes) {
    const ptr = 0x200000;
    for (let i = 0; i < wasmBytes.length; i++) {
      mem[ptr + i] = wasmBytes[i];
    }
    const err = interp.load(ptr, wasmBytes.length);
    if (err !== 0) {
      console.log(`  LOAD FAILED: error code ${err}, debug=${mem[0x0F0000]}`);
      return false;
    }
    return true;
  }

  // Test 1: i32.const 42
  console.log('Test 1: i32.const 42');
  {
    const wasm = new Uint8Array([
      0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
      0x03, 0x02, 0x01, 0x00,
      0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2A, 0x0B,
    ]);

    if (!loadWasm(wasm)) return;

    const codeSize = comp.jit_compile(0);
    const ptr = 0x100000;
    const emitted = new Uint8Array(mem.slice(ptr, ptr + codeSize));
    console.log(`  Compiled: ${codeSize} bytes`);
    console.log(`  Emitted: ${hex(emitted, Math.min(codeSize, 32))}`);

    const pass1 = emitted[0] === 0x55 && emitted[1] === 0x48 &&
                  emitted[2] === 0x89 && emitted[3] === 0xE5;
    console.log(`  Prologue (55 48 89 E5): ${pass1}`);

    const hasPush42 = emitted[4] === 0x68 && emitted[5] === 42;
    console.log(`  push imm32(42): ${hasPush42}`);

    const hasEpilogue = emitted[codeSize-5] === 0x58 &&  // pop rax
                        emitted[codeSize-4] === 0x31 &&  // xor
                        emitted[codeSize-2] === 0x5D;    // pop rbp
    console.log(`  Epilogue: ${hasEpilogue}`);
  }

  // Test 2: i32.add
  console.log('\nTest 2: i32.add (3+5)');
  {
    const wasm = new Uint8Array([
      0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
      0x03, 0x02, 0x01, 0x00,
      0x0A, 0x09, 0x01, 0x07, 0x00, 0x41, 0x03, 0x41, 0x05, 0x6A, 0x0B,
    ]);

    if (!loadWasm(wasm)) return;

    const codeSize = comp.jit_compile(0);
    const ptr = 0x100000;
    const emitted = new Uint8Array(mem.slice(ptr, ptr + codeSize));
    console.log(`  Compiled: ${codeSize} bytes`);
    console.log(`  Emitted: ${hex(emitted, Math.min(codeSize, 48))}`);

    const hasPrologue = emitted[0] === 0x55 && emitted[1] === 0x48 && emitted[2] === 0x89 && emitted[3] === 0xE5;
    const hasPush3 = emitted.slice(4, 20).some((_, i) => emitted[4+i] === 0x68 && emitted[5+i] === 3);
    const hasPush5 = emitted.slice(4, 20).some((_, i) => emitted[4+i] === 0x68 && emitted[5+i] === 5);
    // i32.add = pop rcx (59), pop rax (58), add eax,ecx (01 C8), push rax (50)
    const hasAdd = emitted.slice(0, codeSize-5).some((_, i) =>
      emitted[i] === 0x59 && emitted[i+1] === 0x58 && emitted[i+2] === 0x01 && emitted[i+3] === 0xC8
    );
    console.log(`  Prologue: ${hasPrologue}, push 3: ${hasPush3}, push 5: ${hasPush5}, add: ${hasAdd}`);
  }

  // Test 3: local.get
  console.log('\nTest 3: local.get 0');
  {
    const wasm = new Uint8Array([
      0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
      0x01, 0x06, 0x01, 0x60, 0x01, 0x7F, 0x01, 0x7F,
      0x03, 0x02, 0x01, 0x00,
      0x0A, 0x06, 0x01, 0x04, 0x00, 0x20, 0x00, 0x0B,
    ]);

    if (!loadWasm(wasm)) return;

    const codeSize = comp.jit_compile(0);
    const ptr = 0x100000;
    const emitted = new Uint8Array(mem.slice(ptr, ptr + codeSize));
    console.log(`  Compiled: ${codeSize} bytes`);
    console.log(`  Emitted: ${hex(emitted, Math.min(codeSize, 48))}`);

    // local.get: mov rdx, [r15+0] (REX.W 8B 97 + dword), mov rax, [rdx+disp] (REX.W 8B 82 + dword), push rax (50)
    // or: REX.W 8B 04 24 [rsp] ... 
    const hasPush = emitted[codeSize-6] === 0x50 || emitted[codeSize-5] === 0x50;
    // look for REX.W (0x48) followed by 0x8B
    const hasRexMov = emitted.slice(4, codeSize-5).some((_, i) =>
      emitted[i] === 0x48 && emitted[i+1] === 0x8B
    );
    console.log(`  Has REX.W mov (local.load): ${hasRexMov}, push: ${hasPush}`);
  }

  console.log('\nAll tests done.');
}

main().catch(e => console.error(e));
