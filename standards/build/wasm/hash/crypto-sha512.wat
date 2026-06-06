(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(global $m63h0 (mut i64) (i64.const 0))
  (global $m63h1 (mut i64) (i64.const 0))
  (global $m63h2 (mut i64) (i64.const 0))
  (global $m63h3 (mut i64) (i64.const 0))
  (global $m63h4 (mut i64) (i64.const 0))
  (global $h5 (mut i64) (i64.const 0))
  (global $h6 (mut i64) (i64.const 0))
  (global $h7 (mut i64) (i64.const 0))

  (func (export "proto_standard_id") (result i32)
    i32.const 300074)

  (func $sigma0 (param $x i64) (result i64)
    local.get $x
    i64.const 1
    i64.rotr
    local.get $x
    i64.const 8
    i64.rotr
    i64.xor
    local.get $x
    i64.const 7
    i64.shr_u
    i64.xor)

  (func $sigma1 (param $x i64) (result i64)
    local.get $x
    i64.const 19
    i64.rotr
    local.get $x
    i64.const 61
    i64.rotr
    i64.xor
    local.get $x
    i64.const 6
    i64.shr_u
    i64.xor)

  (func $big_sigma0 (param $x i64) (result i64)
    local.get $x
    i64.const 28
    i64.rotr
    local.get $x
    i64.const 34
    i64.rotr
    i64.xor
    local.get $x
    i64.const 39
    i64.rotr
    i64.xor)

  (func $big_sigma1 (param $x i64) (result i64)
    local.get $x
    i64.const 14
    i64.rotr
    local.get $x
    i64.const 18
    i64.rotr
    i64.xor
    local.get $x
    i64.const 41
    i64.rotr
    i64.xor)

  (func $m63ch (param $x i64) (param $y i64) (param $z i64) (result i64)
    local.get $x
    local.get $y
    i64.and
    local.get $x
    i64.const -1
    i64.xor
    local.get $z
    i64.and
    i64.xor)

  (func $m63maj (param $x i64) (param $y i64) (param $z i64) (result i64)
    local.get $x
    local.get $y
    i64.and
    local.get $x
    local.get $z
    i64.and
    i64.xor
    local.get $y
    local.get $z
    i64.and
    i64.xor)

  (func $m63k (param $t i32) (result i64)
    local.get $t
    i32.const 0
    i32.eq
    if
      i64.const 0x428a2f98d728ae22
      return
    end
    local.get $t
    i32.const 1
    i32.eq
    if
      i64.const 0x7137449123ef65cd
      return
    end
    local.get $t
    i32.const 2
    i32.eq
    if
      i64.const 0xb5c0fbcfec4d3b2f
      return
    end
    local.get $t
    i32.const 3
    i32.eq
    if
      i64.const 0xe9b5dba58189dbbc
      return
    end
    local.get $t
    i32.const 4
    i32.eq
    if
      i64.const 0x3956c25bf348b538
      return
    end
    local.get $t
    i32.const 5
    i32.eq
    if
      i64.const 0x59f111f1b605d019
      return
    end
    local.get $t
    i32.const 6
    i32.eq
    if
      i64.const 0x923f82a4af194f9b
      return
    end
    local.get $t
    i32.const 7
    i32.eq
    if
      i64.const 0xab1c5ed5da6d8118
      return
    end
    local.get $t
    i32.const 8
    i32.eq
    if
      i64.const 0xd807aa98a3030242
      return
    end
    local.get $t
    i32.const 9
    i32.eq
    if
      i64.const 0x12835b0145706fbe
      return
    end
    local.get $t
    i32.const 10
    i32.eq
    if
      i64.const 0x243185be4ee4b28c
      return
    end
    local.get $t
    i32.const 11
    i32.eq
    if
      i64.const 0x550c7dc3d5ffb4e2
      return
    end
    local.get $t
    i32.const 12
    i32.eq
    if
      i64.const 0x72be5d74f27b896f
      return
    end
    local.get $t
    i32.const 13
    i32.eq
    if
      i64.const 0x80deb1fe3b1696b1
      return
    end
    local.get $t
    i32.const 14
    i32.eq
    if
      i64.const 0x9bdc06a725c71235
      return
    end
    local.get $t
    i32.const 15
    i32.eq
    if
      i64.const 0xc19bf174cf692694
      return
    end
    local.get $t
    i32.const 16
    i32.eq
    if
      i64.const 0xe49b69c19ef14ad2
      return
    end
    local.get $t
    i32.const 17
    i32.eq
    if
      i64.const 0xefbe4786384f25e3
      return
    end
    local.get $t
    i32.const 18
    i32.eq
    if
      i64.const 0x0fc19dc68b8cd5b5
      return
    end
    local.get $t
    i32.const 19
    i32.eq
    if
      i64.const 0x240ca1cc77ac9c65
      return
    end
    local.get $t
    i32.const 20
    i32.eq
    if
      i64.const 0x2de92c6f592b0275
      return
    end
    local.get $t
    i32.const 21
    i32.eq
    if
      i64.const 0x4a7484aa6ea6e483
      return
    end
    local.get $t
    i32.const 22
    i32.eq
    if
      i64.const 0x5cb0a9dcbd41fbd4
      return
    end
    local.get $t
    i32.const 23
    i32.eq
    if
      i64.const 0x76f988da831153b5
      return
    end
    local.get $t
    i32.const 24
    i32.eq
    if
      i64.const 0x983e5152ee66dfab
      return
    end
    local.get $t
    i32.const 25
    i32.eq
    if
      i64.const 0xa831c66d2db43210
      return
    end
    local.get $t
    i32.const 26
    i32.eq
    if
      i64.const 0xb00327c898fb213f
      return
    end
    local.get $t
    i32.const 27
    i32.eq
    if
      i64.const 0xbf597fc7beef0ee4
      return
    end
    local.get $t
    i32.const 28
    i32.eq
    if
      i64.const 0xc6e00bf33da88fc2
      return
    end
    local.get $t
    i32.const 29
    i32.eq
    if
      i64.const 0xd5a79147930aa725
      return
    end
    local.get $t
    i32.const 30
    i32.eq
    if
      i64.const 0x06ca6351e003826f
      return
    end
    local.get $t
    i32.const 31
    i32.eq
    if
      i64.const 0x142929670a0e6e70
      return
    end
    local.get $t
    i32.const 32
    i32.eq
    if
      i64.const 0x27b70a8546d22ffc
      return
    end
    local.get $t
    i32.const 33
    i32.eq
    if
      i64.const 0x2e1b21385c26c926
      return
    end
    local.get $t
    i32.const 34
    i32.eq
    if
      i64.const 0x4d2c6dfc5ac42aed
      return
    end
    local.get $t
    i32.const 35
    i32.eq
    if
      i64.const 0x53380d139d95b3df
      return
    end
    local.get $t
    i32.const 36
    i32.eq
    if
      i64.const 0x650a73548baf63de
      return
    end
    local.get $t
    i32.const 37
    i32.eq
    if
      i64.const 0x766a0abb3c77b2a8
      return
    end
    local.get $t
    i32.const 38
    i32.eq
    if
      i64.const 0x81c2c92e47edaee6
      return
    end
    local.get $t
    i32.const 39
    i32.eq
    if
      i64.const 0x92722c851482353b
      return
    end
    local.get $t
    i32.const 40
    i32.eq
    if
      i64.const 0xa2bfe8a14cf10364
      return
    end
    local.get $t
    i32.const 41
    i32.eq
    if
      i64.const 0xa81a664bbc423001
      return
    end
    local.get $t
    i32.const 42
    i32.eq
    if
      i64.const 0xc24b8b70d0f89791
      return
    end
    local.get $t
    i32.const 43
    i32.eq
    if
      i64.const 0xc76c51a30654be30
      return
    end
    local.get $t
    i32.const 44
    i32.eq
    if
      i64.const 0xd192e819d6ef5218
      return
    end
    local.get $t
    i32.const 45
    i32.eq
    if
      i64.const 0xd69906245565a910
      return
    end
    local.get $t
    i32.const 46
    i32.eq
    if
      i64.const 0xf40e35855771202a
      return
    end
    local.get $t
    i32.const 47
    i32.eq
    if
      i64.const 0x106aa07032bbd1b8
      return
    end
    local.get $t
    i32.const 48
    i32.eq
    if
      i64.const 0x19a4c116b8d2d0c8
      return
    end
    local.get $t
    i32.const 49
    i32.eq
    if
      i64.const 0x1e376c085141ab53
      return
    end
    local.get $t
    i32.const 50
    i32.eq
    if
      i64.const 0x2748774cdf8eeb99
      return
    end
    local.get $t
    i32.const 51
    i32.eq
    if
      i64.const 0x34b0bcb5e19b48a8
      return
    end
    local.get $t
    i32.const 52
    i32.eq
    if
      i64.const 0x391c0cb3c5c95a63
      return
    end
    local.get $t
    i32.const 53
    i32.eq
    if
      i64.const 0x4ed8aa4ae3418acb
      return
    end
    local.get $t
    i32.const 54
    i32.eq
    if
      i64.const 0x5b9cca4f7763e373
      return
    end
    local.get $t
    i32.const 55
    i32.eq
    if
      i64.const 0x682e6ff3d6b2b8a3
      return
    end
    local.get $t
    i32.const 56
    i32.eq
    if
      i64.const 0x748f82ee5defb2fc
      return
    end
    local.get $t
    i32.const 57
    i32.eq
    if
      i64.const 0x78a5636f43172f60
      return
    end
    local.get $t
    i32.const 58
    i32.eq
    if
      i64.const 0x84c87814a1f0ab72
      return
    end
    local.get $t
    i32.const 59
    i32.eq
    if
      i64.const 0x8cc702081a6439ec
      return
    end
    local.get $t
    i32.const 60
    i32.eq
    if
      i64.const 0x90befffa23631e28
      return
    end
    local.get $t
    i32.const 61
    i32.eq
    if
      i64.const 0xa4506cebde82bde9
      return
    end
    local.get $t
    i32.const 62
    i32.eq
    if
      i64.const 0xbef9a3f7b2c67915
      return
    end
    local.get $t
    i32.const 63
    i32.eq
    if
      i64.const 0xc67178f2e372532b
      return
    end
    local.get $t
    i32.const 64
    i32.eq
    if
      i64.const 0xca273eceea26619c
      return
    end
    local.get $t
    i32.const 65
    i32.eq
    if
      i64.const 0xd186b8c721c0c207
      return
    end
    local.get $t
    i32.const 66
    i32.eq
    if
      i64.const 0xeada7dd6cde0eb1e
      return
    end
    local.get $t
    i32.const 67
    i32.eq
    if
      i64.const 0xf57d4f7fee6ed178
      return
    end
    local.get $t
    i32.const 68
    i32.eq
    if
      i64.const 0x06f067aa72176fba
      return
    end
    local.get $t
    i32.const 69
    i32.eq
    if
      i64.const 0x0a637dc5a2c898a6
      return
    end
    local.get $t
    i32.const 70
    i32.eq
    if
      i64.const 0x113f9804bef90dae
      return
    end
    local.get $t
    i32.const 71
    i32.eq
    if
      i64.const 0x1b710b35131c471b
      return
    end
    local.get $t
    i32.const 72
    i32.eq
    if
      i64.const 0x28db77f523047d84
      return
    end
    local.get $t
    i32.const 73
    i32.eq
    if
      i64.const 0x32caab7b40c72493
      return
    end
    local.get $t
    i32.const 74
    i32.eq
    if
      i64.const 0x3c9ebe0a15c9bebc
      return
    end
    local.get $t
    i32.const 75
    i32.eq
    if
      i64.const 0x431d67c49c100d4c
      return
    end
    local.get $t
    i32.const 76
    i32.eq
    if
      i64.const 0x4cc5d4becb3e42b6
      return
    end
    local.get $t
    i32.const 77
    i32.eq
    if
      i64.const 0x597f299cfc657e2a
      return
    end
    local.get $t
    i32.const 78
    i32.eq
    if
      i64.const 0x5fcb6fab3ad6faec
      return
    end
    local.get $t
    i32.const 79
    i32.eq
    if
      i64.const 0x6c44198c4a475817
      return
    end
    i64.const 0)

  (func $m63message_byte
    (param $ptr i32) (param $len i32) (param $total i32) (param $pos i32)
    (result i32)
    (local $bit_len i64)
    (local $shift i64)
    local.get $pos
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $pos
      i32.add
      i32.load8_u
      return
    end
    local.get $pos
    local.get $len
    i32.eq
    if
      i32.const 128
      return
    end
    local.get $pos
    local.get $total
    i32.const 8
    i32.sub
    i32.ge_u
    if
      local.get $len
      i64.extend_i32_u
      i64.const 3
      i64.shl
      local.set $bit_len
      local.get $total
      i32.const 1
      i32.sub
      local.get $pos
      i32.sub
      i64.extend_i32_u
      i64.const 8
      i64.mul
      local.set $shift
      local.get $bit_len
      local.get $shift
      i64.shr_u
      i32.wrap_i64
      i32.const 255
      i32.and
      return
    end
    i32.const 0)

  (func $m63message_word
    (param $ptr i32) (param $len i32) (param $total i32) (param $block i32) (param $word i32)
    (result i64)
    (local $base i32)
    local.get $block
    local.get $word
    i32.const 3
    i32.shl
    i32.add
    local.set $base
      (i64.or
      (i64.or
      (i64.or
      (i64.or
      (i64.or
      (i64.or
      (i64.or
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 0))))
          (i64.const 56))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 1))))
          (i64.const 48)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 2))))
          (i64.const 40)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 3))))
          (i64.const 32)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 4))))
          (i64.const 24)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 5))))
          (i64.const 16)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 6))))
          (i64.const 8)))
        (i64.shl
          (i64.extend_i32_u
            (call $m63message_byte
              (local.get $ptr)
              (local.get $len)
              (local.get $total)
              (i32.add (local.get $base) (i32.const 7))))
          (i64.const 0))))

  (func $word_at (param $wptr i32) (param $t i32) (result i64)
    local.get $wptr
    local.get $t
    i32.const 3
    i32.shl
    i32.add
    i64.load)

  (func $store_word (param $wptr i32) (param $t i32) (param $value i64)
    local.get $wptr
    local.get $t
    i32.const 3
    i32.shl
    i32.add
    local.get $value
    i64.store)

  (func $m63compress (param $ptr i32) (param $len i32) (param $total i32) (param $block i32)
    (local $wptr i32)
    (local $t i32)
    (local $a i64) (local $b i64) (local $c i64) (local $d i64)
    (local $e i64) (local $f i64) (local $g i64) (local $h i64)
    (local $t1 i64) (local $t2 i64)
    (local.set $wptr (i32.const 56000))
    (local.set $t (i32.const 0))
    (loop $load
      (call $store_word
        (local.get $wptr)
        (local.get $t)
        (call $m63message_word (local.get $ptr) (local.get $len) (local.get $total) (local.get $block) (local.get $t)))
      local.get $t
      i32.const 1
      i32.add
      local.set $t
      local.get $t
      i32.const 16
      i32.lt_u
      br_if $load)
    (loop $expand
      (call $store_word
        (local.get $wptr)
        (local.get $t)
        (i64.add
          (i64.add
            (call $sigma1 (call $word_at (local.get $wptr) (i32.sub (local.get $t) (i32.const 2))))
            (call $word_at (local.get $wptr) (i32.sub (local.get $t) (i32.const 7))))
          (i64.add
            (call $sigma0 (call $word_at (local.get $wptr) (i32.sub (local.get $t) (i32.const 15))))
            (call $word_at (local.get $wptr) (i32.sub (local.get $t) (i32.const 16))))))
      local.get $t
      i32.const 1
      i32.add
      local.set $t
      local.get $t
      i32.const 80
      i32.lt_u
      br_if $expand)
    global.get $m63h0 local.set $a
    global.get $m63h1 local.set $b
    global.get $m63h2 local.set $c
    global.get $m63h3 local.set $d
    global.get $m63h4 local.set $e
    global.get $h5 local.set $f
    global.get $h6 local.set $g
    global.get $h7 local.set $h
    (local.set $t (i32.const 0))
    (loop $round
      (local.set $t1
        (i64.add
          (i64.add
            (i64.add
              (i64.add (local.get $h) (call $big_sigma1 (local.get $e)))
              (call $m63ch (local.get $e) (local.get $f) (local.get $g)))
            (call $m63k (local.get $t)))
          (call $word_at (local.get $wptr) (local.get $t))))
      (local.set $t2
        (i64.add
          (call $big_sigma0 (local.get $a))
          (call $m63maj (local.get $a) (local.get $b) (local.get $c))))
      local.get $g local.set $h
      local.get $f local.set $g
      local.get $e local.set $f
      local.get $d local.get $t1 i64.add local.set $e
      local.get $c local.set $d
      local.get $b local.set $c
      local.get $a local.set $b
      local.get $t1 local.get $t2 i64.add local.set $a
      local.get $t
      i32.const 1
      i32.add
      local.set $t
      local.get $t
      i32.const 80
      i32.lt_u
      br_if $round)
    global.get $m63h0 local.get $a i64.add global.set $m63h0
    global.get $m63h1 local.get $b i64.add global.set $m63h1
    global.get $m63h2 local.get $c i64.add global.set $m63h2
    global.get $m63h3 local.get $d i64.add global.set $m63h3
    global.get $m63h4 local.get $e i64.add global.set $m63h4
    global.get $h5 local.get $f i64.add global.set $h5
    global.get $h6 local.get $g i64.add global.set $h6
    global.get $h7 local.get $h i64.add global.set $h7)

  (func $m63digest_byte (param $index i32) (result i32)
    (local $word i64)
    (local $which i32)
    local.get $index
    i32.const 3
    i32.shr_u
    local.set $which
    global.get $m63h0
    local.set $word
    local.get $which i32.const 1 i32.eq if global.get $m63h1 local.set $word end
    local.get $which i32.const 2 i32.eq if global.get $m63h2 local.set $word end
    local.get $which i32.const 3 i32.eq if global.get $m63h3 local.set $word end
    local.get $which i32.const 4 i32.eq if global.get $m63h4 local.set $word end
    local.get $which i32.const 5 i32.eq if global.get $h5 local.set $word end
    local.get $which i32.const 6 i32.eq if global.get $h6 local.set $word end
    local.get $which i32.const 7 i32.eq if global.get $h7 local.set $word end
    local.get $word
    i64.const 56
    local.get $index
    i32.const 7
    i32.and
    i64.extend_i32_u
    i64.const 8
    i64.mul
    i64.sub
    i64.shr_u
    i32.wrap_i64
    i32.const 255
    i32.and)

  (func $sha512_init
    i64.const 0x6a09e667f3bcc908 global.set $m63h0
    i64.const 0xbb67ae8584caa73b global.set $m63h1
    i64.const 0x3c6ef372fe94f82b global.set $m63h2
    i64.const 0xa54ff53a5f1d36f1 global.set $m63h3
    i64.const 0x510e527fade682d1 global.set $m63h4
    i64.const 0x9b05688c2b3e6c1f global.set $h5
    i64.const 0x1f83d9abfb41bd6b global.set $h6
    i64.const 0x5be0cd19137e2179 global.set $h7)

  (func $sha384_init
    i64.const 0xcbbb9d5dc1059ed8 global.set $m63h0
    i64.const 0x629a292a367cd507 global.set $m63h1
    i64.const 0x9159015a3070dd17 global.set $m63h2
    i64.const 0x152fecd8f70e5939 global.set $m63h3
    i64.const 0x67332667ffc00b31 global.set $m63h4
    i64.const 0x8eb44a8768581511 global.set $h5
    i64.const 0xdb0c2e0d64f98fa7 global.set $h6
    i64.const 0x47b5481dbefa4fa4 global.set $h7)

  (func $m63hash (param $ptr i32) (param $len i32) (param $out_ptr i32) (param $out_len i32) (result i64)
    (local $total i32)
    (local $block i32)
    (local $i i32)
    local.get $len
    i32.const 17
    i32.add
    i32.const 127
    i32.add
    i32.const -128
    i32.and
    local.set $total
    (loop $blocks
      (call $m63compress (local.get $ptr) (local.get $len) (local.get $total) (local.get $block))
      local.get $block
      i32.const 128
      i32.add
      local.set $block
      local.get $block
      local.get $total
      i32.lt_u
      br_if $blocks)
    (loop $emit
      local.get $out_ptr
      local.get $i
      i32.add
      local.get $i
      call $m63digest_byte
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      local.get $out_len
      i32.lt_u
      br_if $emit)
    i32.const 0
    local.get $out_len
    call $pack)

  (func (export "sha512") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i64)
    call $sha512_init
    local.get $ptr
    local.get $len
    local.get $out_ptr
    i32.const 64
    call $m63hash)

  (func (export "sha384") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i64)
    call $sha384_init
    local.get $ptr
    local.get $len
    local.get $out_ptr
    i32.const 48
    call $m63hash)
)
