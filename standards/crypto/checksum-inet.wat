  ;; Internet checksum — RFC 1071 one's complement 16-bit checksum.
  ;; Exports: inet_checksum(data_ptr, data_len) -> i32 (16-bit checksum)
  (func (export "inet_checksum") (param $ptr i32) (param $len i32) (result i32)
    (local $sum i32) (local $i i32)
    i32.const 0 local.set $sum
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.const 1 i32.sub i32.ge_u br_if $done
      local.get $ptr local.get $i i32.add i32.load8_u i32.const 8 i32.shl
      local.get $ptr local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.or
      local.get $sum i32.add local.set $sum
      local.get $i i32.const 2 i32.add local.set $i
      br $loop
    end
    end
    local.get $i local.get $len i32.lt_u if
      local.get $sum local.get $ptr local.get $i i32.add i32.load8_u i32.const 8 i32.shl i32.add local.set $sum
    end
    local.get $sum i32.const 16 i32.shr_u local.get $sum i32.const 0xffff i32.and i32.add
    local.tee $sum
    i32.const 16 i32.shr_u local.get $sum i32.add
    i32.const 0xffff i32.and
    i32.const 0xffff i32.xor
  )
