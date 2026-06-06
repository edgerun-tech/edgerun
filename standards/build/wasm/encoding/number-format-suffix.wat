(module
  (import "edgerun-core" "memory" (memory 1))

  ;; SI-suffixed number formatting (K/M/B) and comma-delimited format.
  ;;
  ;; Exports:
  ;;   format_suffix(val, out_ptr) -> out_len  (e.g. 1234567 -> "1.23M")
  ;;   format_number(val, out_ptr) -> out_len  (e.g. 1234567 -> "1,234,567")
  ;;   parse_suffix(in_ptr, in_len) -> i64     (e.g. "1.23M" -> 1230000)
  (func (export "proto_standard_id") (result i32) i32.const 300528)

  (func $format_number (export "format_number") (param $val i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $buf i32) (local $neg i32) (local $d i32) (local $n i32)
    local.get $val i32.const 0 i32.lt_s
    if
      i32.const 1 local.set $neg
      i32.const 0 local.get $val i32.sub local.set $val
    end
    i32.const 256 local.set $buf
    i32.const 0 local.set $i
    block $done_digits
    loop $digits
      local.get $val i32.const 10 i32.div_u local.set $n
      local.get $val local.get $n i32.const 10 i32.mul i32.sub local.set $d
      local.get $buf local.get $i i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      local.get $n local.set $val
      local.get $val br_if $digits
    end
    end
    i32.const 0 local.set $o
    local.get $neg if
      local.get $out local.get $o i32.add i32.const 45 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    block $done_out
    loop $out_loop
      local.get $i i32.const 0 i32.le_s br_if $done_out
      local.get $i i32.const 1 i32.sub local.set $i
      local.get $o i32.const 3 i32.gt_u
      local.get $i i32.const 0 i32.gt_s i32.and
      local.get $i i32.const 3 i32.rem_u i32.eqz i32.and
      if
        local.get $out local.get $o i32.add i32.const 44 i32.store8
        local.get $o i32.const 1 i32.add local.set $o
      end
      local.get $out local.get $o i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      br $out_loop
    end
    end
    local.get $o
  )

  (func $write_frac (param $out_ptr i32) (param $num i32) (param $div i32)
    (local $d i32)
    local.get $num local.get $div i32.const 100 i32.div_u i32.div_u
    local.tee $d
    if
      local.get $out_ptr i32.const 46 i32.store8  ;; '.'
      local.get $d i32.const 10 i32.lt_u
      if
        local.get $out_ptr i32.const 1 i32.add i32.const 48 i32.store8  ;; '0'
        local.get $out_ptr i32.const 2 i32.add local.get $d i32.const 48 i32.add i32.store8
      else
        local.get $out_ptr i32.const 1 i32.add
        local.get $d i32.const 10 i32.div_u i32.const 48 i32.add i32.store8
        local.get $out_ptr i32.const 2 i32.add
        local.get $d i32.const 10 i32.rem_u i32.const 48 i32.add i32.store8
      end
    end
  )

  (func (export "format_suffix") (param $val i32) (param $out i32) (result i32)
    (local $abs i32) (local $suffix i32) (local $div i32) (local $d i32)
    (local $o i32) (local $neg i32)
    local.get $val i32.const 0 i32.lt_s
    if
      i32.const 1 local.set $neg
      i32.const 0 local.get $val i32.sub local.set $abs
    else
      local.get $val local.set $abs
    end
    local.get $abs i32.const 10000 i32.lt_u
    if
      local.get $val local.get $out call $format_number return
    end
    i32.const 0 local.set $suffix
    local.get $abs i32.const 1000000000 i32.ge_u
    if
      i32.const 3 local.set $suffix
      i32.const 1000000000 local.set $div
    else
      local.get $abs i32.const 1000000 i32.ge_u
      if
        i32.const 2 local.set $suffix
        i32.const 1000000 local.set $div
      else
        i32.const 1 local.set $suffix
        i32.const 1000 local.set $div
      end
    end
    i32.const 0 local.set $o
    local.get $neg if
      local.get $out local.get $o i32.add i32.const 45 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    local.get $abs local.get $div i32.div_u local.set $d
    local.get $d i32.const 100 i32.lt_u
    if
      local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $abs local.get $d local.get $div i32.mul i32.sub
      local.get $out local.get $o i32.add local.get $div call $write_frac
    else
      local.get $out local.get $o i32.add
      local.get $d i32.const 10 i32.div_u i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $d i32.const 10 i32.rem_u
      local.tee $d
      if
        local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
        local.get $o i32.const 1 i32.add local.set $o
      end
      local.get $out local.get $o i32.add i32.const 46 i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $abs local.get $d local.get $div i32.mul i32.sub
      local.get $div i32.const 10 i32.div_u i32.div_u
      local.set $d
      local.get $out local.get $o i32.add local.get $d i32.const 48 i32.add i32.store8
      local.get $o i32.const 1 i32.add local.set $o
    end
    local.get $suffix
    if (result i32)
      local.get $out local.get $o i32.add
      local.get $suffix
      i32.const 1 i32.eq if (result i32) i32.const 75 else
      local.get $suffix i32.const 2 i32.eq if (result i32) i32.const 77 else
      i32.const 66
      end end
      i32.store8
      local.get $o i32.const 1 i32.add
    else
      local.get $o
    end
  )

  (func (export "parse_suffix") (param $in i32) (param $len i32) (result i64)
    (local $val i64) (local $i i32) (local $c i32) (local $neg i32)
    (local $decimal i64) (local $div i64) (local $mult i64)
    i64.const 0 local.set $val
    i64.const 1 local.set $div
    i64.const 1 local.set $mult
    i32.const 0 local.set $neg
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 45 i32.eq
      if
        i32.const 1 local.set $neg
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 44 i32.eq
      if
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 46 i32.eq
      if
        i64.const 1 local.set $decimal
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $c i32.const 48 i32.ge_u
      local.get $c i32.const 57 i32.le_u i32.and
      if
        local.get $decimal i64.const 0 i64.ne
        if
          local.get $div i64.const 10 i64.mul local.set $div
        end
        local.get $val i64.const 10 i64.mul
        local.get $c i32.const 48 i32.sub i64.extend_i32_s i64.add local.set $val
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      br $done
    end
    end
    local.get $c i32.const 75 i32.eq
    if
      i64.const 1000 local.set $mult
    else
      local.get $c i32.const 77 i32.eq
      if
        i64.const 1000000 local.set $mult
      else
        local.get $c i32.const 66 i32.eq
        if
          i64.const 1000000000 local.set $mult
        end
      end
    end
    local.get $div i64.const 1 i64.gt_u
    if
      local.get $val local.get $div i64.div_u local.set $val
    end
    local.get $val local.get $mult i64.mul local.set $val
    local.get $neg if
      local.get $val i64.const -1 i64.mul local.set $val
    end
    local.get $val
  )
)