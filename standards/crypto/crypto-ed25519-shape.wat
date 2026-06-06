
(func $m57copy (param $src i32) (param $dst i32) (param $len i32)
    (local $i i32)
    loop $loop
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
        br $loop
      end
    end)

  ;; Status 0 ok, 2 length/output short, 3 invalid.
  (func (export "ed25519_seed_status") (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 32
    i32.eq
    if
      i32.const 0
      return
    end
    i32.const 2)

  ;; Shape-only public key gate: exact 32 bytes and not the all-zero encoding.
  (func (export "ed25519_public_key_status") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $nonzero i32)
    local.get $len
    i32.const 32
    i32.ne
    if
      i32.const 2
      return
    end
    loop $loop
      local.get $i
      i32.const 32
      i32.lt_u
      if
        local.get $nonzero
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.or
        local.set $nonzero
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $nonzero
    i32.eqz
    if
      i32.const 3
      return
    end
    i32.const 0)

  ;; Split a 64-byte Ed25519 signature into the encoded R point and scalar s.
  ;; Returns status | written<<32. A successful split writes 64 bytes total.
  (func (export "ed25519_signature_split")
    (param $sig_ptr i32) (param $sig_len i32) (param $r_out i32) (param $s_out i32)
    (result i64)
    local.get $sig_len
    i32.const 64
    i32.ne
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $sig_ptr
    local.get $r_out
    i32.const 32
    call $m57copy
    local.get $sig_ptr
    i32.const 32
    i32.add
    local.get $s_out
    i32.const 32
    call $m57copy
    i32.const 0
    i32.const 64
    call $pack)

  ;; Prune the lower SHA-512 half used as an Ed25519 expanded secret scalar:
  ;; h[0] &= 248; h[31] &= 127; h[31] |= 64.
  ;; Returns status | written<<32. Input and output are 32 bytes.
  (func (export "ed25519_expanded_secret_prune")
    (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32)
    (result i64)
    local.get $in_len
    i32.const 32
    i32.ne
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $in_ptr
    local.get $out_ptr
    i32.const 32
    call $m57copy
    local.get $out_ptr
    local.get $out_ptr
    i32.load8_u
    i32.const 248
    i32.and
    i32.store8
    local.get $out_ptr
    i32.const 31
    i32.add
    local.get $out_ptr
    i32.const 31
    i32.add
    i32.load8_u
    i32.const 127
    i32.and
    i32.const 64
    i32.or
    i32.store8
    i32.const 0
    i32.const 32
    call $pack)
