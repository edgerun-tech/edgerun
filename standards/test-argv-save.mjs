import { readFileSync, writeFileSync } from 'fs';

const wasm = readFileSync(new URL('./edgerun.wasm', import.meta.url));
const module = await WebAssembly.compile(wasm);
const instance = await WebAssembly.instantiate(module, {});

const { memory, _start, compile_all_to_elf_x86_64 } = instance.exports;

// A minimal WASM module that exports _start
const helloWasm = new Uint8Array([
  0x00, 0x61, 0x73, 0x6D, // magic \0asm
  0x01, 0x00, 0x00, 0x00, // version 1
  // type section: (func)
  0x01, 0x04, 0x01, 0x60, 0x00, 0x00,
  // function section: 1 func, type 0
  0x03, 0x02, 0x01, 0x00,
  // export section: export "_start" as func 0
  0x07, 0x0A, 0x01, 0x06, 0x5F, 0x73, 0x74, 0x61, 0x72, 0x74, 0x00, 0x00,
  // code section: 1 code, size=2, locals=0, body=0B
  0x0A, 0x04, 0x01, 0x02, 0x00, 0x0B,
]);

const buf = new Uint8Array(memory.buffer);
const wasmAddr = 0x800000;
for (let i = 0; i < helloWasm.length; i++) {
  buf[wasmAddr + i] = helloWasm[i];
}
const view = new DataView(memory.buffer);
view.setUint32(0x800000, wasmAddr, true);
view.setUint32(0x800004, helloWasm.length, true);

// Call _start which loads + compiles to ELF
try {
  _start();
  const elfAddr = view.getUint32(0x800008, true);
  const elfSize = view.getUint32(0x80000C, true);
  console.log('_start returned OK');
  console.log('ELF at 0x' + elfAddr.toString(16) + ', size ' + elfSize);

  const elf = new Uint8Array(memory.buffer.slice(elfAddr, elfAddr + elfSize));
  writeFileSync('/tmp/test-argv.elf', elf);

  // Verify argv save opcodes
  let found = 0;
  for (let i = 0; i < elf.length - 12; i++) {
    if (elf[i] === 0x59 && // pop rcx
        elf[i+1] === 0x49 && elf[i+2] === 0x89 && elf[i+3] === 0x4F && elf[i+4] === 0x48 && // mov [r15+72], rcx
        elf[i+5] === 0x48 && elf[i+6] === 0x89 && elf[i+7] === 0xE0 && // mov rax, rsp
        elf[i+8] === 0x49 && elf[i+9] === 0x89 && elf[i+10] === 0x47 && elf[i+11] === 0x50) { // mov [r15+80], rax
      console.log('✓ argv save opcodes at ELF offset 0x' + i.toString(16));
      found = 1;
      break;
    }
  }
  if (!found) console.log('✗ argv save opcodes NOT found');
} catch (e) {
  console.error('_start failed:', e.message);
}
