import { readFileSync } from 'fs';
import { WASI } from 'wasi';

const wasi = new WASI({ version: 'preview1' });
const wasm = readFileSync('/tmp/edgerun-fresh.wasm');

const linuxStubs = {
  read: () => -1, write: () => -1, open: () => -1, close: () => -1,
  poll: () => -1, mmap: () => -1, munmap: () => 0, socket: () => -1,
  connect: () => -1, sendmsg: () => -1, memfd_create: () => -1, ftruncate: () => -1,
};

const { instance } = await WebAssembly.instantiate(wasm, {
  wasi_snapshot_preview1: wasi.wasiImport,
  linux: linuxStubs,
});
const e = instance.exports;

const HEX_BUF = 0x7F000;
function hex(ptr, len) {
  e.hex_encode_lower(ptr, len, HEX_BUF, len * 2 + 1);
  return new TextDecoder().decode(new Uint8Array(e.memory.buffer, HEX_BUF, len * 2));
}

const mem = new Uint8Array(e.memory.buffer);

// Dump the full SHA256 K constants area (256 bytes from 0x7530)
console.log('Full SHA256 K constants 0x7530-0x7630:');
for (let off = 0; off < 256; off += 16) {
  const addr = 0x7530 + off;
  console.log(`  ${addr.toString(16)}: ${hex(addr, 16)}`);
}

// Expected: the data should have K[63] = 0xc67178f2 at offset 0xFC
// in LE bytes: f2 78 71 c6 at 0x762C
console.log('');
console.log('K[60] expected @0x7620: 90 eb e6 50 ->', hex(0x7620, 4), '(LE)');
console.log('K[61] expected @0x7624: a4 f7 a3 f9 ->', hex(0x7624, 4), '(LE)');
console.log('K[62] expected @0x7628: be f2 78 71 ->', hex(0x7628, 4), '(LE)');
console.log('K[63] expected @0x762C: c6 71 78 f2 ->', hex(0x762C, 4), '(LE)');

// Check if there are other data sections overlapping
console.log('');
console.log('Check for data sections from wasm2wat:');
