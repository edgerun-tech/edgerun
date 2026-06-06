
  ;; CP1252 (Windows-1252) decode table.
  ;; Bytes 0x80-0x9F map to Unicode characters via a lookup table.
  ;; All other bytes 0x00-0x7F and 0xA0-0xFF pass through as-is (Latin-1).
  ;; Exports: cp1252_decode_byte(byte) -> i32 (Unicode codepoint)
  ;;          cp1252_decode_string(input_ptr, input_len, output_ptr) -> output_len
  ;; CP1252 decode table: 32 entries × 2 bytes (little-endian i16) at offset 0
  ;; Index = byte - 0x80
  (data (i32.const 0)
    "\ac\20"  ;; 0x80  € U+20AC
    "\00\00"  ;; 0x81  (unused)
    "\1a\20"  ;; 0x82  ‚ U+201A
    "\92\01"  ;; 0x83  ƒ U+0192
    "\1e\20"  ;; 0x84  „ U+201E
    "\26\20"  ;; 0x85  … U+2026
    "\20\20"  ;; 0x86  † U+2020
    "\21\20"  ;; 0x87  ‡ U+2021
    "\c6\02"  ;; 0x88  ˆ U+02C6
    "\30\20"  ;; 0x89  ‰ U+2030
    "\60\01"  ;; 0x8A  Š U+0160
    "\39\20"  ;; 0x8B  ‹ U+2039
    "\52\01"  ;; 0x8C  Œ U+0152
    "\00\00"  ;; 0x8D  (unused)
    "\7d\01"  ;; 0x8E  Ž U+017D
    "\00\00"  ;; 0x8F  (unused)
    "\00\00"  ;; 0x90  (unused)
    "\18\20"  ;; 0x91  ' U+2018
    "\19\20"  ;; 0x92  ' U+2019
    "\1c\20"  ;; 0x93  " U+201C
    "\1d\20"  ;; 0x94  " U+201D
    "\22\20"  ;; 0x95  • U+2022
    "\13\20"  ;; 0x96  – U+2013
    "\14\20"  ;; 0x97  — U+2014
    "\dc\02"  ;; 0x98  ˜ U+02DC
    "\22\21"  ;; 0x99  ™ U+2122
    "\61\01"  ;; 0x9A  š U+0161
    "\3a\20"  ;; 0x9B  › U+203A
    "\53\01"  ;; 0x9C  œ U+0153
    "\00\00"  ;; 0x9D  (unused)
    "\7e\01"  ;; 0x9E  ž U+017E
    "\78\01"  ;; 0x9F  Ÿ U+0178
  )
  (func $cp1252_decode_byte (export "cp1252_decode_byte") (param $byte i32) (result i32)
    (local $cp i32)
    local.get $byte i32.const 128 i32.ge_u
    if
      local.get $byte i32.const 160 i32.lt_u
      if
        local.get $byte i32.const 128 i32.sub i32.const 1 i32.shl
        i32.load16_u local.tee $cp
        if (result i32)
          local.get $cp
        else
          i32.const 63
        end
        return
      end
    end
    local.get $byte
  )

  (func (export "cp1252_decode_string") (param $in i32) (param $in_len i32) (param $out i32) (result i32)
    (local $i i32) (local $out_pos i32) (local $b i32) (local $cp i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $out_pos
    block $done
    loop $loop
      local.get $i local.get $in_len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.tee $b
      i32.eqz if
        local.get $i i32.const 1 i32.add local.set $i
        br $done
      end
      local.get $b call $cp1252_decode_byte local.set $cp
      local.get $out local.get $out_pos i32.add
      local.get $cp i32.const 0xff i32.and i32.store8
      local.get $cp i32.const 8 i32.shr_u i32.const 0xff i32.and
      if
        local.get $out local.get $out_pos i32.const 1 i32.add i32.add
        local.get $cp i32.const 8 i32.shr_u i32.store8
        local.get $out_pos i32.const 2 i32.add local.set $out_pos
      else
        local.get $out_pos i32.const 1 i32.add local.set $out_pos
      end
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $out_pos
  )
