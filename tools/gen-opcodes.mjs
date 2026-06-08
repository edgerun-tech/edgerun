#!/usr/bin/env bun
// Generate opcode lookup table for WAT parser
// Output: out/gen/opcode-table.wat

const { writeText, ensureDir } = await import('./build-lib.mjs');

// [keyword, opcode, imm_type, prefix_flags]
// imm_type: 0=none, 1=i32_const, 2=i64_const, 3=local_idx, 4=global_idx,
//           5=func_idx, 6=type_idx, 7=br_label, 8=memarg, 9=blocktype,
//           10=call_indirect, 11=memory_op, 12=br_table
// prefix_flags: 0=normal, 1=0xFC, 2=0xFD

const TABLE = [
  // Control flow
  ["unreachable", 0x00, 0, 0],
  ["nop", 0x01, 0, 0],
  ["block", 0x02, 9, 0],
  ["loop", 0x03, 9, 0],
  ["if", 0x04, 9, 0],
  ["else", 0x05, 0, 0],
  ["end", 0x0B, 0, 0],
  ["br", 0x0C, 7, 0],
  ["br_if", 0x0D, 7, 0],
  ["br_table", 0x0E, 12, 0],
  ["return", 0x0F, 0, 0],

  // Calls
  ["call", 0x10, 5, 0],
  ["call_indirect", 0x11, 10, 0],

  // Parametric
  ["drop", 0x1A, 0, 0],
  ["select", 0x1B, 0, 0],

  // Variable access
  ["local.get", 0x20, 3, 0],
  ["local.set", 0x21, 3, 0],
  ["local.tee", 0x22, 3, 0],
  ["global.get", 0x23, 4, 0],
  ["global.set", 0x24, 4, 0],

  // Memory loads
  ["i32.load", 0x28, 8, 0],
  ["i64.load", 0x29, 8, 0],
  ["f32.load", 0x2A, 8, 0],
  ["f64.load", 0x2B, 8, 0],
  ["i32.load8_s", 0x2C, 8, 0],
  ["i32.load8_u", 0x2D, 8, 0],
  ["i32.load16_s", 0x2E, 8, 0],
  ["i32.load16_u", 0x2F, 8, 0],
  ["i64.load8_s", 0x30, 8, 0],
  ["i64.load8_u", 0x31, 8, 0],
  ["i64.load16_s", 0x32, 8, 0],
  ["i64.load16_u", 0x33, 8, 0],
  ["i64.load32_s", 0x34, 8, 0],
  ["i64.load32_u", 0x35, 8, 0],

  // Memory stores
  ["i32.store", 0x36, 8, 0],
  ["i64.store", 0x37, 8, 0],
  ["f32.store", 0x38, 8, 0],
  ["f64.store", 0x39, 8, 0],
  ["i32.store8", 0x3A, 8, 0],
  ["i32.store16", 0x3B, 8, 0],
  ["i64.store8", 0x3C, 8, 0],
  ["i64.store16", 0x3D, 8, 0],
  ["i64.store32", 0x3E, 8, 0],

  // Memory management
  ["memory.size", 0x3F, 11, 0],
  ["memory.grow", 0x40, 11, 0],

  // Constants
  ["i32.const", 0x41, 1, 0],
  ["i64.const", 0x42, 2, 0],
  ["f32.const", 0x43, 12, 0],
  ["f64.const", 0x44, 12, 0],

  // i32 comparison
  ["i32.eqz", 0x45, 0, 0],
  ["i32.eq", 0x46, 0, 0],
  ["i32.ne", 0x47, 0, 0],
  ["i32.lt_s", 0x48, 0, 0],
  ["i32.lt_u", 0x49, 0, 0],
  ["i32.gt_s", 0x4A, 0, 0],
  ["i32.gt_u", 0x4B, 0, 0],
  ["i32.le_s", 0x4C, 0, 0],
  ["i32.le_u", 0x4D, 0, 0],
  ["i32.ge_s", 0x4E, 0, 0],
  ["i32.ge_u", 0x4F, 0, 0],

  // i64 comparison
  ["i64.eqz", 0x50, 0, 0],
  ["i64.eq", 0x51, 0, 0],
  ["i64.ne", 0x52, 0, 0],
  ["i64.lt_s", 0x53, 0, 0],
  ["i64.lt_u", 0x54, 0, 0],
  ["i64.gt_s", 0x55, 0, 0],
  ["i64.gt_u", 0x56, 0, 0],
  ["i64.le_s", 0x57, 0, 0],
  ["i64.le_u", 0x58, 0, 0],
  ["i64.ge_s", 0x59, 0, 0],
  ["i64.ge_u", 0x5A, 0, 0],

  // f32 comparison
  ["f32.eq", 0x5B, 0, 0],
  ["f32.ne", 0x5C, 0, 0],
  ["f32.lt", 0x5D, 0, 0],
  ["f32.gt", 0x5E, 0, 0],
  ["f32.le", 0x5F, 0, 0],
  ["f32.ge", 0x60, 0, 0],

  // f64 comparison
  ["f64.eq", 0x61, 0, 0],
  ["f64.ne", 0x62, 0, 0],
  ["f64.lt", 0x63, 0, 0],
  ["f64.gt", 0x64, 0, 0],
  ["f64.le", 0x65, 0, 0],
  ["f64.ge", 0x66, 0, 0],

  // i32 unary
  ["i32.clz", 0x67, 0, 0],
  ["i32.ctz", 0x68, 0, 0],
  ["i32.popcnt", 0x69, 0, 0],

  // i32 binary
  ["i32.add", 0x6A, 0, 0],
  ["i32.sub", 0x6B, 0, 0],
  ["i32.mul", 0x6C, 0, 0],
  ["i32.div_s", 0x6D, 0, 0],
  ["i32.div_u", 0x6E, 0, 0],
  ["i32.rem_s", 0x6F, 0, 0],
  ["i32.rem_u", 0x70, 0, 0],
  ["i32.and", 0x71, 0, 0],
  ["i32.or", 0x72, 0, 0],
  ["i32.xor", 0x73, 0, 0],
  ["i32.shl", 0x74, 0, 0],
  ["i32.shr_s", 0x75, 0, 0],
  ["i32.shr_u", 0x76, 0, 0],
  ["i32.rotl", 0x77, 0, 0],
  ["i32.rotr", 0x78, 0, 0],

  // i64 unary
  ["i64.clz", 0x79, 0, 0],
  ["i64.ctz", 0x7A, 0, 0],
  ["i64.popcnt", 0x7B, 0, 0],

  // i64 binary
  ["i64.add", 0x7C, 0, 0],
  ["i64.sub", 0x7D, 0, 0],
  ["i64.mul", 0x7E, 0, 0],
  ["i64.div_s", 0x7F, 0, 0],
  ["i64.div_u", 0x80, 0, 0],
  ["i64.rem_s", 0x81, 0, 0],
  ["i64.rem_u", 0x82, 0, 0],
  ["i64.and", 0x83, 0, 0],
  ["i64.or", 0x84, 0, 0],
  ["i64.xor", 0x85, 0, 0],
  ["i64.shl", 0x86, 0, 0],
  ["i64.shr_s", 0x87, 0, 0],
  ["i64.shr_u", 0x88, 0, 0],
  ["i64.rotl", 0x89, 0, 0],
  ["i64.rotr", 0x8A, 0, 0],

  // f32 unary
  ["f32.abs", 0x8B, 0, 0],
  ["f32.neg", 0x8C, 0, 0],
  ["f32.ceil", 0x8D, 0, 0],
  ["f32.floor", 0x8E, 0, 0],
  ["f32.trunc", 0x8F, 0, 0],
  ["f32.nearest", 0x90, 0, 0],
  ["f32.sqrt", 0x91, 0, 0],

  // f32 binary
  ["f32.add", 0x92, 0, 0],
  ["f32.sub", 0x93, 0, 0],
  ["f32.mul", 0x94, 0, 0],
  ["f32.div", 0x95, 0, 0],
  ["f32.min", 0x96, 0, 0],
  ["f32.max", 0x97, 0, 0],
  ["f32.copysign", 0x98, 0, 0],

  // f64 unary
  ["f64.abs", 0x99, 0, 0],
  ["f64.neg", 0x9A, 0, 0],
  ["f64.ceil", 0x9B, 0, 0],
  ["f64.floor", 0x9C, 0, 0],
  ["f64.trunc", 0x9D, 0, 0],
  ["f64.nearest", 0x9E, 0, 0],
  ["f64.sqrt", 0x9F, 0, 0],

  // f64 binary
  ["f64.add", 0xA0, 0, 0],
  ["f64.sub", 0xA1, 0, 0],
  ["f64.mul", 0xA2, 0, 0],
  ["f64.div", 0xA3, 0, 0],
  ["f64.min", 0xA4, 0, 0],
  ["f64.max", 0xA5, 0, 0],
  ["f64.copysign", 0xA6, 0, 0],

  // Conversions
  ["i32.wrap_i64", 0xA7, 0, 0],
  ["i32.trunc_f32_s", 0xA8, 0, 0],
  ["i32.trunc_f32_u", 0xA9, 0, 0],
  ["i32.trunc_f64_s", 0xAA, 0, 0],
  ["i32.trunc_f64_u", 0xAB, 0, 0],
  ["i64.extend_i32_s", 0xAC, 0, 0],
  ["i64.extend_i32_u", 0xAD, 0, 0],
  ["i64.trunc_f32_s", 0xAE, 0, 0],
  ["i64.trunc_f32_u", 0xAF, 0, 0],
  ["i64.trunc_f64_s", 0xB0, 0, 0],
  ["i64.trunc_f64_u", 0xB1, 0, 0],
  ["f32.convert_i32_s", 0xB2, 0, 0],
  ["f32.convert_i32_u", 0xB3, 0, 0],
  ["f32.convert_i64_s", 0xB4, 0, 0],
  ["f32.convert_i64_u", 0xB5, 0, 0],
  ["f32.demote_f64", 0xB6, 0, 0],
  ["f64.convert_i32_s", 0xB7, 0, 0],
  ["f64.convert_i32_u", 0xB8, 0, 0],
  ["f64.convert_i64_s", 0xB9, 0, 0],
  ["f64.convert_i64_u", 0xBA, 0, 0],
  ["f64.promote_f32", 0xBB, 0, 0],

  // Reinterpretations
  ["i32.reinterpret_f32", 0xBC, 0, 0],
  ["i64.reinterpret_f64", 0xBD, 0, 0],
  ["f32.reinterpret_i32", 0xBE, 0, 0],
  ["f64.reinterpret_i64", 0xBF, 0, 0],

  // Sign extension
  ["i32.extend8_s", 0xC0, 0, 0],
  ["i32.extend16_s", 0xC1, 0, 0],
  ["i64.extend8_s", 0xC2, 0, 0],
  ["i64.extend16_s", 0xC3, 0, 0],
  ["i64.extend32_s", 0xC4, 0, 0],

  // i32x4 SIMD (0xFD prefix)
  ["i32x4.splat", 0x10, 0, 2],
  ["i32x4.extract_lane", 0x15, 16, 2],
  ["i32x4.eq", 0x2B, 0, 2],

  // i8x16 SIMD (0xFD prefix)
  ["i8x16.splat", 0x0C, 0, 2],
  ["i8x16.extract_lane_u", 0x11, 16, 2],
  ["i8x16.add", 0x3E, 0, 2],
  ["i8x16.sub", 0x42, 0, 2],
  ["i8x16.eq", 0x23, 0, 2],
  ["i8x16.all_true", 0x55, 0, 2],
  ["i8x16.bitmask", 0x58, 0, 2],
  ["i8x16.shl", 0x4E, 0, 2],
  ["i8x16.shr_u", 0x50, 0, 2],
  ["i8x16.shuffle", 0x0D, 17, 2],
  ["i8x16.ge_u", 0x2A, 0, 2],
  ["i8x16.gt_u", 0x29, 0, 2],
  ["i8x16.le_u", 0x28, 0, 2],

  // v128 SIMD (0xFD prefix)
  ["v128.load", 0x04, 8, 2],
  ["v128.store", 0x05, 8, 2],
  ["v128.and", 0x4E, 0, 2],
  ["v128.or", 0x4F, 0, 2],
  ["v128.xor", 0x50, 0, 2],
  ["v128.bitselect", 0x4B, 0, 2],
  ["v128.any_true", 0x4D, 0, 2],
];

// Generate the WAT data block
function genDataBlock(entries) {
  const parts = [];
  for (const [kw, opcode, immType, flags] of entries) {
    parts.push(String.fromCharCode(kw.length, opcode, immType, flags));
    for (let i = 0; i < kw.length; i++) {
      parts.push(kw[i]);
    }
  }
  // End marker: len=0
  parts.push('\x00');
  return parts.join('');
}

const data = genDataBlock(TABLE);
// Convert to hex escape string
let hexStr = '';
for (let i = 0; i < data.length; i++) {
  hexStr += '\\' + data.charCodeAt(i).toString(16).padStart(2, '0');
}

const wat = `;; Auto-generated opcode table for WAT parser
;; Format: len(1) + opcode(1) + imm_type(1) + flags(1) + keyword(len)
;; End marker: len=0
(data (i32.const {{WAT_OPCODE_LUT}}) "${hexStr}")
`;

ensureDir('out/gen');
writeText('out/gen/opcode-table.wat', wat);
console.log(`✓ out/gen/opcode-table.wat (${TABLE.length} entries, ${data.length} bytes)`);
