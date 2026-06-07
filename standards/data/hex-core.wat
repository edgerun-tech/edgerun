;; Hex utility functions — shared core for nibble/hex-char conversions.
  ;; $hex_digit: nibble → lowercase hex char (0-9a-f), writes to memory at $out
  ;; $hex_digit_upper: nibble → uppercase hex char (0-9A-F), writes to memory at $out
  ;; $hex_char_upper: nibble → uppercase hex char value
  ;; $parse_hex_digit: hex char → 0..15, returns -1 on error

  (func $hex_digit (export "hex_digit") (param $d i32) (param $out i32)
    local.get $d i32.const 10 i32.lt_s
    if
      local.get $out local.get $d i32.const 48 i32.add i32.store8
    else
      local.get $out local.get $d i32.const 87 i32.add i32.store8
    end)

  (func $hex_digit_upper (export "hex_digit_upper") (param $d i32) (param $out i32)
    local.get $d i32.const 10 i32.lt_s
    if
      local.get $out local.get $d i32.const 48 i32.add i32.store8
    else
      local.get $out local.get $d i32.const 55 i32.add i32.store8
    end)

  (func $hex_char_upper (export "hex_char_upper") (param $n i32) (result i32)
    local.get $n
    i32.const 10
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 48
      i32.add
    else
      local.get $n
      i32.const 55
      i32.add
    end)

  (func $parse_hex_digit (export "parse_hex_digit") (param $c i32) (result i32)
    (local $d i32)
    local.get $c i32.const 48 i32.sub
    local.tee $d
    i32.const 10 i32.lt_u if (result i32)
      local.get $d
    else
      local.get $d i32.const 32 i32.or i32.const 87 i32.sub
      local.tee $d
      i32.const 0 i32.ge_s local.get $d i32.const 6 i32.lt_s i32.and if (result i32)
        local.get $d i32.const 10 i32.add
      else
        i32.const -1
      end
    end)
