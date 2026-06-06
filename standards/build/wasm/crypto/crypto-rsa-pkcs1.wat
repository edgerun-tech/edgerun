  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300079)


  (func $digest_len (param $alg i32) (result i32)
    local.get $alg
    i32.const 256
    i32.eq
    if
      i32.const 32
      return
    end
    local.get $alg
    i32.const 384
    i32.eq
    if
      i32.const 48
      return
    end
    local.get $alg
    i32.const 512
    i32.eq
    if
      i32.const 64
      return
    end
    i32.const 0)

  (func $prefix_byte (param $alg i32) (param $idx i32) (result i32)
    local.get $idx
    i32.const 0
    i32.eq
    if
      i32.const 0x30
      return
    end
    local.get $idx
    i32.const 1
    i32.eq
    if
      local.get $alg
      i32.const 256
      i32.eq
      if
        i32.const 0x31
        return
      end
      local.get $alg
      i32.const 384
      i32.eq
      if
        i32.const 0x41
        return
      end
      local.get $alg
      i32.const 512
      i32.eq
      if
        i32.const 0x51
        return
      end
      i32.const -1
      return
    end
    local.get $idx
    i32.const 2
    i32.eq
    if
      i32.const 0x30
      return
    end
    local.get $idx
    i32.const 3
    i32.eq
    if
      i32.const 0x0d
      return
    end
    local.get $idx
    i32.const 4
    i32.eq
    if
      i32.const 0x06
      return
    end
    local.get $idx
    i32.const 5
    i32.eq
    if
      i32.const 0x09
      return
    end
    local.get $idx
    i32.const 6
    i32.eq
    if
      i32.const 0x60
      return
    end
    local.get $idx
    i32.const 7
    i32.eq
    if
      i32.const 0x86
      return
    end
    local.get $idx
    i32.const 8
    i32.eq
    if
      i32.const 0x48
      return
    end
    local.get $idx
    i32.const 9
    i32.eq
    if
      i32.const 0x01
      return
    end
    local.get $idx
    i32.const 10
    i32.eq
    if
      i32.const 0x65
      return
    end
    local.get $idx
    i32.const 11
    i32.eq
    if
      i32.const 0x03
      return
    end
    local.get $idx
    i32.const 12
    i32.eq
    if
      i32.const 0x04
      return
    end
    local.get $idx
    i32.const 13
    i32.eq
    if
      local.get $alg
      i32.const 256
      i32.eq
      if
        i32.const 0x02
        return
      end
      local.get $alg
      i32.const 384
      i32.eq
      if
        i32.const 0x02
        return
      end
      local.get $alg
      i32.const 512
      i32.eq
      if
        i32.const 0x02
        return
      end
      i32.const -1
      return
    end
    local.get $idx
    i32.const 14
    i32.eq
    if
      local.get $alg
      i32.const 256
      i32.eq
      if
        i32.const 0x01
        return
      end
      local.get $alg
      i32.const 384
      i32.eq
      if
        i32.const 0x02
        return
      end
      local.get $alg
      i32.const 512
      i32.eq
      if
        i32.const 0x03
        return
      end
      i32.const -1
      return
    end
    local.get $idx
    i32.const 15
    i32.eq
    if
      i32.const 0x05
      return
    end
    local.get $idx
    i32.const 16
    i32.eq
    if
      i32.const 0x00
      return
    end
    local.get $idx
    i32.const 17
    i32.eq
    if
      i32.const 0x04
      return
    end
    local.get $idx
    i32.const 18
    i32.eq
    if
      local.get $alg
      call $digest_len
      return
    end
    i32.const -1)

  (func $write_prefix (param $alg i32) (param $dst i32)
    (local $i i32)
    loop $again
      local.get $i
      i32.const 19
      i32.lt_u
      if
        local.get $dst
        local.get $i
        i32.add
        local.get $alg
        local.get $i
        call $prefix_byte
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $m60copy (param $src i32) (param $dst i32) (param $len i32)
    (local $i i32)
    loop $again
      local.get $i
      local.get $len
      i32.lt_u
      if
        local.get $dst
        local.get $i
        i32.add
        local.get $src
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func (export "rsa_pkcs1_v15_verify")
    (param $hash_alg i32)
    (param $digest_ptr i32)
    (param $digest_len i32)
    (param $em_ptr i32)
    (param $em_len i32)
    (result i32)
    (local $want_digest_len i32)
    (local $t_len i32)
    (local $pad_end i32)
    (local $prefix_start i32)
    (local $digest_start i32)
    (local $i i32)

    local.get $hash_alg
    call $digest_len
    local.tee $want_digest_len
    i32.eqz
    if
      i32.const 1
      return
    end

    local.get $digest_len
    local.get $want_digest_len
    i32.ne
    if
      i32.const 3
      return
    end

    i32.const 19
    local.get $want_digest_len
    i32.add
    local.tee $t_len
    i32.const 11
    i32.add
    local.get $em_len
    i32.gt_u
    if
      i32.const 2
      return
    end

    local.get $em_ptr
    i32.load8_u
    i32.const 0
    i32.ne
    if
      i32.const 3
      return
    end
    local.get $em_ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $em_len
    local.get $t_len
    i32.sub
    i32.const 1
    i32.sub
    local.tee $pad_end
    local.set $i

    i32.const 2
    local.set $i
    loop $pad
      local.get $i
      local.get $pad_end
      i32.lt_u
      if
        local.get $em_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.const 0xff
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $pad
      end
    end

    local.get $em_ptr
    local.get $pad_end
    i32.add
    i32.load8_u
    i32.const 0
    i32.ne
    if
      i32.const 3
      return
    end

    local.get $em_len
    local.get $t_len
    i32.sub
    local.set $prefix_start
    local.get $em_len
    local.get $want_digest_len
    i32.sub
    local.set $digest_start

    i32.const 0
    local.set $i
    loop $prefix
      local.get $i
      i32.const 19
      i32.lt_u
      if
        local.get $em_ptr
        local.get $prefix_start
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        local.get $hash_alg
        local.get $i
        call $prefix_byte
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $prefix
      end
    end

    i32.const 0
    local.set $i
    loop $digest
      local.get $i
      local.get $want_digest_len
      i32.lt_u
      if
        local.get $em_ptr
        local.get $digest_start
        i32.add
        local.get $i
        i32.add
        i32.load8_u
        local.get $digest_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $digest
      end
    end

    i32.const 0)

  (func (export "rsa_pkcs1_v15_emit")
    (param $hash_alg i32)
    (param $digest_ptr i32)
    (param $digest_len i32)
    (param $out_ptr i32)
    (param $out_len i32)
    (result i64)
    (local $want_digest_len i32)
    (local $t_len i32)
    (local $pad_end i32)
    (local $prefix_start i32)
    (local $digest_start i32)
    (local $i i32)

    local.get $hash_alg
    call $digest_len
    local.tee $want_digest_len
    i32.eqz
    if
      i32.const 1
      i32.const 0
      call $pack
      return
    end

    local.get $digest_len
    local.get $want_digest_len
    i32.ne
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    i32.const 19
    local.get $want_digest_len
    i32.add
    local.tee $t_len
    i32.const 11
    i32.add
    local.get $out_len
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $out_ptr
    i32.const 0
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    i32.const 1
    i32.store8

    local.get $out_len
    local.get $t_len
    i32.sub
    i32.const 1
    i32.sub
    local.set $pad_end
    i32.const 2
    local.set $i
    loop $pad
      local.get $i
      local.get $pad_end
      i32.lt_u
      if
        local.get $out_ptr
        local.get $i
        i32.add
        i32.const 0xff
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $pad
      end
    end

    local.get $out_ptr
    local.get $pad_end
    i32.add
    i32.const 0
    i32.store8

    local.get $out_len
    local.get $t_len
    i32.sub
    local.set $prefix_start
    local.get $hash_alg
    local.get $out_ptr
    local.get $prefix_start
    i32.add
    call $write_prefix

    local.get $out_len
    local.get $want_digest_len
    i32.sub
    local.set $digest_start
    local.get $digest_ptr
    local.get $out_ptr
    local.get $digest_start
    i32.add
    local.get $want_digest_len
    call $m60copy

    i32.const 0
    local.get $out_len
    call $pack)