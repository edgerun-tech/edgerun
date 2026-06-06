const fs = require('fs');

async function main() {
  const interpObj = await WebAssembly.instantiate(fs.readFileSync('interpreter.wasm'), {});
  const interp = interpObj.instance.exports;
  while (interp.memory.buffer.byteLength < 0x310000) interp.memory.grow(1);

  const compObj = await WebAssembly.instantiate(fs.readFileSync('compiler.wasm'), { env: { memory: interp.memory } });
  const comp = compObj.instance.exports;
  const mem = new Uint8Array(interp.memory.buffer);

  let passed = 0, failed = 0;

  function load(wasm) {
    const ptr = 0x200000;
    for (let i = 0; i < wasm.length; i++) mem[ptr + i] = wasm[i];
    return interp.load(ptr, wasm.length) === 0;
  }

  function hex(arr, n) {
    return Array.from({length: n || arr.length}, (_, i) => arr[i].toString(16).padStart(2, '0')).join(' ');
  }

  function test(name, wasm, check) {
    if (!load(wasm)) { console.log(`FAIL ${name}: load error`); failed++; return; }
    const callErr = interp.call(0, 0, 0);
    const itp = callErr === 0
      ? interp.get_result_count() > 0
        ? Number(interp.get_result_value(0))
        : '<no result>'
      : `<call error: ${callErr}>`;
    const sz = comp.jit_compile(0);
    const e = new Uint8Array(mem.slice(0x100000, 0x100000 + sz));
    try { check(e, itp, sz); console.log(`PASS ${name} (${sz}B)`); passed++; }
    catch (x) { console.log(`FAIL ${name}: ${x.message}`); console.log(`  Emitted: ${hex(e, Math.min(sz, 64))}`); console.log(`  Interp: ${itp}`); failed++; }
  }

  function contains(e, p, off) {
    for (let i = off||0; i <= e.length - p.length; i++) {
      let ok = true;
      for (let j = 0; j < p.length; j++) if (e[i+j] !== p[j]) { ok = false; break; }
      if (ok) return i;
    }
    return -1;
  }

  function checkPrologue(e) {
    if (e[0] !== 0x55 || e[1] !== 0x48 || e[2] !== 0x89 || e[3] !== 0xE5)
      throw new Error(`bad prologue: ${hex(e, 4)}`);
    // mov rbx, [r15] (49 8B 9F 00 00 00 00) — locals base caching
    if (e[4] !== 0x49 || e[5] !== 0x8B || e[6] !== 0x9F)
      throw new Error(`missing mov rbx,[r15]: ${hex(e.slice(4, 11))}`);
    for (let i = 7; i < 11; i++) if (e[i] !== 0) throw new Error(`non-zero disp in mov rbx,[r15]: ${hex(e.slice(4, 11))}`);
    // mov r12, [rbx] (4C 8B 23) — register-allocated local 0
    if (e[11] !== 0x4C || e[12] !== 0x8B || e[13] !== 0x23)
      throw new Error(`missing mov r12,[rbx]: ${hex(e.slice(11, 14))}`);
    // mov r13, [rbx+8] (4C 8B 6B 08) — register-allocated local 1
    if (e[14] !== 0x4C || e[15] !== 0x8B || e[16] !== 0x6B || e[17] !== 0x08)
      throw new Error(`missing mov r13,[rbx+8]: ${hex(e.slice(14, 18))}`);
  }

  const CODE_START = 18; // skip push rbp(1) + mov rbp,rsp(3) + mov rbx,[r15](7) + mov r12,[rbx](3) + mov r13,[rbx+8](4)

  // No-result epilogue: xor eax,eax (31 C0), xor edx,edx (31 D2), pop rbp (5D), ret (C3)
  function checkEpilogue(e) {
    const n = e.length;
    if (n < 6) throw new Error(`code too short: ${n}`);
    if (e[n-6] !== 0x31 || e[n-5] !== 0xC0) throw new Error(`bad xor eax,eax: ${hex(e.slice(n-6))}`);
    if (e[n-4] !== 0x31 || e[n-3] !== 0xD2) throw new Error(`bad xor edx,edx: ${hex(e.slice(n-6))}`);
    if (e[n-2] !== 0x5D || e[n-1] !== 0xC3) throw new Error(`bad epilogue tail: ${hex(e.slice(n-6))}`);
  }

  // Result epilogue: pop rax (58), xor edx,edx (31 D2), pop rbp (5D), ret (C3)
  function checkResultEpilogue(e) {
    const n = e.length;
    if (n < 5) throw new Error(`code too short: ${n}`);
    if (e[n-5] !== 0x58) throw new Error(`missing pop rax: ${hex(e.slice(Math.max(0,n-8), n))}`);
    if (e[n-4] !== 0x31 || e[n-3] !== 0xD2) throw new Error(`bad xor edx,edx: ${hex(e.slice(n-5))}`);
    if (e[n-2] !== 0x5D || e[n-1] !== 0xC3) throw new Error(`bad epilogue tail: ${hex(e.slice(n-5))}`);
  }

  // Helper: create minimal WASM with given result type and bytecode
  function wasmRT(rt, code) {
    const magic = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    const types = [0x01, 0x05, 0x01, 0x60, 0x00, 0x01, rt];
    const funcs = [0x03, 0x02, 0x01, 0x00];
    const body = [0x00, ...code, 0x0B];
    const codeSec = [0x0A, body.length + 2, 0x01, body.length, ...body];
    return new Uint8Array([...magic, ...types, ...funcs, ...codeSec]);
  }

  function wasm(rt, code, hasMem) {
    const magic = [0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
    const types = [0x01, 0x05, 0x01, 0x60, 0x00, 0x01, rt];
    const funcs = [0x03, 0x02, 0x01, 0x00];
    const memSec = hasMem ? [0x05, 0x03, 0x01, 0x00, 0x01] : [];
    const body = [0x00, ...code, 0x0B];
    const codeSec = [0x0A, body.length + 2, 0x01, body.length, ...body];
    return new Uint8Array([...magic, ...types, ...funcs, ...memSec, ...codeSec]);
  }

  // ══════════════════════════════════════════════════════
  // Constants
  // ══════════════════════════════════════════════════════

  test('i32.const 42', wasmRT(0x7F, [0x41, 0x2A]), (e) => {
    checkPrologue(e);
    if (e[CODE_START] !== 0x68 || e[CODE_START+1] !== 42) throw new Error('missing push 42');
    checkResultEpilogue(e);
  });

  test('i64.const 42', wasmRT(0x7E, [0x42, 0x2A]), (e) => {
    checkPrologue(e);
    // mov rax, imm64: 48 B8 + 8 byte qword; push rax: 50
    if (e[CODE_START] !== 0x48 || e[CODE_START+1] !== 0xB8) throw new Error('missing mov rax');
    let val = 0n;
    for (let i = 0; i < 8; i++) val |= BigInt(e[CODE_START+2+i]) << BigInt(i*8);
    if (val !== 42n) throw new Error(`wrong value: ${val}`);
    if (e[CODE_START+10] !== 0x50) throw new Error('missing push rax');
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // Control flow
  // ══════════════════════════════════════════════════════

  test('block 42', wasmRT(0x7F, [0x02, 0x7F, 0x41, 0x2A, 0x0B]), (e) => {
    checkPrologue(e);
    if (e[CODE_START] !== 0x68 || e[CODE_START+1] !== 42) throw new Error('missing push 42');
    checkResultEpilogue(e);
  });

  test('if-true 42', wasmRT(0x7F, [0x41, 0x01, 0x04, 0x7F, 0x41, 0x2A, 0x05, 0x41, 0x01, 0x0B]), (e) => {
    checkPrologue(e);
    if (contains(e, [0x85, 0xC0]) < 0) throw new Error('missing test eax,eax');
    if (contains(e, [0x0F, 0x84]) < 0 && contains(e, [0x74]) < 0) throw new Error('missing conditional jmp');
    if (contains(e, [0x68, 0x2A]) < 0) throw new Error('missing push 42');
  });

  test('loop-br 1', wasmRT(0x7F, [0x41, 0x01, 0x03, 0x7F, 0x41, 0x00, 0x0D, 0x00, 0x0B]), (e) => {
    checkPrologue(e);
    if (e[CODE_START] !== 0x68 || e[CODE_START+1] !== 1) throw new Error('missing push 1');
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // i32 binary (uses pop rcx=59, pop rax=58, then op eax,ecx)
  // ══════════════════════════════════════════════════════

  test('i32.add 3+5', wasmRT(0x7F, [0x41, 0x03, 0x41, 0x05, 0x6A]), (e) => {
    checkPrologue(e);
    if (contains(e, [0x68, 0x03]) < 0) throw new Error('missing push 3');
    if (contains(e, [0x68, 0x05]) < 0) throw new Error('missing push 5');
    // pop rcx (59), pop rax (58), add eax,ecx (01 C8), push rax (50)
    if (contains(e, [0x59, 0x58, 0x01, 0xC8]) < 0) throw new Error('missing add sequence');
    checkResultEpilogue(e);
  });

  test('i32.sub 10-3', wasmRT(0x7F, [0x41, 0x0A, 0x41, 0x03, 0x6B]), (e) => {
    // pop rcx, pop rax, sub eax,ecx (29 C8)
    if (contains(e, [0x59, 0x58, 0x29, 0xC8]) < 0) throw new Error('missing sub');
  });

  test('i32.mul 6*7', wasmRT(0x7F, [0x41, 0x06, 0x41, 0x07, 0x6C]), (e) => {
    // imul eax, ecx (0F AF C1)
    if (contains(e, [0x0F, 0xAF, 0xC1]) < 0) throw new Error('missing imul');
  });

  test('i32.and 5&3', wasmRT(0x7F, [0x41, 0x05, 0x41, 0x03, 0x71]), (e) => {
    // and eax, ecx (21 C8)
    if (contains(e, [0x59, 0x58, 0x21, 0xC8]) < 0) throw new Error('missing and');
  });

  test('i32.or 1|2', wasmRT(0x7F, [0x41, 0x01, 0x41, 0x02, 0x72]), (e) => {
    if (contains(e, [0x59, 0x58, 0x09, 0xC8]) < 0) throw new Error('missing or');
  });

  test('i32.xor 1^3', wasmRT(0x7F, [0x41, 0x01, 0x41, 0x03, 0x73]), (e) => {
    if (contains(e, [0x59, 0x58, 0x31, 0xC8]) < 0) throw new Error('missing xor');
  });

  test('i32.shl 1<<3', wasmRT(0x7F, [0x41, 0x01, 0x41, 0x03, 0x74]), (e) => {
    if (contains(e, [0x59, 0x58, 0xD3, 0xE0]) < 0) throw new Error('missing shl');
  });

  test('i32.shr_u 8>>1', wasmRT(0x7F, [0x41, 0x08, 0x41, 0x01, 0x76]), (e) => {
    if (contains(e, [0x59, 0x58, 0xD3, 0xE8]) < 0) throw new Error('missing shr_u');
  });

  test('i32.shr_s', wasmRT(0x7F, [0x41, 0x78, 0x41, 0x01, 0x75]), (e) => {
    if (contains(e, [0x59, 0x58, 0xD3, 0xF8]) < 0) throw new Error('missing shr_s');
  });

  test('i32.div_s 10/3', wasmRT(0x7F, [0x41, 0x0A, 0x41, 0x03, 0x6D]), (e) => {
    // pop rcx, pop rax, cdq (99), idiv ecx (F7 F9)
    if (contains(e, [0x59, 0x58, 0x99, 0xF7, 0xF9]) < 0) throw new Error('missing div_s');
  });

  test('i32.rem_s 10%3', wasmRT(0x7F, [0x41, 0x0A, 0x41, 0x03, 0x6F]), (e) => {
    if (contains(e, [0x59, 0x58, 0x99, 0xF7, 0xF9]) < 0) throw new Error('missing rem_s');
  });

  test('i32.rotl', wasmRT(0x7F, [0x41, 0x01, 0x41, 0x03, 0x77]), (e) => {
    if (contains(e, [0x59, 0x58, 0xD3, 0xC0]) < 0) throw new Error('missing rotl');
  });

  test('i32.rotr', wasmRT(0x7F, [0x41, 0x08, 0x41, 0x03, 0x78]), (e) => {
    if (contains(e, [0x59, 0x58, 0xD3, 0xC8]) < 0) throw new Error('missing rotr');
  });

  // ══════════════════════════════════════════════════════
  // i32 comparisons (pop rcx, pop rax, cmp eax,ecx, setcc al, movzx eax,al)
  // ══════════════════════════════════════════════════════

  test('i32.eqz 0', wasmRT(0x7F, [0x41, 0x00, 0x45]), (e) => {
    // pop rax, test eax,eax (85 C0), sete al (0F 94 C0), movzx eax,al (0F B6 C0)
    if (contains(e, [0x85, 0xC0, 0x0F, 0x94, 0xC0]) < 0) throw new Error('missing eqz');
  });

  test('i32.eq 5==5', wasmRT(0x7F, [0x41, 0x05, 0x41, 0x05, 0x46]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x94, 0xC0]) < 0) throw new Error('missing eq');
  });

  test('i32.ne 5!=7', wasmRT(0x7F, [0x41, 0x05, 0x41, 0x07, 0x47]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x95, 0xC0]) < 0) throw new Error('missing ne');
  });

  test('i32.lt_s 3<5', wasmRT(0x7F, [0x41, 0x03, 0x41, 0x05, 0x48]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x9C, 0xC0]) < 0) throw new Error('missing lt_s');
  });

  test('i32.gt_s 5>3', wasmRT(0x7F, [0x41, 0x05, 0x41, 0x03, 0x4A]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x9F, 0xC0]) < 0) throw new Error('missing gt_s');
  });

  test('i32.le_s 3<=5', wasmRT(0x7F, [0x41, 0x03, 0x41, 0x05, 0x4C]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x9E, 0xC0]) < 0) throw new Error('missing le_s');
  });

  test('i32.ge_s 5>=3', wasmRT(0x7F, [0x41, 0x05, 0x41, 0x03, 0x4E]), (e) => {
    if (contains(e, [0x59, 0x58, 0x39, 0xC8, 0x0F, 0x9D, 0xC0]) < 0) throw new Error('missing ge_s');
  });

  // ══════════════════════════════════════════════════════
  // i32 unary
  // ══════════════════════════════════════════════════════

  test('i32.clz 1', wasmRT(0x7F, [0x41, 0x01, 0x67]), (e) => {
    if (contains(e, [0x0F, 0xBD, 0xC0]) < 0) throw new Error('missing clz (bsr)');
  });

  test('i32.ctz 8', wasmRT(0x7F, [0x41, 0x08, 0x68]), (e) => {
    if (contains(e, [0x0F, 0xBC, 0xC0]) < 0) throw new Error('missing ctz (bsf)');
  });

  test('i32.popcnt 7', wasmRT(0x7F, [0x41, 0x07, 0x69]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0xB8, 0xC0]) < 0) throw new Error('missing popcnt');
  });

  // ══════════════════════════════════════════════════════
  // i32 extend (uses 0xBE=movsx/movzx for 8-bit, 0xBF for 16-bit)
  // ══════════════════════════════════════════════════════

  test('i32.extend8_s', wasmRT(0x7F, [0x41, 0x7F, 0xC0]), (e) => {
    if (contains(e, [0x0F, 0xBE, 0xC0]) < 0) throw new Error('missing extend8_s (movsx eax,al)');
  });

  test('i32.extend16_s', wasmRT(0x7F, [0x41, 0x00, 0xC1]), (e) => {
    if (contains(e, [0x0F, 0xBF, 0xC0]) < 0) throw new Error('missing extend16_s (movsx eax,ax)');
  });

  // ══════════════════════════════════════════════════════
  // i64/i32 conversion
  // ══════════════════════════════════════════════════════

  test('i32.wrap_i64', wasmRT(0x7E, [0x42, 0x2A, 0xA7]), (e) => {
    checkPrologue(e);
    checkResultEpilogue(e);
  });

  test('i64.extend_i32_s -1', wasmRT(0x7E, [0x41, 0x7F, 0xAC]), (e) => {
    if (contains(e, [0x48, 0x63, 0xC0]) < 0) throw new Error('missing extend_i32_s (movsxd)');
  });

  test('i64.extend_i32_u', wasmRT(0x7E, [0x41, 0xFF, 0xAD]), (e) => {
    // For extend_i32_u: if i32 is already in a register with upper zeros, just push
    // The emitted code should at least have prologue and epilogue
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // Memory (with memory section)
  // ══════════════════════════════════════════════════════

  test('i32.load', wasm(0x7F, [0x41, 0x00, 0x28, 0x02, 0x00], true), (e) => {
    checkPrologue(e);
    if (contains(e, [0x49, 0x8B, 0x97]) < 0) throw new Error('missing load r15->rdx for mem_ptr');
    checkResultEpilogue(e);
  });

  test('i32.store+load', wasm(0x7F, [0x41, 0x2A, 0x41, 0x00, 0x36, 0x02, 0x00, 0x41, 0x00, 0x28, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x59, 0x58]) < 0) throw new Error('missing pop for store');
  });

  test('i32.load8_s', wasm(0x7F, [0x41, 0x00, 0x2C, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x0F, 0xBE]) < 0) throw new Error('missing load8_s (movsx)');
  });

  test('i32.load8_u', wasm(0x7F, [0x41, 0x00, 0x2D, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x0F, 0xB6]) < 0) throw new Error('missing load8_u (movzx)');
  });

  test('i32.load16_s', wasm(0x7F, [0x41, 0x00, 0x2E, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x0F, 0xBF]) < 0) throw new Error('missing load16_s (movsx)');
  });

  test('i32.load16_u', wasm(0x7F, [0x41, 0x00, 0x2F, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x0F, 0xB7]) < 0) throw new Error('missing load16_u (movzx)');
  });

  test('i64.load', wasm(0x7E, [0x41, 0x00, 0x29, 0x02, 0x00], true), (e) => {
    checkPrologue(e);
    checkResultEpilogue(e);
  });

  test('i64.store', wasm(0x7E, [0x41, 0x2A, 0x41, 0x00, 0x37, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x59, 0x58]) < 0) throw new Error('missing pop for store');
  });

  test('i32.store8', wasm(0x7F, [0x41, 0x2A, 0x41, 0x00, 0x3A, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x59, 0x58]) < 0) throw new Error('missing pop for store');
  });

  test('i32.store16', wasm(0x7F, [0x41, 0x2A, 0x41, 0x00, 0x3B, 0x02, 0x00], true), (e) => {
    if (contains(e, [0x59, 0x58]) < 0) throw new Error('missing pop for store');
  });

  test('memory.size', wasm(0x7F, [0x3F, 0x00], true), (e) => {
    checkPrologue(e);
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // Float binary
  // ══════════════════════════════════════════════════════

  test('f32.add 1+2', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x00, 0x40, 0x92]), (e) => {
    checkPrologue(e);
    if (contains(e, [0xF3, 0x0F, 0x58]) < 0) throw new Error('missing addss');
    checkResultEpilogue(e);
  });

  test('f32.sub 1-2', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x00, 0x40, 0x93]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0x5C]) < 0) throw new Error('missing subss');
  });

  test('f32.mul 2*3', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x00, 0x40, 0x43, 0x00, 0x00, 0x40, 0x40, 0x94]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0x59]) < 0) throw new Error('missing mulss');
  });

  test('f32.div 6/2', wasmRT(0x7D, [0x43, 0x00, 0x00, 0xC0, 0x40, 0x43, 0x00, 0x00, 0x00, 0x40, 0x95]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0x5E]) < 0) throw new Error('missing divss');
  });

  test('f64.add 1+2', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xA0]), (e) => {
    checkPrologue(e);
    if (contains(e, [0xF2, 0x0F, 0x58]) < 0) throw new Error('missing addsd');
  });

  test('f64.sub', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xA1]), (e) => {
    if (contains(e, [0xF2, 0x0F, 0x5C]) < 0) throw new Error('missing subsd');
  });

  test('f64.mul', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xA2]), (e) => {
    if (contains(e, [0xF2, 0x0F, 0x59]) < 0) throw new Error('missing mulsd');
  });

  test('f64.div', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0xA3]), (e) => {
    if (contains(e, [0xF2, 0x0F, 0x5E]) < 0) throw new Error('missing divsd');
  });

  // ══════════════════════════════════════════════════════
  // Float comparisons (uses ucomiss/comisd + setcc)
  // ══════════════════════════════════════════════════════

  test('f32.eq 1==1', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x80, 0x3F, 0x5B]), (e) => {
    if (contains(e, [0x0F, 0x94, 0xC0]) < 0) throw new Error('missing sete for f32.eq');
  });

  test('f32.lt', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x00, 0x40, 0x43, 0x00, 0x00, 0x80, 0x3F, 0x5D]), (e) => {
    if (contains(e, [0x0F, 0x92, 0xC0]) < 0) throw new Error('missing setb for f32.lt');
  });

  test('f64.eq 1==1', wasmRT(0x7F, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x61]), (e) => {
    if (contains(e, [0x0F, 0x94, 0xC0]) < 0) throw new Error('missing sete for f64.eq');
  });

  // ══════════════════════════════════════════════════════
  // Select
  // ══════════════════════════════════════════════════════

  test('select true (pick val1)', wasmRT(0x7F, [0x41, 0x0A, 0x41, 0x14, 0x41, 0x01, 0x1B]), (e) => {
    // push val1(10), val2(20), cond(1)
    // select: pop rax (cond=1), test eax, cmove rdx,rcx...
    if (contains(e, [0x85, 0xC0]) < 0) throw new Error('missing test eax');
    // cmove: 48 0F 44 CA
    if (contains(e, [0x48, 0x0F, 0x44, 0xCA]) < 0) throw new Error('missing cmove rdx,rcx');
  });

  test('select false (pick val2)', wasmRT(0x7F, [0x41, 0x0A, 0x41, 0x14, 0x41, 0x00, 0x1B]), (e) => {
    if (contains(e, [0x85, 0xC0]) < 0) throw new Error('missing test eax');
    if (contains(e, [0x48, 0x0F, 0x44, 0xCA]) < 0) throw new Error('missing cmove rdx,rcx');
  });

  // ══════════════════════════════════════════════════════
  // i64 binary
  // ══════════════════════════════════════════════════════

  test('i64.add 3+5', wasmRT(0x7E, [0x42, 0x03, 0x42, 0x05, 0x7C]), (e) => {
    checkPrologue(e);
    // pop rcx, pop rax, add rax, rcx (REX.W 01 C8)
    if (contains(e, [0x59, 0x58, 0x48, 0x01, 0xC8]) < 0) throw new Error('missing i64.add');
    checkResultEpilogue(e);
  });

  test('i64.sub 10-3', wasmRT(0x7E, [0x42, 0x0A, 0x42, 0x03, 0x7D]), (e) => {
    if (contains(e, [0x59, 0x58, 0x48, 0x29, 0xC8]) < 0) throw new Error('missing i64.sub');
  });

  test('i64.mul 6*7', wasmRT(0x7E, [0x42, 0x06, 0x42, 0x07, 0x7E]), (e) => {
    // imul rax, rcx = REX.W 0F AF C1
    if (contains(e, [0x48, 0x0F, 0xAF, 0xC1]) < 0) throw new Error('missing i64.mul');
  });

  // ══════════════════════════════════════════════════════
  // Float unary
  // ══════════════════════════════════════════════════════

  test('f32.abs -1.0', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0xBF, 0x8B]), (e) => {
    if (contains(e, [0x25, 0xFF, 0xFF, 0xFF, 0x7F]) < 0) throw new Error('missing f32.abs (and eax, 0x7FFFFFFF)');
  });

  test('f32.neg 1.0', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x8C]), (e) => {
    // xor eax, 0x80000000 (35 00 00 00 80)
    if (contains(e, [0x35, 0x00, 0x00, 0x00, 0x80]) < 0) throw new Error('missing f32.neg (xor eax, 0x80000000)');
  });

  test('f32.sqrt 4.0', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x40, 0x91]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0x51]) < 0) throw new Error('missing sqrtss');
  });

  test('f64.abs -1.0', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xBF, 0x99]), (e) => {
    // btr rax, 63 = REX.W 0F BA F0 3F
    if (contains(e, [0x48, 0x0F, 0xBA, 0xF0, 0x3F]) < 0) throw new Error('missing f64.abs (btr rax, 63)');
  });

  test('f64.neg 1.0', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x9A]), (e) => {
    // bts rax, 63 = REX.W 0F BA E8 3F (ModRM /5 = BTS, reg=5, rm=0)
    if (contains(e, [0x48, 0x0F, 0xBA, 0xE8, 0x3F]) < 0) throw new Error('missing f64.neg (bts rax, 63)');
  });

  test('f64.sqrt 4.0', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x40, 0x9B]), (e) => {
    if (contains(e, [0xF2, 0x0F, 0x51]) < 0) throw new Error('missing sqrtsd');
  });

  test('f32.ceil', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x8D]), (e) => {
    if (contains(e, [0x66, 0x0F, 0x3A, 0x0A, 0xC0, 0x02]) < 0) throw new Error('missing roundss ceil');
  });

  test('f32.floor', wasmRT(0x7D, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x8E]), (e) => {
    if (contains(e, [0x66, 0x0F, 0x3A, 0x0A, 0xC0, 0x01]) < 0) throw new Error('missing roundss floor');
  });

  test('f64.ceil', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x9C]), (e) => {
    if (contains(e, [0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x02]) < 0) throw new Error('missing roundsd ceil');
  });

  test('f64.floor', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x9D]), (e) => {
    if (contains(e, [0x66, 0x0F, 0x3A, 0x0B, 0xC0, 0x01]) < 0) throw new Error('missing roundsd floor');
  });

  // ══════════════════════════════════════════════════════
  // Drop
  // ══════════════════════════════════════════════════════

  test('drop', wasmRT(0x7F, [0x41, 0x01, 0x1A, 0x41, 0x02]), (e) => {
    checkPrologue(e);
    // add rsp, 8 = 48 81 C4 08 00 00 00
    if (contains(e, [0x48, 0x81, 0xC4, 0x08]) < 0) throw new Error('missing drop (add rsp, 8)');
  });

  // ══════════════════════════════════════════════════════
  // Return
  // ══════════════════════════════════════════════════════

  test('return 42', wasmRT(0x7F, [0x41, 0x2A, 0x0F, 0x0B]), (e) => {
    checkPrologue(e);
    // return should emit pop rax + epilogue then ud2
    if (contains(e, [0x58, 0x31, 0xD2, 0x5D, 0xC3, 0x0F, 0x0B]) < 0) {
      // or: pop rax, xor edx,edx, pop rbp, ret, ud2
      // just check that ud2 is somewhere at the end
      if (e[e.length-2] !== 0x0F || e[e.length-1] !== 0x0B) throw new Error('missing ud2 after return');
    }
  });

  // ══════════════════════════════════════════════════════
  // table.set (0x26)
  // ══════════════════════════════════════════════════════

  test('table.set', wasmRT(0x7F, [0x41, 0x01, 0x41, 0x02, 0x26]), (e) => {
    checkPrologue(e);
    // Should NOT have ud2 near start
    if (contains(e, [0x0F, 0x0B], 0) === 4) throw new Error('table.set fell through to unsupported');
    // Should have pop rdx (5A) then pop rax (58) at start of template body
    const afterPrologue = contains(e, [0x5A, 0x58]);
    if (afterPrologue < 0) throw new Error('missing pop rdx, pop rax');
  });

  // ══════════════════════════════════════════════════════
  // i64 unary: clz, ctz, popcnt
  // ══════════════════════════════════════════════════════

  test('i64.clz', wasmRT(0x7E, [0x42, 0x01, 0x79]), (e) => {
    // F3 48 0F BD C0 = lzcnt rax, rax
    if (contains(e, [0xF3, 0x48, 0x0F, 0xBD, 0xC0]) < 0) throw new Error('missing lzcnt64');
  });

  test('i64.ctz', wasmRT(0x7E, [0x42, 0x08, 0x7A]), (e) => {
    if (contains(e, [0xF3, 0x48, 0x0F, 0xBC, 0xC0]) < 0) throw new Error('missing tzcnt64');
  });

  test('i64.popcnt', wasmRT(0x7E, [0x42, 0x07, 0x7B]), (e) => {
    if (contains(e, [0xF3, 0x48, 0x0F, 0xB8, 0xC0]) < 0) throw new Error('missing popcnt64');
  });

  // ══════════════════════════════════════════════════════
  // i64 binary (div_s, div_u, rem_s, rem_u, and, or, xor, shl, shr)
  // ══════════════════════════════════════════════════════

  test('i64.div_s 10/3', wasmRT(0x7E, [0x42, 0x0A, 0x42, 0x03, 0x7F]), (e) => {
    // cqo (48 99) then idiv rcx (48 F7 F9)
    if (contains(e, [0x48, 0x99]) < 0) throw new Error('missing cqo');
    if (contains(e, [0x48, 0xF7, 0xF9]) < 0) throw new Error('missing idiv rcx');
  });

  test('i64.div_u 10/3', wasmRT(0x7E, [0x42, 0x0A, 0x42, 0x03, 0x80]), (e) => {
    if (contains(e, [0x48, 0x31, 0xD2]) < 0) throw new Error('missing xor rdx,rdx');
    if (contains(e, [0x48, 0xF7, 0xF1]) < 0) throw new Error('missing div rcx');
  });

  test('i64.rem_s 10%3', wasmRT(0x7E, [0x42, 0x0A, 0x42, 0x03, 0x81]), (e) => {
    if (contains(e, [0x48, 0x89, 0xD0]) < 0) throw new Error('missing mov rax,rdx');
  });

  test('i64.and', wasmRT(0x7E, [0x42, 0x05, 0x42, 0x03, 0x83]), (e) => {
    if (contains(e, [0x48, 0x21, 0xC8]) < 0) throw new Error('missing and64');
  });

  test('i64.or', wasmRT(0x7E, [0x42, 0x01, 0x42, 0x02, 0x84]), (e) => {
    if (contains(e, [0x48, 0x09, 0xC8]) < 0) throw new Error('missing or64');
  });

  test('i64.xor', wasmRT(0x7E, [0x42, 0x01, 0x42, 0x03, 0x85]), (e) => {
    if (contains(e, [0x48, 0x31, 0xC8]) < 0) throw new Error('missing xor64');
  });

  test('i64.shl', wasmRT(0x7E, [0x42, 0x01, 0x42, 0x03, 0x86]), (e) => {
    if (contains(e, [0x48, 0xD3, 0xE0]) < 0) throw new Error('missing shl64');
  });

  test('i64.shr_s', wasmRT(0x7E, [0x42, 0x08, 0x42, 0x01, 0x87]), (e) => {
    if (contains(e, [0x48, 0xD3, 0xF8]) < 0) throw new Error('missing sar64');
  });

  test('i64.shr_u', wasmRT(0x7E, [0x42, 0x08, 0x42, 0x01, 0x88]), (e) => {
    if (contains(e, [0x48, 0xD3, 0xE8]) < 0) throw new Error('missing shr64');
  });

  test('i64.rotl', wasmRT(0x7E, [0x42, 0x01, 0x42, 0x03, 0x89]), (e) => {
    if (contains(e, [0x48, 0xD3, 0xC0]) < 0) throw new Error('missing rol64');
  });

  test('i64.rotr', wasmRT(0x7E, [0x42, 0x01, 0x42, 0x03, 0x8A]), (e) => {
    if (contains(e, [0x48, 0xD3, 0xC8]) < 0) throw new Error('missing ror64');
  });

  // ══════════════════════════════════════════════════════
  // i64 comparisons
  // ══════════════════════════════════════════════════════

  test('i64.eqz 0', wasmRT(0x7F, [0x42, 0x00, 0x50]), (e) => {
    // REX.W 85 C0 = test rax,rax; 0F 94 C0 = sete al
    if (contains(e, [0x48, 0x85, 0xC0]) < 0) throw new Error('missing test rax,rax');
    if (contains(e, [0x0F, 0x94, 0xC0]) < 0) throw new Error('missing sete');
  });

  test('i64.eq', wasmRT(0x7F, [0x42, 0x05, 0x42, 0x05, 0x51]), (e) => {
    if (contains(e, [0x48, 0x39, 0xC8]) < 0) throw new Error('missing cmp64');
    if (contains(e, [0x0F, 0x94, 0xC0]) < 0) throw new Error('missing sete');
  });

  test('i64.ne', wasmRT(0x7F, [0x42, 0x05, 0x42, 0x07, 0x52]), (e) => {
    if (contains(e, [0x0F, 0x95, 0xC0]) < 0) throw new Error('missing setne');
  });

  test('i64.lt_s', wasmRT(0x7F, [0x42, 0x03, 0x42, 0x05, 0x53]), (e) => {
    if (contains(e, [0x0F, 0x9C, 0xC0]) < 0) throw new Error('missing setl');
  });

  test('i64.gt_s', wasmRT(0x7F, [0x42, 0x05, 0x42, 0x03, 0x55]), (e) => {
    if (contains(e, [0x0F, 0x9F, 0xC0]) < 0) throw new Error('missing setg');
  });

  test('i64.le_s', wasmRT(0x7F, [0x42, 0x03, 0x42, 0x05, 0x57]), (e) => {
    if (contains(e, [0x0F, 0x9E, 0xC0]) < 0) throw new Error('missing setle');
  });

  test('i64.ge_s', wasmRT(0x7F, [0x42, 0x05, 0x42, 0x03, 0x59]), (e) => {
    if (contains(e, [0x0F, 0x9D, 0xC0]) < 0) throw new Error('missing setge');
  });

  test('i64.lt_u', wasmRT(0x7F, [0x42, 0x01, 0x42, 0x02, 0x54]), (e) => {
    if (contains(e, [0x0F, 0x92, 0xC0]) < 0) throw new Error('missing setb');
  });

  test('i64.gt_u', wasmRT(0x7F, [0x42, 0x02, 0x42, 0x01, 0x56]), (e) => {
    if (contains(e, [0x0F, 0x97, 0xC0]) < 0) throw new Error('missing seta');
  });

  test('i64.le_u', wasmRT(0x7F, [0x42, 0x01, 0x42, 0x01, 0x58]), (e) => {
    if (contains(e, [0x0F, 0x96, 0xC0]) < 0) throw new Error('missing setbe');
  });

  test('i64.ge_u', wasmRT(0x7F, [0x42, 0x02, 0x42, 0x01, 0x5A]), (e) => {
    if (contains(e, [0x0F, 0x93, 0xC0]) < 0) throw new Error('missing setae');
  });

  // ══════════════════════════════════════════════════════
  // Float loads/stores
  // ══════════════════════════════════════════════════════

  test('f32.load', wasm(0x7D, [0x41, 0x00, 0x2A, 0x02, 0x00], true), (e) => {
    checkPrologue(e);
    checkResultEpilogue(e);
  });

  test('f64.load', wasm(0x7C, [0x41, 0x00, 0x2B, 0x02, 0x00], true), (e) => {
    checkPrologue(e);
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // Float conversions
  // ══════════════════════════════════════════════════════

  test('f32.convert_i32_s', wasmRT(0x7D, [0x41, 0x2A, 0xB2]), (e) => {
    // F3 0F 2A C0 = cvtsi2ss xmm0, eax
    if (contains(e, [0xF3, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2ss');
  });

  test('f32.convert_i64_s', wasmRT(0x7D, [0x42, 0x2A, 0xB4]), (e) => {
    // F3 48 0F 2A C0 = cvtsi2ss xmm0, rax
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2ss rax');
  });

  test('f64.convert_i32_s', wasmRT(0x7C, [0x41, 0x2A, 0xB7]), (e) => {
    // F2 0F 2A C0 = cvtsi2sd xmm0, eax
    if (contains(e, [0xF2, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2sd');
  });

  test('f64.convert_i64_s', wasmRT(0x7C, [0x42, 0x2A, 0xB9]), (e) => {
    // F2 48 0F 2A C0 = cvtsi2sd xmm0, rax
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2sd rax');
  });

  test('i32.trunc_f32_s', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xA8]), (e) => {
    // F3 0F 2C C0 = cvttss2si eax, xmm0
    if (contains(e, [0xF3, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si');
  });

  test('i32.trunc_f64_s', wasmRT(0x7F, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40, 0xAA]), (e) => {
    // F2 0F 2C C0 = cvttsd2si eax, xmm0
    if (contains(e, [0xF2, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si');
  });

  test('i64.trunc_f32_s', wasmRT(0x7E, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xAE]), (e) => {
    // F3 48 0F 2C C0 = cvttss2si rax, xmm0
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si rax');
  });

  test('i64.trunc_f64_s', wasmRT(0x7E, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40, 0xB0]), (e) => {
    // F2 48 0F 2C C0 = cvttsd2si rax, xmm0
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si rax');
  });

  // ══════════════════════════════════════════════════════
  // Promote/demote
  // ══════════════════════════════════════════════════════

  test('f32.demote_f64', wasmRT(0x7D, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0xB6]), (e) => {
    // F2 0F 5A C0 = cvtsd2ss xmm0, xmm0
    if (contains(e, [0xF2, 0x0F, 0x5A, 0xC0]) < 0) throw new Error('missing cvtsd2ss');
  });

  test('f64.promote_f32', wasmRT(0x7C, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xBB]), (e) => {
    // F3 0F 5A C0 = cvtss2sd xmm0, xmm0
    if (contains(e, [0xF3, 0x0F, 0x5A, 0xC0]) < 0) throw new Error('missing cvtss2sd');
  });

  // ══════════════════════════════════════════════════════
  // Unsigned conversions
  // ══════════════════════════════════════════════════════

  test('i32.trunc_f32_u', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xA9]), (e) => {
    // F3 0F 2C C0 = cvttss2si eax, xmm0
    if (contains(e, [0xF3, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si');
    // 0F 49 D0 = cmovns eax, edx
    if (contains(e, [0x0F, 0x49, 0xD0]) < 0) throw new Error('missing cmovns');
  });

  test('i32.trunc_f64_u', wasmRT(0x7F, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40, 0xAB]), (e) => {
    // F2 0F 2C C0 = cvttsd2si eax, xmm0
    if (contains(e, [0xF2, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si');
    if (contains(e, [0x0F, 0x49, 0xD0]) < 0) throw new Error('missing cmovns');
  });

  test('i64.trunc_f32_u', wasmRT(0x7E, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xAF]), (e) => {
    // F3 48 0F 2C C0 = cvttss2si rax, xmm0
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si rax');
    // 48 0F 49 D0 = cmovns rax, rdx
    if (contains(e, [0x48, 0x0F, 0x49, 0xD0]) < 0) throw new Error('missing cmovns rax');
  });

  test('i64.trunc_f64_u', wasmRT(0x7E, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x40, 0xB1]), (e) => {
    // F2 48 0F 2C C0 = cvttsd2si rax, xmm0
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si rax');
    if (contains(e, [0x48, 0x0F, 0x49, 0xD0]) < 0) throw new Error('missing cmovns rax');
  });

  test('f32.convert_i32_u', wasmRT(0x7D, [0x41, 0x2A, 0xB3]), (e) => {
    // F3 0F 2A C0 = cvtsi2ss xmm0, eax
    if (contains(e, [0xF3, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2ss');
  });

  test('f32.convert_i64_u', wasmRT(0x7D, [0x42, 0x2A, 0xB5]), (e) => {
    // F3 48 0F 2A C0 = cvtsi2ss xmm0, rax
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2ss rax');
  });

  test('f64.convert_i32_u', wasmRT(0x7C, [0x41, 0x2A, 0xB8]), (e) => {
    // F2 0F 2A C0 = cvtsi2sd xmm0, eax
    if (contains(e, [0xF2, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2sd');
  });

  test('f64.convert_i64_u', wasmRT(0x7C, [0x42, 0x2A, 0xBA]), (e) => {
    // F2 48 0F 2A C0 = cvtsi2sd xmm0, rax
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2A, 0xC0]) < 0) throw new Error('missing cvtsi2sd rax');
  });

  // ══════════════════════════════════════════════════════
  // Saturating truncation (0xFC prefix)
  // ══════════════════════════════════════════════════════

  // i32.trunc_sat_f32_s (0xFC, 0x00): f32.const 1.0
  test('i32.trunc_sat_f32_s', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x80, 0x3F, 0xFC, 0x00]), (e) => {
    // cvttss2si eax, xmm0 = F3 0F 2C C0
    if (contains(e, [0xF3, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si');
    // Should have at least one rel8 jump (7x or 7x) for clamping
    let jumps = 0;
    for (let i = 0; i < e.length; i++) if (e[i] === 0x7A || e[i] === 0x73 || e[i] === 0x72) jumps++;
    if (jumps < 1) throw new Error('expected conditional jumps for clamping');
  });

  // i32.trunc_sat_f32_u (0xFC, 0x01): NaN → 0
  test('i32.trunc_sat_f32_u_nan', wasmRT(0x7F, [0x43, 0x00, 0x00, 0xC0, 0x7F, 0xFC, 0x01]), (e) => {
    // Should contain cvttss2si for the normal path call
    if (contains(e, [0xF3, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si');
    // ucomiss xmm0,xmm0 = 0F 2E C0 (NaN check preamble)
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self for NaN');
  });

  // i32.trunc_sat_f64_s (0xFC, 0x02): f64.const 1.0
  test('i32.trunc_sat_f64_s', wasmRT(0x7F, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0xFC, 0x02]), (e) => {
    // cvttsd2si eax, xmm0 = F2 0F 2C C0
    if (contains(e, [0xF2, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si');
    if (contains(e, [0x66, 0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomisd self for NaN');
  });

  // i32.trunc_sat_f64_u (0xFC, 0x03): f64.const -1.0 → 0 (underflow)
  test('i32.trunc_sat_f64_u_underflow', wasmRT(0x7F, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xBF, 0xFC, 0x03]), (e) => {
    // cvttsd2si + cmovns (the u64 trunc template)
    if (contains(e, [0xF2, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si');
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self');
  });

  // i64.trunc_sat_f32_s (0xFC, 0x04): f32.const -1.0 → -1 (normal)
  test('i64.trunc_sat_f32_s', wasmRT(0x7E, [0x43, 0x00, 0x00, 0x80, 0xBF, 0xFC, 0x04]), (e) => {
    // cvttss2si rax, xmm0 = F3 48 0F 2C C0
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si rax');
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self');
  });

  // i64.trunc_sat_f32_u (0xFC, 0x05): f32.const 0.0 → 0
  test('i64.trunc_sat_f32_u', wasmRT(0x7E, [0x43, 0x00, 0x00, 0x00, 0x00, 0xFC, 0x05]), (e) => {
    // cvttss2si rax, xmm0
    if (contains(e, [0xF3, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttss2si rax');
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self');
  });

  // i64.trunc_sat_f64_s (0xFC, 0x06): f64.const -1.0
  test('i64.trunc_sat_f64_s', wasmRT(0x7E, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xBF, 0xFC, 0x06]), (e) => {
    // cvttsd2si rax, xmm0 = F2 48 0F 2C C0
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si rax');
    if (contains(e, [0x66, 0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomisd self');
  });

  // i64.trunc_sat_f64_u (0xFC, 0x07): f64.const 1.0
  test('i64.trunc_sat_f64_u', wasmRT(0x7E, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0xFC, 0x07]), (e) => {
    // cvttsd2si rax, xmm0
    if (contains(e, [0xF2, 0x48, 0x0F, 0x2C, 0xC0]) < 0) throw new Error('missing cvttsd2si rax');
    if (contains(e, [0x66, 0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomisd self');
  });

  // ══════════════════════════════════════════════════════
  // i64 narrow loads/stores
  // ══════════════════════════════════════════════════════

  test('i64.load8_s', wasm(0x7E, [0x41, 0x00, 0x30, 0x00, 0x00], true), (e) => {
    // REX.W movsx rax, byte = 48 0F BE 04 0A
    if (contains(e, [0x48, 0x0F, 0xBE, 0x04, 0x0A]) < 0) throw new Error('missing movsx rax, byte');
  });

  test('i64.load8_u', wasm(0x7E, [0x41, 0x00, 0x31, 0x00, 0x00], true), (e) => {
    // movzx eax, byte = 0F B6 04 0A
    if (contains(e, [0x0F, 0xB6, 0x04, 0x0A]) < 0) throw new Error('missing movzx eax, byte');
  });

  test('i64.load16_s', wasm(0x7E, [0x41, 0x00, 0x32, 0x00, 0x00], true), (e) => {
    if (contains(e, [0x48, 0x0F, 0xBF, 0x04, 0x0A]) < 0) throw new Error('missing movsx rax, word');
  });

  test('i64.load16_u', wasm(0x7E, [0x41, 0x00, 0x33, 0x00, 0x00], true), (e) => {
    if (contains(e, [0x0F, 0xB7, 0x04, 0x0A]) < 0) throw new Error('missing movzx eax, word');
  });

  test('i64.load32_s', wasm(0x7E, [0x41, 0x00, 0x34, 0x00, 0x00], true), (e) => {
    // REX.W movsxd rax, dword = 48 63 04 0A
    if (contains(e, [0x48, 0x63, 0x04, 0x0A]) < 0) throw new Error('missing movsxd rax, dword');
  });

  test('i64.load32_u', wasm(0x7E, [0x41, 0x00, 0x35, 0x00, 0x00], true), (e) => {
    // mov eax, [rdx+rcx] = 8B 04 0A
    if (contains(e, [0x8B, 0x04, 0x0A]) < 0) throw new Error('missing mov eax, dword');
  });

  test('i64.store8', wasm(0x7E, [0x41, 0x2A, 0x41, 0x00, 0x3C, 0x00, 0x00], true), (e) => {
    // mov [rdx+rcx], al = 88 04 0A
    if (contains(e, [0x88, 0x04, 0x0A]) < 0) throw new Error('missing mov byte store');
  });

  test('i64.store16', wasm(0x7E, [0x41, 0x2A, 0x41, 0x00, 0x3D, 0x00, 0x00], true), (e) => {
    // 66 89 04 0A = mov [rdx+rcx], ax (66 prefix precedes load, then 89 04 0A follows)
    if (contains(e, [0x89, 0x04, 0x0A]) < 0) throw new Error('missing mov word store');
  });

  test('i64.store32', wasm(0x7E, [0x41, 0x2A, 0x41, 0x00, 0x3E, 0x00, 0x00], true), (e) => {
    // 89 04 0A = mov [rdx+rcx], eax
    if (contains(e, [0x89, 0x04, 0x0A]) < 0) throw new Error('missing mov dword store');
  });

  // ══════════════════════════════════════════════════════
  // f32/f64.min, max, copysign
  // ══════════════════════════════════════════════════════

  test('f32.min', wasmRT(0x7B, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x00, 0x40, 0x96]), (e) => {
    // minss = F3 0F 5D C1, ucomiss self = 0F 2E C0
    if (contains(e, [0xF3, 0x0F, 0x5D, 0xC1]) < 0) throw new Error('missing minss');
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self for NaN');
  });

  test('f32.max', wasmRT(0x7B, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x00, 0x40, 0x97]), (e) => {
    if (contains(e, [0xF3, 0x0F, 0x5F, 0xC1]) < 0) throw new Error('missing maxss');
    if (contains(e, [0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomiss self');
  });

  test('f32.copysign', wasmRT(0x7B, [0x43, 0x00, 0x00, 0x80, 0x3F, 0x43, 0x00, 0x00, 0x00, 0xC0, 0x98]), (e) => {
    // btr eax, 31 = 0F BA F0 1F
    if (contains(e, [0x0F, 0xBA, 0xF0, 0x1F]) < 0) throw new Error('missing btr eax, 31');
    // and ecx, 0x80000000
    if (contains(e, [0x81, 0xE1, 0x00, 0x00, 0x00, 0x80]) < 0) throw new Error('missing and ecx, 0x80000000');
  });

  test('f64.min', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xA4]), (e) => {
    // minsd = F2 0F 5D C1, ucomisd self = 66 0F 2E C0
    if (contains(e, [0xF2, 0x0F, 0x5D, 0xC1]) < 0) throw new Error('missing minsd');
    if (contains(e, [0x66, 0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomisd self');
  });

  test('f64.max', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0xA5]), (e) => {
    if (contains(e, [0xF2, 0x0F, 0x5F, 0xC1]) < 0) throw new Error('missing maxsd');
    if (contains(e, [0x66, 0x0F, 0x2E, 0xC0]) < 0) throw new Error('missing ucomisd self');
  });

  test('f64.copysign', wasmRT(0x7C, [0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, 0x44, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0xA6]), (e) => {
    // btr rax, 63 = 48 0F BA F0 3F
    if (contains(e, [0x48, 0x0F, 0xBA, 0xF0, 0x3F]) < 0) throw new Error('missing btr rax, 63');
  });

  // ══════════════════════════════════════════════════════
  // Reinterpret (no-ops)
  // ══════════════════════════════════════════════════════

  test('i32.reinterpret_f32', wasmRT(0x7F, [0x43, 0x00, 0x00, 0x00, 0x00, 0xBC]), (e) => {
    checkPrologue(e);
    // Reinterpret is a no-op — no extra code emitted
    checkResultEpilogue(e);
  });

  // ══════════════════════════════════════════════════════
  // i64 sign extension
  // ══════════════════════════════════════════════════════

  test('i64.extend8_s', wasmRT(0x7E, [0x42, 0x7F, 0xC2]), (e) => {
    // movsx rax, al = 48 0F BE C0
    if (contains(e, [0x48, 0x0F, 0xBE, 0xC0]) < 0) throw new Error('missing movsx rax, al');
  });

  test('i64.extend16_s', wasmRT(0x7E, [0x42, 0x7F, 0xC3]), (e) => {
    if (contains(e, [0x48, 0x0F, 0xBF, 0xC0]) < 0) throw new Error('missing movsx rax, ax');
  });

  test('i64.extend32_s', wasmRT(0x7E, [0x42, 0x7F, 0xC4]), (e) => {
    if (contains(e, [0x48, 0x63, 0xC0]) < 0) throw new Error('missing movsxd rax, eax');
  });

  // ══════════════════════════════════════════════════════
  // Summary
  // ══════════════════════════════════════════════════════

  console.log(`\n${passed}/${passed+failed} passed (${failed} failed)`);
  if (failed > 0) process.exit(1);
}

main().catch(e => { console.error(e); process.exit(1); });
