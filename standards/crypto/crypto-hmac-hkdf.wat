;; Algorithm profile lookup table at 0x3100 (scratch area)
;; 2 entries × 4 bytes: [block_size, digest_size]
;; Index: (alg >> 7) - 2  (0=SHA-256, 1=SHA-384)
(data (i32.const 0x3100) "\40\00\00\00\20\00\00\00\80\00\00\00\30\00\00\00")

(func $alg_lookup (param $alg i32) (param $field i32) (result i32)
  (local $idx i32)
  local.get $alg i32.const 7 i32.shr_u i32.const 2 i32.sub local.set $idx
  local.get $idx i32.const 0 i32.lt_s if i32.const 0 return end
  local.get $idx i32.const 2 i32.ge_u if i32.const 0 return end
  i32.const 0x3100
  local.get $idx
  i32.const 3
  i32.shl
  local.get $field
  i32.const 2
  i32.shl
  i32.add
  i32.add
  i32.load)

  (func (export "crypto_hmac_profile") (param $alg i32) (param $out_ptr i32) (result i32)
    (local $block i32)
    (local $digest i32)
    local.get $alg
    i32.const 0
    call $alg_lookup
    local.tee $block
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $alg
    i32.const 1
    call $alg_lookup
    local.set $digest
    local.get $out_ptr
    local.get $block
    i32.store
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $digest
    i32.store
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $digest
    i32.const 255
    i32.mul
    i32.store
    i32.const 0)

  (func (export "crypto_hmac_key_pad")
    (param $alg i32)
    (param $key_ptr i32)
    (param $key_len i32)
    (param $ipad_out i32)
    (param $opad_out i32)
    (result i32)
    (local $block i32)
    (local $i i32)
    (local $key_byte i32)
    local.get $alg
    i32.const 0
    call $alg_lookup
    local.tee $block
    i32.eqz
    if
      i32.const 1
      return
    end
    local.get $key_len
    local.get $block
    i32.gt_u
    if
      i32.const 4
      return
    end
    loop $again
      local.get $i
      local.get $block
      i32.lt_u
      if
        i32.const 0
        local.set $key_byte
        local.get $i
        local.get $key_len
        i32.lt_u
        if
          local.get $key_ptr
          local.get $i
          i32.add
          i32.load8_u
          local.set $key_byte
        end
        local.get $ipad_out
        local.get $i
        i32.add
        local.get $key_byte
        i32.const 0x36
        i32.xor
        i32.store8
        local.get $opad_out
        local.get $i
        i32.add
        local.get $key_byte
        i32.const 0x5c
        i32.xor
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end
    i32.const 0)

  (func (export "crypto_hkdf_expand_input")
    (param $alg i32)
    (param $previous_ptr i32)
    (param $previous_len i32)
    (param $info_ptr i32)
    (param $info_len i32)
    (param $counter i32)
    (param $out_ptr i32)
    (param $out_cap i32)
    (result i64)
    (local $digest i32)
    (local $needed i32)
    local.get $alg
    i32.const 1
    call $alg_lookup
    local.tee $digest
    i32.eqz
    if
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    local.get $previous_len
    local.get $digest
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $counter
    i32.const 0
    i32.eq
    local.get $counter
    i32.const 255
    i32.gt_u
    i32.or
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $previous_len
    local.get $info_len
    i32.add
    i32.const 1
    i32.add
    local.tee $needed
    local.get $out_cap
    i32.gt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out_ptr
    local.get $previous_ptr
    local.get $previous_len
    call $memcpy
    local.get $out_ptr
    local.get $previous_len
    i32.add
    local.get $info_ptr
    local.get $info_len
    call $memcpy
    local.get $out_ptr
    local.get $needed
    i32.add
    i32.const 1
    i32.sub
    local.get $counter
    i32.store8
    i32.const 0
    local.get $needed
    call $pack)

  (func (export "crypto_hmac_digest_required_status") (result i32)
    i32.const 4)
