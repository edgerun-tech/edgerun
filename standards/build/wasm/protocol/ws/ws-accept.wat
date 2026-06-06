  (global $m197h0 (mut i32) (i32.const 0))
  (global $m197h1 (mut i32) (i32.const 0))
  (global $m197h2 (mut i32) (i32.const 0))
  (global $m197h3 (mut i32) (i32.const 0))
  (global $m197h4 (mut i32) (i32.const 0))

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300048)


  (func $rotl (param $value i32) (param $bits i32) (result i32)
    local.get $value
    local.get $bits
    i32.rotl)

  (func $b64val (param $ch i32) (result i32)
    local.get $ch
    i32.const 65
    i32.ge_u
    local.get $ch
    i32.const 90
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 65
      i32.sub
      return
    end
    local.get $ch
    i32.const 97
    i32.ge_u
    local.get $ch
    i32.const 122
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 71
      i32.sub
      return
    end
    local.get $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 4
      i32.add
      return
    end
    local.get $ch
    i32.const 43
    i32.eq
    if
      i32.const 62
      return
    end
    local.get $ch
    i32.const 47
    i32.eq
    if
      i32.const 63
      return
    end
    i32.const -1)

  (func $b64char (param $value i32) (result i32)
    local.get $value
    i32.const 26
    i32.lt_u
    if
      local.get $value
      i32.const 65
      i32.add
      return
    end
    local.get $value
    i32.const 52
    i32.lt_u
    if
      local.get $value
      i32.const 71
      i32.add
      return
    end
    local.get $value
    i32.const 62
    i32.lt_u
    if
      local.get $value
      i32.const 4
      i32.sub
      return
    end
    local.get $value
    i32.const 62
    i32.eq
    if
      i32.const 43
      return
    end
    i32.const 47)

  (func $guid_byte (param $index i32) (result i32)
    local.get $index
    i32.const 0
    i32.eq
    if i32.const 50 return end
    local.get $index
    i32.const 1
    i32.eq
    if i32.const 53 return end
    local.get $index
    i32.const 2
    i32.eq
    if i32.const 56 return end
    local.get $index
    i32.const 3
    i32.eq
    if i32.const 69 return end
    local.get $index
    i32.const 4
    i32.eq
    if i32.const 65 return end
    local.get $index
    i32.const 5
    i32.eq
    if i32.const 70 return end
    local.get $index
    i32.const 6
    i32.eq
    if i32.const 65 return end
    local.get $index
    i32.const 7
    i32.eq
    if i32.const 53 return end
    local.get $index
    i32.const 8
    i32.eq
    if i32.const 45 return end
    local.get $index
    i32.const 9
    i32.eq
    if i32.const 69 return end
    local.get $index
    i32.const 10
    i32.eq
    if i32.const 57 return end
    local.get $index
    i32.const 11
    i32.eq
    if i32.const 49 return end
    local.get $index
    i32.const 12
    i32.eq
    if i32.const 52 return end
    local.get $index
    i32.const 13
    i32.eq
    if i32.const 45 return end
    local.get $index
    i32.const 14
    i32.eq
    if i32.const 52 return end
    local.get $index
    i32.const 15
    i32.eq
    if i32.const 55 return end
    local.get $index
    i32.const 16
    i32.eq
    if i32.const 68 return end
    local.get $index
    i32.const 17
    i32.eq
    if i32.const 65 return end
    local.get $index
    i32.const 18
    i32.eq
    if i32.const 45 return end
    local.get $index
    i32.const 19
    i32.eq
    if i32.const 57 return end
    local.get $index
    i32.const 20
    i32.eq
    if i32.const 53 return end
    local.get $index
    i32.const 21
    i32.eq
    if i32.const 67 return end
    local.get $index
    i32.const 22
    i32.eq
    if i32.const 65 return end
    local.get $index
    i32.const 23
    i32.eq
    if i32.const 45 return end
    local.get $index
    i32.const 24
    i32.eq
    if i32.const 67 return end
    local.get $index
    i32.const 25
    i32.eq
    if i32.const 53 return end
    local.get $index
    i32.const 26
    i32.eq
    if i32.const 65 return end
    local.get $index
    i32.const 27
    i32.eq
    if i32.const 66 return end
    local.get $index
    i32.const 28
    i32.eq
    if i32.const 48 return end
    local.get $index
    i32.const 29
    i32.eq
    if i32.const 68 return end
    local.get $index
    i32.const 30
    i32.eq
    if i32.const 67 return end
    local.get $index
    i32.const 31
    i32.eq
    if i32.const 56 return end
    local.get $index
    i32.const 32
    i32.eq
    if i32.const 53 return end
    local.get $index
    i32.const 33
    i32.eq
    if i32.const 66 return end
    local.get $index
    i32.const 34
    i32.eq
    if i32.const 49 return end
    i32.const 49)

  (func $m197message_byte (param $block i32) (param $offset i32) (param $in_ptr i32) (result i32)
    local.get $block
    i32.eqz
    if
      local.get $offset
      i32.const 24
      i32.lt_u
      if
        local.get $in_ptr
        local.get $offset
        i32.add
        i32.load8_u
        return
      end
      local.get $offset
      i32.const 60
      i32.lt_u
      if
        local.get $offset
        i32.const 24
        i32.sub
        call $guid_byte
        return
      end
      local.get $offset
      i32.const 60
      i32.eq
      if
        i32.const 128
        return
      end
      i32.const 0
      return
    end
    local.get $offset
    i32.const 62
    i32.eq
    if
      i32.const 1
      return
    end
    local.get $offset
    i32.const 63
    i32.eq
    if
      i32.const 224
      return
    end
    i32.const 0)

  (func $m197message_word (param $block i32) (param $word i32) (param $in_ptr i32) (result i32)
    (local $offset i32)
    local.get $word
    i32.const 4
    i32.mul
    local.set $offset
    local.get $block
    local.get $offset
    local.get $in_ptr
    call $m197message_byte
    i32.const 24
    i32.shl
    local.get $block
    local.get $offset
    i32.const 1
    i32.add
    local.get $in_ptr
    call $m197message_byte
    i32.const 16
    i32.shl
    i32.or
    local.get $block
    local.get $offset
    i32.const 2
    i32.add
    local.get $in_ptr
    call $m197message_byte
    i32.const 8
    i32.shl
    i32.or
    local.get $block
    local.get $offset
    i32.const 3
    i32.add
    local.get $in_ptr
    call $m197message_byte
    i32.or)

  (func $sha1_compress (param $block i32) (param $in_ptr i32)
    (local $wptr i32)
    (local $t i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    (local $d i32)
    (local $e i32)
    (local $f i32)
    (local $k i32)
    (local $temp i32)
    (local.set $wptr (i32.const 60000))
    (local.set $t (i32.const 0))
    (loop $load_words
      (i32.store
        (i32.add (local.get $wptr) (i32.mul (local.get $t) (i32.const 4)))
        (call $m197message_word (local.get $block) (local.get $t) (local.get $in_ptr)))
      (local.set $t (i32.add (local.get $t) (i32.const 1)))
      (br_if $load_words (i32.lt_u (local.get $t) (i32.const 16))))
    (loop $expand_words
      (i32.store
        (i32.add (local.get $wptr) (i32.mul (local.get $t) (i32.const 4)))
        (call $rotl
          (i32.xor
            (i32.xor
              (i32.load (i32.add (local.get $wptr) (i32.mul (i32.sub (local.get $t) (i32.const 3)) (i32.const 4))))
              (i32.load (i32.add (local.get $wptr) (i32.mul (i32.sub (local.get $t) (i32.const 8)) (i32.const 4)))))
            (i32.xor
              (i32.load (i32.add (local.get $wptr) (i32.mul (i32.sub (local.get $t) (i32.const 14)) (i32.const 4))))
              (i32.load (i32.add (local.get $wptr) (i32.mul (i32.sub (local.get $t) (i32.const 16)) (i32.const 4))))))
          (i32.const 1)))
      (local.set $t (i32.add (local.get $t) (i32.const 1)))
      (br_if $expand_words (i32.lt_u (local.get $t) (i32.const 80))))
    global.get $m197h0
    local.set $a
    global.get $m197h1
    local.set $b
    global.get $m197h2
    local.set $c
    global.get $m197h3
    local.set $d
    global.get $m197h4
    local.set $e
    (local.set $t (i32.const 0))
    (loop $round
      local.get $t
      i32.const 20
      i32.lt_u
      if
        local.get $b
        local.get $c
        i32.and
        local.get $b
        i32.const -1
        i32.xor
        local.get $d
        i32.and
        i32.or
        local.set $f
        i32.const 0x5a827999
        local.set $k
      else
        local.get $t
        i32.const 40
        i32.lt_u
        if
          local.get $b
          local.get $c
          i32.xor
          local.get $d
          i32.xor
          local.set $f
          i32.const 0x6ed9eba1
          local.set $k
        else
          local.get $t
          i32.const 60
          i32.lt_u
          if
            local.get $b
            local.get $c
            i32.and
            local.get $b
            local.get $d
            i32.and
            i32.or
            local.get $c
            local.get $d
            i32.and
            i32.or
            local.set $f
            i32.const 0x8f1bbcdc
            local.set $k
          else
            local.get $b
            local.get $c
            i32.xor
            local.get $d
            i32.xor
            local.set $f
            i32.const 0xca62c1d6
            local.set $k
          end
        end
      end
      (local.set $temp
        (i32.add
          (i32.add
            (i32.add
              (i32.add
                (call $rotl (local.get $a) (i32.const 5))
                (local.get $f))
              (local.get $e))
            (local.get $k))
          (i32.load (i32.add (local.get $wptr) (i32.mul (local.get $t) (i32.const 4))))))
      local.get $d
      local.set $e
      local.get $c
      local.set $d
      (call $rotl
        (local.get $b)
        (i32.const 30))
      local.set $c
      local.get $a
      local.set $b
      local.get $temp
      local.set $a
      (local.set $t (i32.add (local.get $t) (i32.const 1)))
      (br_if $round (i32.lt_u (local.get $t) (i32.const 80))))
    global.get $m197h0
    local.get $a
    i32.add
    global.set $m197h0
    global.get $m197h1
    local.get $b
    i32.add
    global.set $m197h1
    global.get $m197h2
    local.get $c
    i32.add
    global.set $m197h2
    global.get $m197h3
    local.get $d
    i32.add
    global.set $m197h3
    global.get $m197h4
    local.get $e
    i32.add
    global.set $m197h4)

  (func $m197digest_byte (param $index i32) (result i32)
    (local $word i32)
    local.get $index
    i32.const 4
    i32.div_u
    if (result i32)
      local.get $index
      i32.const 4
      i32.div_u
      i32.const 1
      i32.eq
      if (result i32)
        global.get $m197h1
      else
        local.get $index
        i32.const 4
        i32.div_u
        i32.const 2
        i32.eq
        if (result i32)
          global.get $m197h2
        else
          local.get $index
          i32.const 4
          i32.div_u
          i32.const 3
          i32.eq
          if (result i32)
            global.get $m197h3
          else
            global.get $m197h4
          end
        end
      end
    else
      global.get $m197h0
    end
    local.set $word
    local.get $word
    i32.const 24
    local.get $index
    i32.const 3
    i32.and
    i32.const 8
    i32.mul
    i32.sub
    i32.shr_u
    i32.const 255
    i32.and)

  (func $emit_b64_3 (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $out_ptr i32)
    local.get $out_ptr
    local.get $b0
    i32.const 2
    i32.shr_u
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 1
    i32.add
    local.get $b0
    i32.const 3
    i32.and
    i32.const 4
    i32.shl
    local.get $b1
    i32.const 4
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 2
    i32.add
    local.get $b1
    i32.const 15
    i32.and
    i32.const 2
    i32.shl
    local.get $b2
    i32.const 6
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    i32.const 3
    i32.add
    local.get $b2
    i32.const 63
    i32.and
    call $b64char
    i32.store8)

  (func (export "ws_accept_key") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $v i32)
    (local $out_i i32)
    local.get $in_len
    i32.const 24
    i32.ne
    if
      (return (call $pack (i32.const 4) (i32.const 0)))
    end
    local.get $in_ptr
    i32.const 22
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    local.get $in_ptr
    i32.const 23
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    i32.or
    if
      (return (call $pack (i32.const 3) (i32.const 0)))
    end
    (loop $validate
      local.get $in_ptr
      local.get $i
      i32.add
      i32.load8_u
      call $b64val
      local.set $v
      local.get $v
      i32.const 0
      i32.lt_s
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 21
      i32.eq
      local.get $v
      i32.const 15
      i32.and
      i32.const 0
      i32.ne
      i32.and
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      i32.const 22
      i32.lt_u
      br_if $validate)
    local.get $out_cap
    i32.const 28
    i32.lt_u
    if
      (return (call $pack (i32.const 2) (i32.const 0)))
    end
    i32.const 0x67452301
    global.set $m197h0
    i32.const 0xefcdab89
    global.set $m197h1
    i32.const 0x98badcfe
    global.set $m197h2
    i32.const 0x10325476
    global.set $m197h3
    i32.const 0xc3d2e1f0
    global.set $m197h4
    (call $sha1_compress
      (i32.const 0)
      (local.get $in_ptr))
    (call $sha1_compress
      (i32.const 1)
      (local.get $in_ptr))
    (local.set $i (i32.const 0))
    (local.set $out_i (i32.const 0))
    (loop $emit_full
      (call $emit_b64_3
        (call $m197digest_byte (local.get $i))
        (call $m197digest_byte (i32.add (local.get $i) (i32.const 1)))
        (call $m197digest_byte (i32.add (local.get $i) (i32.const 2)))
        (i32.add (local.get $out_ptr) (local.get $out_i)))
      local.get $i
      i32.const 3
      i32.add
      local.set $i
      local.get $out_i
      i32.const 4
      i32.add
      local.set $out_i
      local.get $i
      i32.const 18
      i32.lt_u
      br_if $emit_full)
    local.get $out_ptr
    local.get $out_i
    i32.add
    (call $m197digest_byte
      (i32.const 18))
    i32.const 2
    i32.shr_u
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 1
    i32.add
    (call $m197digest_byte
      (i32.const 18))
    i32.const 3
    i32.and
    i32.const 4
    i32.shl
    (call $m197digest_byte
      (i32.const 19))
    i32.const 4
    i32.shr_u
    i32.or
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 2
    i32.add
    (call $m197digest_byte
      (i32.const 19))
    i32.const 15
    i32.and
    i32.const 2
    i32.shl
    call $b64char
    i32.store8
    local.get $out_ptr
    local.get $out_i
    i32.add
    i32.const 3
    i32.add
    i32.const 61
    i32.store8
    (call $pack
      (i32.const 0)
      (i32.const 28)))