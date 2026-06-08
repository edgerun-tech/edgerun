#!/usr/bin/env bun
// Pre-compute x86-64 encoding pool from package.json
// Pool: flat Uint8Array, concatenated per opcode as: [variant_0_bytes..., variant_1_bytes..., ...]
// Meta: Int32Array[4*256] → [poolByteOffset, variantByteLen, immSlotOffset, immSlotSize]
//   immSlotOffset = byte offset from start of variant where 0/1/4/8-byte immediate goes, -1 if none
//   immSlotSize = bytes of immediate to write (1, 4, 8)
// Dispatch: base = meta[op*4]; slot = ((d&15)<<4)|(s&15); copy from pool[base + slot*len] for len bytes
import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const DIR = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(DIR, '..');

const pkg = JSON.parse(readFileSync(resolve(ROOT, 'package.json'), 'utf-8'));
const arch = process.argv[2] || 'x86_64';
const data = pkg.edgerun.encodings[arch];
if (!data) { console.error(`no data for ${arch}`); process.exit(1); }

const templates = data.t;
const opcodes = data.wasm;
const REGS = data.regs || {};

// ── Parse a compound entry like "test32_rr($d,$d) jcc_rel32(0x95)" ──
// Returns array of { tmpl, args } in order
function parse(raw) {
  if (!raw || raw === '') return null;
  const parts = [];
  let i = 0;
  while (i < raw.length) {
    // Skip whitespace
    while (i < raw.length && raw[i] === ' ') i++;
    if (i >= raw.length) break;
    const start = i;
    // Find template name (alphanumeric + _)
    while (i < raw.length && /[a-zA-Z0-9_]/.test(raw[i])) i++;
    if (i === start) break; // shouldn't happen
    const tmpl = raw.slice(start, i);
    // Check for params in parens
    if (i < raw.length && raw[i] === '(') {
      let depth = 1, j = i + 1;
      while (j < raw.length && depth > 0) {
        if (raw[j] === '(') depth++;
        else if (raw[j] === ')') depth--;
        j++;
      }
      const argstr = raw.slice(i + 1, j - 1);
      const args = {};
      for (const pair of argstr.split(',')) {
        const eq = pair.indexOf('=');
        if (eq >= 0) {
          args[pair.slice(0, eq).trim()] = pair.slice(eq + 1).trim();
        } else {
          // positional args — map by index
          args[String(parts.length)] = pair.trim();
        }
      }
      parts.push({ tmpl, args });
      i = j;
    } else {
      parts.push({ tmpl, args: {} });
    }
  }
  return parts.length ? parts : null;
}

// ── Resolve template string to bytes with register slots ──────────
function resolveTmpl(str, d, s, params) {
  const out = [];
  let immIdx = -1, immSize = 0;

  const regVal = v => {
    if (v === '$d' || v === '$l') return d;
    if (v === '$s' || v === '$r') return s;
    if (v.startsWith('$')) return s; // generic reg slot, use $s
    if (v.startsWith('@')) return params[v] !== undefined ? params[v] : 0;
    if (REGS[v] !== undefined) return REGS[v];
    return parseInt(v);
  };

  for (const tok of str.split(/\s+/)) {
    if (!tok) continue;

    // @param as standalone byte
    if (tok.startsWith('@') && !tok.includes('(')) {
      const val = params[tok];
      if (val !== undefined) { out.push(val); continue; }
      // runtime param slot (1 byte)
      immIdx = out.length; immSize = 1; out.push(0);
      continue;
    }

    const p = tok.indexOf('(');
    const base = p < 0 ? tok : tok.slice(0, p);
    const args = p < 0 ? [] : tok.slice(p + 1, -1).split(',');

    switch (base) {
      case 'rex': {
        const w = +args[0], r = regVal(args[1]), x = +args[2], b = regVal(args[3]);
        out.push(0x40 | (w << 3) | (((r >> 3) & 1) << 2) | (((x >> 3) & 1) << 1) | ((b >> 3) & 1));
        break;
      }
      case 'modrm': {
        const mod = +args[0], regv = regVal(args[1]), rm = regVal(args[2]);
        out.push((mod << 6) | ((regv & 7) << 3) | (rm & 7));
        break;
      }
      case 'regcode':
        out.push(parseInt(args[0], 16) | (regVal(args[1]) & 7));
        break;
      case 'sib':
        out.push((+args[0] << 6) | ((regVal(args[1]) & 7) << 3) | (regVal(args[2]) & 7));
        break;
      case 'imm8':  immIdx = out.length; immSize = 1; out.push(0); break;
      case 'imm32': immIdx = out.length; immSize = 4; out.push(0,0,0,0); break;
      case 'imm64': immIdx = out.length; immSize = 8; out.push(0,0,0,0,0,0,0,0); break;
      case 'rel8':  immIdx = out.length; immSize = 1; out.push(0); break;
      case 'rel32': immIdx = out.length; immSize = 4; out.push(0,0,0,0); break;
      default:
        if (/^0x[0-9a-fA-F]+$/.test(tok)) out.push(parseInt(tok, 16));
        else if (/^\d+$/.test(tok)) out.push(+tok);
        else throw Error(`unknown "${tok}"`);
    }
  }
  return { bytes: out, immIdx, immSize };
}

// ── Detect register slots ────────────────────────────────────────
function usedSlots(str) {
  const s = new Set();
  if (/\$[dl]/.test(str)) s.add('d');
  if (/\$[sr]/.test(str)) s.add('s');
  return s;
}

// ── Build all variant bytes for one opcode ────────────────────────
function buildOp(parts, count, hasD, hasS) {
  const results = [];

  for (let i = 0; i < count; i++) {
    const d = hasD && hasS ? (i >> 4) & 0xF : hasD ? i & 0xF : 0;
    const s = hasD && hasS ? i & 0xF : hasS ? i & 0xF : 0;

    const buf = [];
    let immIdx = -1, immSize = 0;
    let byteOff = 0;

    for (const { tmpl, args } of parts) {
      const str = templates[tmpl];
      if (!str) throw Error(`unknown template "${tmpl}"`);

      // Resolve @param values for this part from args
      const pms = {};
      for (const [k, v] of Object.entries(args)) {
        if (v.startsWith('$')) continue; // runtime param
        pms['@' + k] = v.startsWith('0x') ? parseInt(v, 16) : +v;
      }

      const res = resolveTmpl(str, d, s, pms);
      buf.push(...res.bytes);
      if (res.immIdx >= 0) {
        immIdx = byteOff + res.immIdx;
        immSize = res.immSize;
      }
      byteOff = buf.length;
    }

    results.push({ bytes: new Uint8Array(buf), immIdx, immSize });
  }
  return results;
}

// ── Scan all opcodes ────────────────────────────────────────────
const opInfo = []; // index by opcode, null = no encoding
for (let op = 0; op < 256; op++) {
  const key = `0x${op.toString(16).padStart(2, '0').toUpperCase()}`;
  const raw = opcodes[key];
  const parts = parse(raw);
  if (!parts || parts.length === 0) { opInfo.push(null); continue; }

  // Check if any template is unknown
  let ok = true;
  for (const { tmpl } of parts) {
    if (!templates[tmpl]) { ok = false; break; }
  }
  if (!ok) { opInfo.push(null); continue; }

  // Detect register slot usage across ALL parts
  let hasD = false, hasS = false;
  for (const { tmpl } of parts) {
    const slots = usedSlots(templates[tmpl]);
    if (slots.has('d')) hasD = true;
    if (slots.has('s')) hasS = true;
  }

  const count = hasD && hasS ? 256 : (hasD || hasS ? 16 : 1);
  const variants = buildOp(parts, count, hasD, hasS);
  const len = variants[0].bytes.length;

  opInfo.push({ count, len, variants, hasD, hasS });
}

// ── Assign pool offsets ─────────────────────────────────────────
let poolPos = 0;
for (let op = 0; op < 256; op++) {
  const info = opInfo[op];
  if (!info) continue;
  const variantLen = info.len;
  info.off = poolPos;
  poolPos += info.count * variantLen;
}

// ── Build pool ──────────────────────────────────────────────────
const pool = new Uint8Array(poolPos);
const meta = new Int32Array(256 * 5);
meta.fill(-1);

for (let op = 0; op < 256; op++) {
  const info = opInfo[op];
  if (!info) continue;

  const { variants, count, len, off, hasD, hasS } = info;
  const slotCode = hasD && hasS ? 3 : hasD ? 1 : hasS ? 2 : 0;
  meta[op * 5 + 0] = off;
  meta[op * 5 + 1] = len;
  meta[op * 5 + 2] = variants[0].immIdx;
  meta[op * 5 + 3] = variants[0].immSize;
  meta[op * 5 + 4] = slotCode;

  for (let i = 0; i < count; i++) {
    const { bytes } = variants[i];
    const pos = off + i * len;
    pool.set(bytes, pos);
  }
}

// ── Write output ────────────────────────────────────────────────
const outDir = resolve(ROOT, 'out', 'gen');
mkdirSync(outDir, { recursive: true });

const poolName = arch === 'x86_64' ? '' : `-${arch}`;
writeFileSync(resolve(outDir, `jit-pool${poolName}.bin`), pool);
writeFileSync(resolve(outDir, `jit-meta${poolName}.bin`), new Uint8Array(meta.buffer));

// JS module
const poolArr = [...pool];
const metaArr = [...meta];
const code = `// Auto-generated for ${arch}
export const pt = new Uint8Array([${poolArr.join(',')}]);
export const mt = new Int32Array([${metaArr.join(',')}]);
`;
writeFileSync(resolve(outDir, `jit-encoder-${arch}.mjs`), code);

const validCount = opInfo.filter(x => x !== null).length;
const totalVariants = opInfo.reduce((s, x) => s + (x ? x.count * x.len : 0), 0);
console.log(`pool: ${pool.length} bytes (${validCount} opcodes, ${totalVariants} total variant-bytes)`);
console.log(`meta: ${meta.length} ints`);
