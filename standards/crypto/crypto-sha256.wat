(func $m62range_ok (param $ptr i32) (param $len i32) (result i32)
  local.get $ptr local.get $len i32.const 65536 call $range_ok)

  (func $m62w_addr (param $i i32) (result i32)
    i32.const 65536
    local.get $i
    i32.const 2
    i32.shl
    i32.add)

  (data (i32.const 0x10100) "\98\2f\8a\42\91\44\37\71\cf\fb\c0\b5\a5\db\b5\e9\5b\c2\56\39\f1\11\f1\59\a4\82\3f\92\d5\5e\1c\ab\98\aa\07\d8\01\5b\83\12\be\85\31\24\c3\7d\0c\55\74\5d\be\72\fe\b1\de\80\a7\06\dc\9b\74\f1\9b\c1\c1\69\9b\e4\86\47\be\ef\c6\9d\c1\0f\cc\a1\0c\24\6f\2c\e9\2d\aa\84\74\4a\dc\a9\b0\5c\da\88\f9\76\52\51\3e\98\6d\c6\31\a8\c8\27\03\b0\c7\7f\59\bf\f3\0b\e0\c6\47\91\a7\d5\51\63\ca\06\67\29\29\14\85\0a\b7\27\38\21\1b\2e\fc\6d\2c\4d\13\0d\38\53\54\73\0a\65\bb\0a\6a\76\2e\c9\c2\81\85\2c\72\92\a1\e8\bf\a2\4b\66\1a\a8\70\8b\4b\c2\a3\51\6c\c7\19\e8\92\d1\24\06\99\d6\85\35\0e\f4\70\a0\6a\10\16\c1\a4\19\08\6c\37\1e\4c\77\48\27\b5\bc\b0\34\b3\0c\1c\39\4a\aa\d8\4e\4f\ca\9c\5b\f3\6f\2e\68\ee\82\8f\74\6f\63\a5\78\14\78\c8\84\08\02\c7\8c\fa\ff\be\90\eb\6c\50\a4\f7\a3\f9\be\f2\78\71\c6")

  (func $m62k (param $i i32) (result i32)
    i32.load (i32.add (i32.const 0x10100) (i32.shl (local.get $i) (i32.const 2))))

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
    local.get $ptr local.get $offset i32.add local.get $word call $store_be32)

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
