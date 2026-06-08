;; crypto-sha1 — SHA-1 hash (work buffer at 0x10000, 80×4 = 320 bytes)

  (func $m61range_ok (param $ptr i32) (param $len i32) (result i32)
    local.get $ptr local.get $len i32.const 0x100000 call $range_ok)

  (func $m61w_addr (param $i i32) (result i32)
    i32.const 65536
    local.get $i
    i32.const 2
    i32.shl
    i32.add)

  (func $padded_byte
    (param $ptr i32)
    (param $len i32)
    (param $padded_len i32)
    (param $index i32)
    (result i32)
    (local $bit_len i64)
    (local $tail_index i32)
    local.get $index
    local.get $len
    i32.lt_u
    if
      local.get $ptr
      local.get $index
      i32.add
      i32.load8_u
      return
    end
    local.get $index
    local.get $len
    i32.eq
    if
      i32.const 0x80
      return
    end
    local.get $index
    local.get $padded_len
    i32.const 8
    i32.sub
    i32.ge_u
    if
      local.get $len
      i64.extend_i32_u
      i64.const 3
      i64.shl
      local.set $bit_len
      local.get $index
      local.get $padded_len
      i32.const 8
      i32.sub
      i32.sub
      local.set $tail_index
      local.get $bit_len
      i64.const 7
      local.get $tail_index
      i64.extend_i32_u
      i64.sub
      i64.const 8
      i64.mul
      i64.shr_u
      i32.wrap_i64
      i32.const 0xff
      i32.and
      return
    end
    i32.const 0)

  (func $word
    (param $ptr i32)
    (param $len i32)
    (param $padded_len i32)
    (param $index i32)
    (result i32)
    (local $base i32)
    local.get $index
    local.set $base
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    call $padded_byte
    i32.const 24
    i32.shl
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 1
    i32.add
    call $padded_byte
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 2
    i32.add
    call $padded_byte
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    local.get $len
    local.get $padded_len
    local.get $base
    i32.const 3
    i32.add
    call $padded_byte
    i32.or)

  ;; delegates to crypto-core
  (func $m61store_be32 (param $ptr i32) (param $value i32)
    local.get $ptr local.get $value call $store_be32)

  (func $sha1 (export "sha1")
    (param $ptr i32)
    (param $len i32)
    (param $out_ptr i32)
    (result i64)
    (local $padded_len i32)
    (local $block i32)
    (local $i i32)
    (local $a i32)
    (local $b i32)
    (local $c i32)
    (local $d i32)
    (local $e i32)
    (local $f i32)
    (local $k i32)
    (local $temp i32)
    (local $h0 i32)
    (local $h1 i32)
    (local $h2 i32)
    (local $h3 i32)
    (local $h4 i32)

    local.get $out_ptr
    i32.const 20
    call $m61range_ok
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $ptr
    local.get $len
    call $m61range_ok
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end

    local.get $len
    i32.const 9
    i32.add
    i32.const 63
    i32.add
    i32.const -64
    i32.and
    local.set $padded_len

    i32.const 0x67452301
    local.set $h0
    i32.const 0xefcdab89
    local.set $h1
    i32.const 0x98badcfe
    local.set $h2
    i32.const 0x10325476
    local.set $h3
    i32.const 0xc3d2e1f0
    local.set $h4

    i32.const 0
    local.set $block
    block $done_blocks
      loop $blocks
        local.get $block
        local.get $padded_len
        i32.ge_u
        br_if $done_blocks

        i32.const 0
        local.set $i
        block $done_load
          loop $load
            local.get $i
            i32.const 16
            i32.ge_u
            br_if $done_load
            local.get $i
            call $m61w_addr
            local.get $ptr
            local.get $len
            local.get $padded_len
            local.get $block
            local.get $i
            i32.const 2
            i32.shl
            i32.add
            call $word
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $load
          end
        end

        i32.const 16
        local.set $i
        block $done_expand
          loop $expand
            local.get $i
            i32.const 80
            i32.ge_u
            br_if $done_expand
            local.get $i
            call $m61w_addr
            local.get $i
            i32.const 3
            i32.sub
            call $m61w_addr
            i32.load
            local.get $i
            i32.const 8
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            local.get $i
            i32.const 14
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            local.get $i
            i32.const 16
            i32.sub
            call $m61w_addr
            i32.load
            i32.xor
            i32.const 1
            i32.rotl
            i32.store
            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $expand
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

        i32.const 0
        local.set $i
        block $done_rounds
          loop $rounds
            local.get $i
            i32.const 80
            i32.ge_u
            br_if $done_rounds

            local.get $i
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
              local.get $i
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
                local.get $i
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

            local.get $a
            i32.const 5
            i32.rotl
            local.get $f
            i32.add
            local.get $e
            i32.add
            local.get $k
            i32.add
            local.get $i
            call $m61w_addr
            i32.load
            i32.add
            local.set $temp
            local.get $d
            local.set $e
            local.get $c
            local.set $d
            local.get $b
            i32.const 30
            i32.rotl
            local.set $c
            local.get $a
            local.set $b
            local.get $temp
            local.set $a

            local.get $i
            i32.const 1
            i32.add
            local.set $i
            br $rounds
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

        local.get $block
        i32.const 64
        i32.add
        local.set $block
        br $blocks
      end
    end

    local.get $out_ptr
    local.get $h0
    call $m61store_be32
    local.get $out_ptr
    i32.const 4
    i32.add
    local.get $h1
    call $m61store_be32
    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $h2
    call $m61store_be32
    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $h3
    call $m61store_be32
    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $h4
    call $m61store_be32

    i32.const 0
    i32.const 20
    call $pack)
