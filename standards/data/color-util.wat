(import "math" "clamp" (func $clamp (param i32 i32 i32) (result i32)))

;; Color utilities — hex/ARGB color parsing, formatting, lerp, alpha compositing.
  ;;
  ;; Exports:
  ;;   hex_to_rgb(hex_ptr, hex_len) -> i32  (packed 0x00RRGGBB, -1 on error)
  ;;   hex_to_argb(hex_ptr, hex_len) -> i32 (packed 0xAARRGGBB, -1 on error)
  ;;   color_lerp(color_a, color_b, t_fixed) -> i32  (t_fixed = 0..256)
  ;;   color_with_alpha(color, alpha) -> i32
  ;;   constrain_byte(val) -> i32
  ;;   rgb_to_hex(r, g, b, out_ptr) -> out_ptr + 7 ("#RRGGBB\0")
  ;;   argb_to_hex(a, r, g, b, out_ptr) -> out_ptr + 9 ("#AARRGGBB\0")
  (func $m49hex_digit (param $c i32) (result i32)
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
    end
  )

  (func $hex_parse (param $ptr i32) (param $len i32) (param $skip_prefix i32) (result i32)
    (local $i i32) (local $val i32) (local $d i32)
    local.get $skip_prefix if
      local.get $len i32.const 2 i32.gt_u
      if
        local.get $ptr i32.load8_u i32.const 35 i32.eq  ;; '#'
        if
          local.get $ptr i32.const 1 i32.add local.set $ptr
          local.get $len i32.const 1 i32.sub local.set $len
        else
          local.get $ptr i32.load16_u i32.const 0x7830 i32.eq  ;; "0x"
          if
            local.get $ptr i32.const 2 i32.add local.set $ptr
            local.get $len i32.const 2 i32.sub local.set $len
          end
        end
      end
    end
    i32.const 0 local.set $val
    i32.const 0 local.set $i
    block $bad
    loop $loop
      local.get $i local.get $len i32.ge_u if
        local.get $val return
      end
      local.get $ptr local.get $i i32.add i32.load8_u call $m49hex_digit
      local.tee $d
      i32.const 0 i32.lt_s if
        i32.const -1 return
      end
      local.get $val i32.const 4 i32.shl local.get $d i32.or local.set $val
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $val
  )

  (func (export "hex_to_rgb") (param $ptr i32) (param $len i32) (result i32)
    (local $val i32)
    local.get $ptr local.get $len i32.const 1 call $hex_parse
    local.tee $val
    i32.const 0 i32.ge_s if
      local.get $val i32.const 0xFFFFFF i32.and return
    end
    i32.const -1
  )

  (func (export "hex_to_argb") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len i32.const 1 call $hex_parse
  )

  (func $constrain_byte (export "constrain_byte") (param $val i32) (result i32)
    (call $clamp (local.get $val) (i32.const 0) (i32.const 255)))

  (func (export "color_with_alpha") (param $color i32) (param $alpha i32) (result i32)
    local.get $alpha call $constrain_byte
    i32.const 24 i32.shl
    local.get $color i32.const 0x00FFFFFF i32.and
    i32.or
  )

  (func (export "color_lerp") (param $c_a i32) (param $c_b i32) (param $t i32) (result i32)
    (local $a_r i32) (local $a_g i32) (local $a_b i32) (local $a_a i32)
    (local $b_r i32) (local $b_g i32) (local $b_b i32) (local $b_a i32)
    (local $r i32) (local $g i32) (local $b i32) (local $alpha i32)
    local.get $c_a i32.const 16 i32.shr_u i32.const 0xff i32.and local.set $a_r
    local.get $c_a i32.const 8 i32.shr_u i32.const 0xff i32.and local.set $a_g
    local.get $c_a i32.const 0xff i32.and local.set $a_b
    local.get $c_a i32.const 24 i32.shr_u i32.const 0xff i32.and local.set $a_a
    local.get $c_b i32.const 16 i32.shr_u i32.const 0xff i32.and local.set $b_r
    local.get $c_b i32.const 8 i32.shr_u i32.const 0xff i32.and local.set $b_g
    local.get $c_b i32.const 0xff i32.and local.set $b_b
    local.get $c_b i32.const 24 i32.shr_u i32.const 0xff i32.and local.set $b_a
    local.get $a_r local.get $b_r local.get $a_r i32.sub local.get $t i32.mul i32.const 256 i32.div_s i32.add local.set $r
    local.get $a_g local.get $b_g local.get $a_g i32.sub local.get $t i32.mul i32.const 256 i32.div_s i32.add local.set $g
    local.get $a_b local.get $b_b local.get $a_b i32.sub local.get $t i32.mul i32.const 256 i32.div_s i32.add local.set $b
    local.get $a_a local.get $b_a local.get $a_a i32.sub local.get $t i32.mul i32.const 256 i32.div_s i32.add local.set $alpha
    local.get $alpha call $constrain_byte i32.const 24 i32.shl
    local.get $r call $constrain_byte i32.const 16 i32.shl i32.or
    local.get $g call $constrain_byte i32.const 8 i32.shl i32.or
    local.get $b call $constrain_byte i32.or
  )

  (func $m49hex_digit_char (param $d i32) (param $out i32)
    local.get $d i32.const 10 i32.lt_s
    if
      local.get $out local.get $d i32.const 48 i32.add i32.store8
    else
      local.get $out local.get $d i32.const 87 i32.add i32.store8
    end
  )

  (func (export "rgb_to_hex") (param $r i32) (param $g i32) (param $b i32) (param $out i32) (result i32)
    local.get $out i32.const 35 i32.store8  ;; '#'
    local.get $r i32.const 4 i32.shr_u local.get $out i32.const 1 i32.add call $m49hex_digit_char
    local.get $r i32.const 0xf i32.and local.get $out i32.const 2 i32.add call $m49hex_digit_char
    local.get $g i32.const 4 i32.shr_u local.get $out i32.const 3 i32.add call $m49hex_digit_char
    local.get $g i32.const 0xf i32.and local.get $out i32.const 4 i32.add call $m49hex_digit_char
    local.get $b i32.const 4 i32.shr_u local.get $out i32.const 5 i32.add call $m49hex_digit_char
    local.get $b i32.const 0xf i32.and local.get $out i32.const 6 i32.add i32.store8
    local.get $out i32.const 7 i32.add
  )

  (func (export "argb_to_hex") (param $a i32) (param $r i32) (param $g i32) (param $b i32) (param $out i32) (result i32)
    local.get $out i32.const 35 i32.store8  ;; '#'
    local.get $a i32.const 4 i32.shr_u local.get $out i32.const 1 i32.add call $m49hex_digit_char
    local.get $a i32.const 0xf i32.and local.get $out i32.const 2 i32.add call $m49hex_digit_char
    local.get $r i32.const 4 i32.shr_u local.get $out i32.const 3 i32.add call $m49hex_digit_char
    local.get $r i32.const 0xf i32.and local.get $out i32.const 4 i32.add call $m49hex_digit_char
    local.get $g i32.const 4 i32.shr_u local.get $out i32.const 5 i32.add call $m49hex_digit_char
    local.get $g i32.const 0xf i32.and local.get $out i32.const 6 i32.add call $m49hex_digit_char
    local.get $b i32.const 4 i32.shr_u local.get $out i32.const 7 i32.add call $m49hex_digit_char
    local.get $b i32.const 0xf i32.and local.get $out i32.const 8 i32.add i32.store8
    local.get $out i32.const 9 i32.add
  )
