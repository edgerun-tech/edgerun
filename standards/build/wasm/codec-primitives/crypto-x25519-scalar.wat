(module
  (memory (export "memory") 2)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300080)

  (func $pack (param $status i32) (param $written i32) (result i64)
    local.get $written
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $status
    i64.extend_i32_u
    i64.or)

  (func $range_ok (param $ptr i32) (param $len i32) (result i32)
    (local $end i32)
    local.get $ptr
    local.get $len
    i32.add
    local.set $end
    local.get $end
    local.get $ptr
    i32.lt_u
    if
      i32.const 0
      return
    end
    local.get $end
    i32.const 131072
    i32.le_u)

  (func $nonzero32 (param $ptr i32) (result i32)
    (local $i i32)
    (local $acc i32)
    i32.const 0
    local.set $i
    i32.const 0
    local.set $acc
    loop $scan
      local.get $acc
      local.get $ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.or
      local.set $acc
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      i32.const 32
      i32.lt_u
      br_if $scan
    end
    local.get $acc
    i32.const 0
    i32.ne)

  (func $key_status (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 32
    i32.ne
    if
      i32.const 2
      return
    end
    local.get $ptr
    i32.const 32
    call $range_ok
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $ptr
    call $nonzero32
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0)

  (func (export "x25519_clamp_scalar") (param $in i32) (param $len i32) (param $out i32) (result i64)
    (local $i i32)
    (local $b i32)
    local.get $len
    i32.const 32
    i32.ne
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $in
    i32.const 32
    call $range_ok
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $out
    i32.const 32
    call $range_ok
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    i32.const 0
    local.set $i
    loop $copy
      local.get $in
      local.get $i
      i32.add
      i32.load8_u
      local.set $b
      local.get $i
      i32.eqz
      if
        local.get $b
        i32.const 248
        i32.and
        local.set $b
      end
      local.get $i
      i32.const 31
      i32.eq
      if
        local.get $b
        i32.const 127
        i32.and
        i32.const 64
        i32.or
        local.set $b
      end
      local.get $out
      local.get $i
      i32.add
      local.get $b
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.tee $i
      i32.const 32
      i32.lt_u
      br_if $copy
    end
    i32.const 0
    i32.const 32
    call $pack)

  (func (export "x25519_public_key_status") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $key_status)

  (func (export "x25519_shared_secret_status") (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr
    local.get $len
    call $key_status))
