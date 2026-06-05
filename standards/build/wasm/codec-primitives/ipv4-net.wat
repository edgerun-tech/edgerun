(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300055)

  (func $pack (param $status i32) (param $value i32) (result i64)
    local.get $value
    i64.extend_i32_u
    i64.const 32
    i64.shl
    local.get $status
    i64.extend_i32_u
    i64.or)

  (func (export "ipv4_to_u32")
    (param $ptr i32) (param $len i32) (param $offset i32)
    (result i64)
    (local $start i32)

    local.get $offset
    i32.const 4
    i32.add
    local.get $len
    i32.gt_u
    if
      i32.const 4
      i32.const 0
      call $pack
      return
    end

    local.get $ptr
    local.get $offset
    i32.add
    local.set $start

    i32.const 0
    local.get $start
    i32.load8_u
    i32.const 24
    i32.shl
    local.get $start
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 16
    i32.shl
    i32.or
    local.get $start
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 8
    i32.shl
    i32.or
    local.get $start
    i32.const 3
    i32.add
    i32.load8_u
    i32.or
    call $pack)

  (func (export "ipv4_from_u32")
    (param $value i32) (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    local.get $out_cap
    i32.const 4
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    local.get $value
    i32.const 24
    i32.shr_u
    i32.const 255
    i32.and
    i32.store8

    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $value
    i32.const 16
    i32.shr_u
    i32.const 255
    i32.and
    i32.store8

    local.get $out_ptr
    i32.const 2
    i32.add
    local.get $value
    i32.const 8
    i32.shr_u
    i32.const 255
    i32.and
    i32.store8

    local.get $out_ptr
    i32.const 3
    i32.add
    local.get $value
    i32.const 255
    i32.and
    i32.store8

    i32.const 0
    i32.const 4
    call $pack)

  (func (export "ipv4_broadcast")
    (param $ip i32) (param $mask i32)
    (result i32)
    local.get $ip
    local.get $mask
    i32.const -1
    i32.xor
    i32.or)

  (func (export "ipv4_network")
    (param $ip i32) (param $mask i32)
    (result i32)
    local.get $ip
    local.get $mask
    i32.and)

  (func (export "ipv4_in_network")
    (param $ip i32) (param $network i32) (param $mask i32)
    (result i32)
    local.get $ip
    local.get $mask
    i32.and
    local.get $network
    i32.eq)
)
