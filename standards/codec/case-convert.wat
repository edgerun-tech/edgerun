
  ;; ASCII string case conversion.
  ;;
  ;; Exports:
  ;;   to_title_case(in_ptr, in_len, out_ptr) -> out_len
  ;;     (SNAKE_CASE -> "Title Case", first letter of each '_'-separated word capitalized)
  ;;   to_upper_ascii(in_ptr, in_len, out_ptr) -> out_len
  ;;   to_lower_ascii(in_ptr, in_len, out_ptr) -> out_len
  (func (export "proto_standard_id") (result i32) i32.const 300530)

  (func $m39is_upper (param $c i32) (result i32)
    local.get $c i32.const 65 i32.ge_u
    local.get $c i32.const 90 i32.le_u i32.and
  )



  (func (export "to_lower_ascii") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $out local.get $i i32.add
      local.get $in local.get $i i32.add i32.load8_u call $to_lower i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $len
  )

  (func (export "to_upper_ascii") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $out local.get $i i32.add
      local.get $in local.get $i i32.add i32.load8_u call $to_upper i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $len
  )

  (func (export "to_title_case") (param $in i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32) (local $o i32) (local $c i32) (local $word_start i32)
    i32.const 0 local.set $i
    i32.const 0 local.set $o
    i32.const 1 local.set $word_start
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $in local.get $i i32.add i32.load8_u local.set $c
      local.get $c i32.const 95 i32.eq  ;; '_'
      if
        local.get $word_start if else
          local.get $out local.get $o i32.add i32.const 32 i32.store8  ;; space
          local.get $o i32.const 1 i32.add local.set $o
        end
        i32.const 1 local.set $word_start
        local.get $i i32.const 1 i32.add local.set $i
        br $loop
      end
      local.get $word_start
      if
        local.get $out local.get $o i32.add local.get $c call $to_upper i32.store8
        i32.const 0 local.set $word_start
      else
        local.get $out local.get $o i32.add local.get $c call $to_lower i32.store8
      end
      local.get $o i32.const 1 i32.add local.set $o
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $o
  )
