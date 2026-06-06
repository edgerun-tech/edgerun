(module
  ;; Wildcard/glob pattern matching — case-insensitive.
  ;; Supports '*' (match any sequence) and '?' (match single char).
  ;;
  ;; Exports:
  ;;   glob_match(pattern_ptr, pattern_len, str_ptr, str_len) -> i32 (1 = match, 0 = no match)
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300527)

  (func $to_lower (param $c i32) (result i32)
    local.get $c i32.const 65 i32.ge_u
    local.get $c i32.const 90 i32.le_u i32.and
    if
      local.get $c i32.const 32 i32.add return
    end
    local.get $c
  )

  (func (export "glob_match") (param $p i32) (param $pl i32) (param $s i32) (param $sl i32) (result i32)
    (local $pi i32) (local $si i32)
    (local $back_p i32) (local $back_s i32) (local $cp i32) (local $cs i32)
    i32.const 0 local.set $back_p
    i32.const 0 local.set $back_s
    i32.const 0 local.set $pi
    i32.const 0 local.set $si
    block $done
    loop $main
      ;; If at end of pattern, check if we matched all string
      local.get $pi local.get $pl i32.ge_u if
        local.get $si local.get $sl i32.ge_u if
          i32.const 1 return
        end
        ;; If we have a backtrack point, use it
        local.get $back_s if
          local.get $back_s local.set $si
          local.get $back_p local.set $pi
          local.get $back_s i32.const 1 i32.add local.set $back_s
          br $main
        end
        i32.const 0 return
      end
      local.get $p local.get $pi i32.add i32.load8_u local.set $cp
      local.get $cp i32.const 42 i32.eq  ;; '*'
      if
        local.get $pi i32.const 1 i32.add local.set $back_p
        local.get $si local.set $back_s
        local.get $back_p local.set $pi
        br $main
      end
      local.get $si local.get $sl i32.ge_u if
        i32.const 0 return
      end
      local.get $s local.get $si i32.add i32.load8_u local.set $cs
      local.get $cp i32.const 63 i32.eq  ;; '?'
      if
        local.get $pi i32.const 1 i32.add local.set $pi
        local.get $si i32.const 1 i32.add local.set $si
        br $main
      end
      local.get $cp call $to_lower
      local.get $cs call $to_lower
      i32.ne if
        ;; Mismatch — try backtrack
        local.get $back_s if
          local.get $back_s local.set $si
          local.get $back_p local.set $pi
          local.get $back_s i32.const 1 i32.add local.set $back_s
          br $main
        end
        i32.const 0 return
      end
      local.get $pi i32.const 1 i32.add local.set $pi
      local.get $si i32.const 1 i32.add local.set $si
      br $main
    end
    end
    i32.const 0
  )
)
