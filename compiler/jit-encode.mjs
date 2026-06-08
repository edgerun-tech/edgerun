// slot codes: 0=noRegs 1=hasD 2=hasS 3=hasD+hasS
export function encode(pool, meta, op, d, s, imm, out, pos) {
  const base = op * 5;
  const off = meta[base];
  if (off < 0) return 0;
  const len = meta[base + 1];
  const immOff = meta[base + 2];
  const immSz = meta[base + 3];
  const sc = meta[base + 4];

  let slot;
  if (sc === 3) slot = ((d & 0xF) << 4) | (s & 0xF);
  else if (sc === 1) slot = d & 0xF;
  else if (sc === 2) slot = s & 0xF;
  else slot = 0;

  const src = off + slot * len;
  out.set(pool.subarray(src, src + len), pos);

  if (immOff >= 0) writeImm(out, pos + immOff, immSz, imm);
  return len;
}

function writeImm(out, p, sz, val) {
  if (sz === 4) {
    out[p] = val; out[p + 1] = val >> 8; out[p + 2] = val >> 16; out[p + 3] = val >> 24;
  } else if (sz === 8) {
    const lo = val[0], hi = val[1];
    out[p] = lo; out[p + 1] = lo >> 8; out[p + 2] = lo >> 16; out[p + 3] = lo >> 24;
    out[p + 4] = hi; out[p + 5] = hi >> 8; out[p + 6] = hi >> 16; out[p + 7] = hi >> 24;
  } else {
    out[p] = val & 0xFF;
  }
}
