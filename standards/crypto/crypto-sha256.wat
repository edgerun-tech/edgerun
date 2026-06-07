(func $m62range_ok (param $ptr i32) (param $len i32) (result i32)
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
    i32.const 65536
    i32.le_u)

  (func $m62w_addr (param $i i32) (result i32)
    i32.const 65536
    local.get $i
    i32.const 2
    i32.shl
    i32.add)

  (func $m62k (param $i i32) (result i32)
    local.get $i
    i32.const 0
    i32.eq
    if i32.const 0x428a2f98 return end
    local.get $i
    i32.const 1
    i32.eq
    if i32.const 0x71374491 return end
    local.get $i
    i32.const 2
    i32.eq
    if i32.const 0xb5c0fbcf return end
    local.get $i
    i32.const 3
    i32.eq
    if i32.const 0xe9b5dba5 return end
    local.get $i
    i32.const 4
    i32.eq
    if i32.const 0x3956c25b return end
    local.get $i
    i32.const 5
    i32.eq
    if i32.const 0x59f111f1 return end
    local.get $i
    i32.const 6
    i32.eq
    if i32.const 0x923f82a4 return end
    local.get $i
    i32.const 7
    i32.eq
    if i32.const 0xab1c5ed5 return end
    local.get $i
    i32.const 8
    i32.eq
    if i32.const 0xd807aa98 return end
    local.get $i
    i32.const 9
    i32.eq
    if i32.const 0x12835b01 return end
    local.get $i
    i32.const 10
    i32.eq
    if i32.const 0x243185be return end
    local.get $i
    i32.const 11
    i32.eq
    if i32.const 0x550c7dc3 return end
    local.get $i
    i32.const 12
    i32.eq
    if i32.const 0x72be5d74 return end
    local.get $i
    i32.const 13
    i32.eq
    if i32.const 0x80deb1fe return end
    local.get $i
    i32.const 14
    i32.eq
    if i32.const 0x9bdc06a7 return end
    local.get $i
    i32.const 15
    i32.eq
    if i32.const 0xc19bf174 return end
    local.get $i
    i32.const 16
    i32.eq
    if i32.const 0xe49b69c1 return end
    local.get $i
    i32.const 17
    i32.eq
    if i32.const 0xefbe4786 return end
    local.get $i
    i32.const 18
    i32.eq
    if i32.const 0x0fc19dc6 return end
    local.get $i
    i32.const 19
    i32.eq
    if i32.const 0x240ca1cc return end
    local.get $i
    i32.const 20
    i32.eq
    if i32.const 0x2de92c6f return end
    local.get $i
    i32.const 21
    i32.eq
    if i32.const 0x4a7484aa return end
    local.get $i
    i32.const 22
    i32.eq
    if i32.const 0x5cb0a9dc return end
    local.get $i
    i32.const 23
    i32.eq
    if i32.const 0x76f988da return end
    local.get $i
    i32.const 24
    i32.eq
    if i32.const 0x983e5152 return end
    local.get $i
    i32.const 25
    i32.eq
    if i32.const 0xa831c66d return end
    local.get $i
    i32.const 26
    i32.eq
    if i32.const 0xb00327c8 return end
    local.get $i
    i32.const 27
    i32.eq
    if i32.const 0xbf597fc7 return end
    local.get $i
    i32.const 28
    i32.eq
    if i32.const 0xc6e00bf3 return end
    local.get $i
    i32.const 29
    i32.eq
    if i32.const 0xd5a79147 return end
    local.get $i
    i32.const 30
    i32.eq
    if i32.const 0x06ca6351 return end
    local.get $i
    i32.const 31
    i32.eq
    if i32.const 0x14292967 return end
    local.get $i
    i32.const 32
    i32.eq
    if i32.const 0x27b70a85 return end
    local.get $i
    i32.const 33
    i32.eq
    if i32.const 0x2e1b2138 return end
    local.get $i
    i32.const 34
    i32.eq
    if i32.const 0x4d2c6dfc return end
    local.get $i
    i32.const 35
    i32.eq
    if i32.const 0x53380d13 return end
    local.get $i
    i32.const 36
    i32.eq
    if i32.const 0x650a7354 return end
    local.get $i
    i32.const 37
    i32.eq
    if i32.const 0x766a0abb return end
    local.get $i
    i32.const 38
    i32.eq
    if i32.const 0x81c2c92e return end
    local.get $i
    i32.const 39
    i32.eq
    if i32.const 0x92722c85 return end
    local.get $i
    i32.const 40
    i32.eq
    if i32.const 0xa2bfe8a1 return end
    local.get $i
    i32.const 41
    i32.eq
    if i32.const 0xa81a664b return end
    local.get $i
    i32.const 42
    i32.eq
    if i32.const 0xc24b8b70 return end
    local.get $i
    i32.const 43
    i32.eq
    if i32.const 0xc76c51a3 return end
    local.get $i
    i32.const 44
    i32.eq
    if i32.const 0xd192e819 return end
    local.get $i
    i32.const 45
    i32.eq
    if i32.const 0xd6990624 return end
    local.get $i
    i32.const 46
    i32.eq
    if i32.const 0xf40e3585 return end
    local.get $i
    i32.const 47
    i32.eq
    if i32.const 0x106aa070 return end
    local.get $i
    i32.const 48
    i32.eq
    if i32.const 0x19a4c116 return end
    local.get $i
    i32.const 49
    i32.eq
    if i32.const 0x1e376c08 return end
    local.get $i
    i32.const 50
    i32.eq
    if i32.const 0x2748774c return end
    local.get $i
    i32.const 51
    i32.eq
    if i32.const 0x34b0bcb5 return end
    local.get $i
    i32.const 52
    i32.eq
    if i32.const 0x391c0cb3 return end
    local.get $i
    i32.const 53
    i32.eq
    if i32.const 0x4ed8aa4a return end
    local.get $i
    i32.const 54
    i32.eq
    if i32.const 0x5b9cca4f return end
    local.get $i
    i32.const 55
    i32.eq
    if i32.const 0x682e6ff3 return end
    local.get $i
    i32.const 56
    i32.eq
    if i32.const 0x748f82ee return end
    local.get $i
    i32.const 57
    i32.eq
    if i32.const 0x78a5636f return end
    local.get $i
    i32.const 58
    i32.eq
    if i32.const 0x84c87814 return end
    local.get $i
    i32.const 59
    i32.eq
    if i32.const 0x8cc70208 return end
    local.get $i
    i32.const 60
    i32.eq
    if i32.const 0x90befffa return end
    local.get $i
    i32.const 61
    i32.eq
    if i32.const 0xa4506ceb return end
    local.get $i
    i32.const 62
    i32.eq
    if i32.const 0xbef9a3f7 return end
    i32.const 0xc67178f2)

  (func $m62byte_at (param $ptr i32) (param $len i32) (param $total i32) (param $off i32) (result i32)
    (local $idx i32)
    local.get $off
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $off
      i32.add
      i32.load8_u
      return
    end
    local.get $off
    local.get $len
    i32.eq
    if
      i32.const 128
      return
    end
    local.get $off
    local.get $total
    i32.const 8
    i32.sub
    i32.ge_u
    if
      local.get $off
      local.get $total
      i32.const 8
      i32.sub
      i32.sub
      local.set $idx
      local.get $idx
      i32.const 0
      i32.eq
      if local.get $len i32.const 29 i32.shr_u i32.const 24 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 1
      i32.eq
      if local.get $len i32.const 29 i32.shr_u i32.const 16 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 2
      i32.eq
      if local.get $len i32.const 29 i32.shr_u i32.const 8 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 3
      i32.eq
      if local.get $len i32.const 29 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 4
      i32.eq
      if local.get $len i32.const 3 i32.shl i32.const 24 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 5
      i32.eq
      if local.get $len i32.const 3 i32.shl i32.const 16 i32.shr_u i32.const 255 i32.and return end
      local.get $idx
      i32.const 6
      i32.eq
      if local.get $len i32.const 3 i32.shl i32.const 8 i32.shr_u i32.const 255 i32.and return end
      local.get $len
      i32.const 3
      i32.shl
      i32.const 255
      i32.and
      return
    end
    i32.const 0)

  (func $msg_word (param $ptr i32) (param $len i32) (param $total i32) (param $off i32) (result i32)
    local.get $ptr
    local.get $len
    local.get $total
    local.get $off
    call $m62byte_at
    i32.const 24
    i32.shl
    local.get $ptr
    local.get $len
    local.get $total
    local.get $off
    i32.const 1
    i32.add
    call $m62byte_at
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $total
    local.get $off
    i32.const 2
    i32.add
    call $m62byte_at
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $total
    local.get $off
    i32.const 3
    i32.add
    call $m62byte_at
    i32.or)

  (func $m62rotr (param $x i32) (param $n i32) (result i32)
    local.get $x
    local.get $n
    i32.rotr)

  (func $small0 (param $x i32) (result i32)
    local.get $x
    i32.const 7
    call $m62rotr
    local.get $x
    i32.const 18
    call $m62rotr
    i32.xor
    local.get $x
    i32.const 3
    i32.shr_u
    i32.xor)

  (func $small1 (param $x i32) (result i32)
    local.get $x
    i32.const 17
    call $m62rotr
    local.get $x
    i32.const 19
    call $m62rotr
    i32.xor
    local.get $x
    i32.const 10
    i32.shr_u
    i32.xor)

  (func $big0 (param $x i32) (result i32)
    local.get $x
    i32.const 2
    call $m62rotr
    local.get $x
    i32.const 13
    call $m62rotr
    i32.xor
    local.get $x
    i32.const 22
    call $m62rotr
    i32.xor)

  (func $big1 (param $x i32) (result i32)
    local.get $x
    i32.const 6
    call $m62rotr
    local.get $x
    i32.const 11
    call $m62rotr
    i32.xor
    local.get $x
    i32.const 25
    call $m62rotr
    i32.xor)

  (func $m62ch (param $x i32) (param $y i32) (param $z i32) (result i32)
    local.get $x
    local.get $y
    i32.and
    local.get $x
    i32.const -1
    i32.xor
    local.get $z
    i32.and
    i32.xor)

  (func $m62maj (param $x i32) (param $y i32) (param $z i32) (result i32)
    local.get $x
    local.get $y
    i32.and
    local.get $x
    local.get $z
    i32.and
    i32.xor
    local.get $y
    local.get $z
    i32.and
    i32.xor)

  (func $m62store_be32 (param $ptr i32) (param $offset i32) (param $word i32)
    local.get $ptr
    local.get $offset
    i32.add
    local.get $word
    i32.const 24
    i32.shr_u
    i32.store8
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 1
    i32.add
    local.get $word
    i32.const 16
    i32.shr_u
    i32.store8
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 2
    i32.add
    local.get $word
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 3
    i32.add
    local.get $word
    i32.store8)

  (func $sha256 (export "sha256") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i64)
    (local $blocks i32)
    (local $total i32)
    (local $block i32)
    (local $i i32)
    (local $w i32)
    (local $s0 i32)
    (local $s1 i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    (local $d i32)
    (local $e i32)
    (local $f i32)
    (local $g i32)
    (local $h i32)
    (local $h0 i32)
    (local $h1 i32)
    (local $h2 i32)
    (local $h3 i32)
    (local $h4 i32)
    (local $h5 i32)
    (local $h6 i32)
    (local $h7 i32)
    (local $t1 i32)
    (local $t2 i32)

    local.get $ptr
    local.get $len
    call $m62range_ok
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $out_ptr
    i32.const 32
    call $m62range_ok
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end

    local.get $len
    i32.const 9
    i32.add
    i32.const 63
    i32.add
    i32.const 6
    i32.shr_u
    local.set $blocks
    local.get $blocks
    i32.const 6
    i32.shl
    local.set $total

    i32.const 0x6a09e667
    local.set $h0
    i32.const 0xbb67ae85
    local.set $h1
    i32.const 0x3c6ef372
    local.set $h2
    i32.const 0xa54ff53a
    local.set $h3
    i32.const 0x510e527f
    local.set $h4
    i32.const 0x9b05688c
    local.set $h5
    i32.const 0x1f83d9ab
    local.set $h6
    i32.const 0x5be0cd19
    local.set $h7

    i32.const 0
    local.set $block
    block $all_done
      loop $block_loop
        local.get $block
        local.get $blocks
        i32.ge_u
        br_if $all_done

        i32.const 0
        local.set $i
        block $words_done
          loop $word_loop
            local.get $i
            i32.const 16
            i32.ge_u
            br_if $words_done
            local.get $i
            call $m62w_addr
            local.get $ptr
            local.get $len
            local.get $total
            local.get $block
            i32.const 6
            i32.shl
            local.get $i
            i32.const 2
            i32.shl
            i32.add
            call $msg_word
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $word_loop
          end
        end

        i32.const 16
        local.set $i
        block $schedule_done
          loop $schedule_loop
            local.get $i
            i32.const 64
            i32.ge_u
            br_if $schedule_done
            local.get $i
            i32.const 15
            i32.sub
            call $m62w_addr
            i32.load
            call $small0
            local.set $s0
            local.get $i
            i32.const 2
            i32.sub
            call $m62w_addr
            i32.load
            call $small1
            local.set $s1
            local.get $i
            call $m62w_addr
            local.get $i
            i32.const 16
            i32.sub
            call $m62w_addr
            i32.load
            local.get $s0
            i32.add
            local.get $i
            i32.const 7
            i32.sub
            call $m62w_addr
            i32.load
            i32.add
            local.get $s1
            i32.add
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $schedule_loop
          end
        end

        local.get $h0
        local.set $a
        local.get $h1
        local.set $b
        local.get $h2
        local.set $c
        local.get $h3
        local.set $d
        local.get $h4
        local.set $e
        local.get $h5
        local.set $f
        local.get $h6
        local.set $g
        local.get $h7
        local.set $h

        i32.const 0
        local.set $i
        block $rounds_done
          loop $round_loop
            local.get $i
            i32.const 64
            i32.ge_u
            br_if $rounds_done
            local.get $h
            local.get $e
            call $big1
            i32.add
            local.get $e
            local.get $f
            local.get $g
            call $m62ch
            i32.add
            local.get $i
            call $m62k
            i32.add
            local.get $i
            call $m62w_addr
            i32.load
            i32.add
            local.set $t1
            local.get $a
            call $big0
            local.get $a
            local.get $b
            local.get $c
            call $m62maj
            i32.add
            local.set $t2
            local.get $g
            local.set $h
            local.get $f
            local.set $g
            local.get $e
            local.set $f
            local.get $d
            local.get $t1
            i32.add
            local.set $e
            local.get $c
            local.set $d
            local.get $b
            local.set $c
            local.get $a
            local.set $b
            local.get $t1
            local.get $t2
            i32.add
            local.set $a
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $round_loop
          end
        end

        local.get $h0
        local.get $a
        i32.add
        local.set $h0
        local.get $h1
        local.get $b
        i32.add
        local.set $h1
        local.get $h2
        local.get $c
        i32.add
        local.set $h2
        local.get $h3
        local.get $d
        i32.add
        local.set $h3
        local.get $h4
        local.get $e
        i32.add
        local.set $h4
        local.get $h5
        local.get $f
        i32.add
        local.set $h5
        local.get $h6
        local.get $g
        i32.add
        local.set $h6
        local.get $h7
        local.get $h
        i32.add
        local.set $h7

        local.get $block
        i32.const 1
        i32.add
        local.set $block
        br $block_loop
      end
    end

    local.get $out_ptr
    i32.const 0
    local.get $h0
    call $m62store_be32
    local.get $out_ptr
    i32.const 4
    local.get $h1
    call $m62store_be32
    local.get $out_ptr
    i32.const 8
    local.get $h2
    call $m62store_be32
    local.get $out_ptr
    i32.const 12
    local.get $h3
    call $m62store_be32
    local.get $out_ptr
    i32.const 16
    local.get $h4
    call $m62store_be32
    local.get $out_ptr
    i32.const 20
    local.get $h5
    call $m62store_be32
    local.get $out_ptr
    i32.const 24
    local.get $h6
    call $m62store_be32
    local.get $out_ptr
    i32.const 28
    local.get $h7
    call $m62store_be32

    i32.const 0
    i32.const 32
    call $pack)
