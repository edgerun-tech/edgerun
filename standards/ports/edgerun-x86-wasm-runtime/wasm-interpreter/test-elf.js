const fs = require('fs');
const { execSync } = require('child_process');

async function main() {
  const interpObj = await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {});
  const interp = interpObj.instance.exports;
  while (interp.memory.buffer.byteLength < 0x810000) interp.memory.grow(1);

  const compObj = await WebAssembly.instantiate(fs.readFileSync('compiler.wasm'), { env: { memory: interp.memory } });
  const comp = compObj.instance.exports;
  const mem = new Uint8Array(interp.memory.buffer);

  function load(wasm) {
    for (let i = 0; i < wasm.length; i++) mem[0x200000 + i] = wasm[i];
    return interp.load(0x200000, wasm.length) === 0;
  }

  function hex(arr, n) {
    return Array.from({length: n || arr.length}, (_, i) => arr[i].toString(16).padStart(2, '0')).join(' ');
  }

  let passed = 0, failed = 0;

  function test(name, wasm, expectedExit) {
    if (!load(wasm)) { console.log(`FAIL ${name}: load error`); failed++; return; }
    const [elfAddr, elfSize] = comp.compile_to_elf(0);
    const elf = new Uint8Array(mem.slice(elfAddr, elfAddr + elfSize));
    const path = `/tmp/test_elf_${name.replace(/[^a-zA-Z0-9]/g, '_')}`;
    fs.writeFileSync(path, elf);
    fs.chmodSync(path, 0o755);
    try {
      let actualExit;
      try {
        execSync(path, { stdio: 'ignore', timeout: 5000 });
        actualExit = 0;
      } catch (e) {
        actualExit = e.status !== undefined ? e.status : -1;
      }
      if (actualExit === expectedExit) {
        console.log(`PASS ${name} (${elfSize}B, exit=${actualExit})`);
        passed++;
      } else {
        console.log(`FAIL ${name}: expected exit ${expectedExit}, got ${actualExit}`);
        console.log(`  ELF: ${hex(elf, Math.min(elfSize, 64))}`);
        failed++;
      }
    } finally {
      try { fs.unlinkSync(path); } catch(_) {}
    }
  }

  // Test 1: i32.const 42
  test('const_42', new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2A, 0x0B,
  ]), 42);

  // Test 2: i32.const 0 + return
  test('const_0', new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x00, 0x0B,
  ]), 0);

  // Test 3: i32.const -1
  test('const_neg1', new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x0A, 0x06, 0x01, 0x04, 0x00, 0x41, 0x7F, 0x0B,
  ]), 255);  // 255 because exit code is unsigned byte

  // Test 4: i32.const 255
  test('const_255', new Uint8Array([
    0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7F,
    0x03, 0x02, 0x01, 0x00,
    0x0A, 0x07, 0x01, 0x05, 0x00, 0x41, 0xFF, 0x01, 0x0B,
  ]), 255);

  console.log(`\n${passed}/${passed+failed} passed (${failed} failed)`);
}

main().catch(e => console.error(e));
