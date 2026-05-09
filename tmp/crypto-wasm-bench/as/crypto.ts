function rotr(x: u32, n: u32): u32 {
  return (x >>> n) | (x << (32 - n));
}

function readU32(ptr: usize): u32 {
  return (
    (load<u8>(ptr) as u32) << 24 |
    (load<u8>(ptr + 1) as u32) << 16 |
    (load<u8>(ptr + 2) as u32) << 8 |
    (load<u8>(ptr + 3) as u32)
  );
}

function writeU32(ptr: usize, value: u32): void {
  store<u8>(ptr, value >>> 24);
  store<u8>(ptr + 1, value >>> 16);
  store<u8>(ptr + 2, value >>> 8);
  store<u8>(ptr + 3, value);
}

const K: StaticArray<u32> = [
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
  0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
  0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
  0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
  0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
  0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
  0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

function sha256Into(inputPtr: usize, inputLen: usize, outPtr: usize): void {
  let h0: u32 = 0x6a09e667, h1: u32 = 0xbb67ae85, h2: u32 = 0x3c6ef372, h3: u32 = 0xa54ff53a;
  let h4: u32 = 0x510e527f, h5: u32 = 0x9b05688c, h6: u32 = 0x1f83d9ab, h7: u32 = 0x5be0cd19;
  const totalLen = ((inputLen + 9 + 63) & ~63) as usize;
  const buf = heap.alloc(totalLen);
  memory.fill(buf, 0, totalLen);
  memory.copy(buf, inputPtr, inputLen);
  store<u8>(buf + inputLen, 0x80);
  const bitLen = inputLen as u64 * 8;
  store<u32>(buf + totalLen - 8, bswap<u32>((bitLen >> 32) as u32));
  store<u32>(buf + totalLen - 4, bswap<u32>(bitLen as u32));

  const w = heap.alloc(64 * sizeof<u32>());
  for (let off: usize = 0; off < totalLen; off += 64) {
    for (let i = 0; i < 16; i++) store<u32>(w + (i << 2), readU32(buf + off + ((i as usize) << 2)));
    for (let i = 16; i < 64; i++) {
      const wm15 = load<u32>(w + ((i - 15) << 2));
      const wm2 = load<u32>(w + ((i - 2) << 2));
      const s0 = rotr(wm15, 7) ^ rotr(wm15, 18) ^ (wm15 >>> 3);
      const s1 = rotr(wm2, 17) ^ rotr(wm2, 19) ^ (wm2 >>> 10);
      store<u32>(w + (i << 2), load<u32>(w + ((i - 16) << 2)) + s0 + load<u32>(w + ((i - 7) << 2)) + s1);
    }

    let a = h0, b = h1, c = h2, d = h3, e = h4, f = h5, g = h6, h = h7;
    for (let i = 0; i < 64; i++) {
      const s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
      const ch = (e & f) ^ (~e & g);
      const temp1 = h + s1 + ch + unchecked(K[i]) + load<u32>(w + (i << 2));
      const s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
      const maj = (a & b) ^ (a & c) ^ (b & c);
      const temp2 = s0 + maj;
      h = g; g = f; f = e; e = d + temp1; d = c; c = b; b = a; a = temp1 + temp2;
    }
    h0 += a; h1 += b; h2 += c; h3 += d; h4 += e; h5 += f; h6 += g; h7 += h;
  }

  writeU32(outPtr, h0); writeU32(outPtr + 4, h1); writeU32(outPtr + 8, h2); writeU32(outPtr + 12, h3);
  writeU32(outPtr + 16, h4); writeU32(outPtr + 20, h5); writeU32(outPtr + 24, h6); writeU32(outPtr + 28, h7);
  heap.free(w);
  heap.free(buf);
}

export function sha256(inputPtr: usize, inputLen: usize, outPtr: usize): i32 {
  sha256Into(inputPtr, inputLen, outPtr);
  return 0;
}

export function hmac_sha256(keyPtr: usize, keyLen: usize, inputPtr: usize, inputLen: usize, outPtr: usize): i32 {
  const block = heap.alloc(64);
  memory.fill(block, 0, 64);
  if (keyLen > 64) {
    sha256Into(keyPtr, keyLen, block);
  } else {
    memory.copy(block, keyPtr, keyLen);
  }

  const inner = heap.alloc(64 + inputLen);
  const outer = heap.alloc(96);
  for (let i: usize = 0; i < 64; i++) {
    const b = load<u8>(block + i);
    store<u8>(inner + i, b ^ 0x36);
    store<u8>(outer + i, b ^ 0x5c);
  }
  memory.copy(inner + 64, inputPtr, inputLen);
  sha256Into(inner, 64 + inputLen, outer + 64);
  sha256Into(outer, 96, outPtr);

  heap.free(outer);
  heap.free(inner);
  heap.free(block);
  return 0;
}
