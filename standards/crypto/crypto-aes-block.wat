(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(data (i32.const 8192) "\63\7c\77\7b\f2\6b\6f\c5\30\01\67\2b\fe\d7\ab\76\ca\82\c9\7d\fa\59\47\f0\ad\d4\a2\af\9c\a4\72\c0\b7\fd\93\26\36\3f\f7\cc\34\a5\e5\f1\71\d8\31\15\04\c7\23\c3\18\96\05\9a\07\12\80\e2\eb\27\b2\75\09\83\2c\1a\1b\6e\5a\a0\52\3b\d6\b3\29\e3\2f\84\53\d1\00\ed\20\fc\b1\5b\6a\cb\be\39\4a\4c\58\cf\d0\ef\aa\fb\43\4d\33\85\45\f9\02\7f\50\3c\9f\a8\51\a3\40\8f\92\9d\38\f5\bc\b6\da\21\10\ff\f3\d2\cd\0c\13\ec\5f\97\44\17\c4\a7\7e\3d\64\5d\19\73\60\81\4f\dc\22\2a\90\88\46\ee\b8\14\de\5e\0b\db\e0\32\3a\0a\49\06\24\5c\c2\d3\ac\62\91\95\e4\79\e7\c8\37\6d\8d\d5\4e\a9\6c\56\f4\ea\65\7a\ae\08\ba\78\25\2e\1c\a6\b4\c6\e8\dd\74\1f\4b\bd\8b\8a\70\3e\b5\66\48\03\f6\0e\61\35\57\b9\86\c1\1d\9e\e1\f8\98\11\69\d9\8e\94\9b\1e\87\e9\ce\55\28\df\8c\a1\89\0d\bf\e6\42\68\41\99\2d\0f\b0\54\bb\16")
  (data (i32.const 8448) "\52\09\6a\d5\30\36\a5\38\bf\40\a3\9e\81\f3\d7\fb\7c\e3\39\82\9b\2f\ff\87\34\8e\43\44\c4\de\e9\cb\54\7b\94\32\a6\c2\23\3d\ee\4c\95\0b\42\fa\c3\4e\08\2e\a1\66\28\d9\24\b2\76\5b\a2\49\6d\8b\d1\25\72\f8\f6\64\86\68\98\16\d4\a4\5c\cc\5d\65\b6\92\6c\70\48\50\fd\ed\b9\da\5e\15\46\57\a7\8d\9d\84\90\d8\ab\00\8c\bc\d3\0a\f7\e4\58\05\b8\b3\45\06\d0\2c\1e\8f\ca\3f\0f\02\c1\af\bd\03\01\13\8a\6b\3a\91\11\41\4f\67\dc\ea\97\f2\cf\ce\f0\b4\e6\73\96\ac\74\22\e7\ad\35\85\e2\f9\37\e8\1c\75\df\6e\47\f1\1a\71\1d\29\c5\89\6f\b7\62\0e\aa\18\be\1b\fc\56\3e\4b\c6\d2\79\20\9a\db\c0\fe\78\cd\5a\f4\1f\dd\a8\33\88\07\c7\31\b1\12\10\59\27\80\ec\5f\60\51\7f\a9\19\b5\4a\0d\2d\e5\7a\9f\93\c9\9c\ef\a0\e0\3b\4d\ae\2a\f5\b0\c8\eb\bb\3c\83\53\99\61\17\2b\04\7e\ba\77\d6\26\e1\69\14\63\55\21\0c\7d")
  (data (i32.const 8704) "\00\01\02\04\08\10\20\40\80\1b\36")

  (func (export "proto_standard_id") (result i32)
    i32.const 300083)

  (func $m53range_ok (param $ptr i32) (param $len i32) (result i32)
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

  (func $valid (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (param $want_key i32) (result i32)
    local.get $key_len
    local.get $want_key
    i32.ne
    if i32.const 0 return end
    local.get $block_len
    i32.const 16
    i32.ne
    if i32.const 0 return end
    local.get $key
    local.get $key_len
    call $m53range_ok
    i32.eqz
    if i32.const 0 return end
    local.get $block
    i32.const 16
    call $m53range_ok
    i32.eqz
    if i32.const 0 return end
    local.get $out
    i32.const 16
    call $m53range_ok)

  (func $sbox (param $x i32) (result i32)
    i32.const 8192
    local.get $x
    i32.const 255
    i32.and
    i32.add
    i32.load8_u)

  (func $inv_sbox (param $x i32) (result i32)
    i32.const 8448
    local.get $x
    i32.const 255
    i32.and
    i32.add
    i32.load8_u)

  (func $rcon (param $x i32) (result i32)
    i32.const 8704
    local.get $x
    i32.add
    i32.load8_u)

  (func $m53xtime (param $x i32) (result i32)
    local.get $x
    i32.const 128
    i32.and
    if (result i32)
      local.get $x
      i32.const 1
      i32.shl
      i32.const 0x11b
      i32.xor
    else
      local.get $x
      i32.const 1
      i32.shl
    end
    i32.const 255
    i32.and)

  (func $gf_mul (param $x i32) (param $y i32) (result i32)
    (local $out i32)
    block $done
      loop $again
        local.get $y
        i32.eqz
        br_if $done
        local.get $y
        i32.const 1
        i32.and
        if
          local.get $out
          local.get $x
          i32.xor
          local.set $out
        end
        local.get $x
        call $m53xtime
        local.set $x
        local.get $y
        i32.const 1
        i32.shr_u
        local.set $y
        br $again
      end
    end
    local.get $out
    i32.const 255
    i32.and)

  (func $expand_key (param $key i32) (param $nk i32) (param $nr i32) (param $rk i32)
    (local $i i32) (local $idx i32) (local $j i32) (local $k i32)
    (local $t0 i32) (local $t1 i32) (local $t2 i32) (local $t3 i32)
    i32.const 0
    local.set $idx
    block $copy_done
      loop $copy
        local.get $idx
        local.get $nk
        i32.const 2
        i32.shl
        i32.ge_u
        br_if $copy_done
        local.get $rk
        local.get $idx
        i32.add
        local.get $key
        local.get $idx
        i32.add
        i32.load8_u
        i32.store8
        local.get $idx
        i32.const 1
        i32.add
        local.set $idx
        br $copy
      end
    end
    local.get $nk
    local.set $i
    block $done
      loop $words
        local.get $i
        i32.const 4
        local.get $nr
        i32.const 1
        i32.add
        i32.mul
        i32.ge_u
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        i32.const 2
        i32.shl
        local.set $j
        local.get $rk
        local.get $j
        i32.add
        i32.load8_u
        local.set $t0
        local.get $rk
        local.get $j
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        local.set $t1
        local.get $rk
        local.get $j
        i32.const 2
        i32.add
        i32.add
        i32.load8_u
        local.set $t2
        local.get $rk
        local.get $j
        i32.const 3
        i32.add
        i32.add
        i32.load8_u
        local.set $t3
        local.get $i
        local.get $nk
        i32.rem_u
        i32.eqz
        if
          local.get $t1
          call $sbox
          local.get $i
          local.get $nk
          i32.div_u
          call $rcon
          i32.xor
          local.set $t0
          local.get $t2
          call $sbox
          local.set $t1
          local.get $t3
          call $sbox
          local.set $t2
          local.get $rk
          local.get $j
          i32.add
          i32.load8_u
          call $sbox
          local.set $t3
        else
          local.get $nk
          i32.const 6
          i32.gt_u
          local.get $i
          local.get $nk
          i32.rem_u
          i32.const 4
          i32.eq
          i32.and
          if
            local.get $t0
            call $sbox
            local.set $t0
            local.get $t1
            call $sbox
            local.set $t1
            local.get $t2
            call $sbox
            local.set $t2
            local.get $t3
            call $sbox
            local.set $t3
          end
        end
        local.get $i
        local.get $nk
        i32.sub
        i32.const 2
        i32.shl
        local.set $j
        local.get $i
        i32.const 2
        i32.shl
        local.set $k
        local.get $rk
        local.get $k
        i32.add
        local.get $rk
        local.get $j
        i32.add
        i32.load8_u
        local.get $t0
        i32.xor
        i32.store8
        local.get $rk
        local.get $k
        i32.const 1
        i32.add
        i32.add
        local.get $rk
        local.get $j
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        local.get $t1
        i32.xor
        i32.store8
        local.get $rk
        local.get $k
        i32.const 2
        i32.add
        i32.add
        local.get $rk
        local.get $j
        i32.const 2
        i32.add
        i32.add
        i32.load8_u
        local.get $t2
        i32.xor
        i32.store8
        local.get $rk
        local.get $k
        i32.const 3
        i32.add
        i32.add
        local.get $rk
        local.get $j
        i32.const 3
        i32.add
        i32.add
        i32.load8_u
        local.get $t3
        i32.xor
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $words
      end
    end)

  (func $copy_state (param $src i32) (param $state i32)
    (local $i i32)
    block $done
      loop $again
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $state
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

  (func $write_state (param $state i32) (param $out i32)
    (local $i i32)
    block $done
      loop $again
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $out
        local.get $i
        i32.add
        local.get $state
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

  (func $m53add_round_key (param $state i32) (param $round_key i32)
    (local $i i32)
    block $done
      loop $again
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $state
        local.get $i
        i32.add
        local.get $state
        local.get $i
        i32.add
        i32.load8_u
        local.get $round_key
        local.get $i
        i32.add
        i32.load8_u
        i32.xor
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $m53sub_bytes (param $state i32)
    (local $i i32)
    block $done
      loop $again
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $state
        local.get $i
        i32.add
        local.get $state
        local.get $i
        i32.add
        i32.load8_u
        call $sbox
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $inv_sub_bytes (param $state i32)
    (local $i i32)
    block $done
      loop $again
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $state
        local.get $i
        i32.add
        local.get $state
        local.get $i
        i32.add
        i32.load8_u
        call $inv_sbox
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $again
      end
    end)

  (func $m53shift_rows (param $s i32)
    (local $t i32)
    local.get $s
    i32.const 1
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 1
    i32.add
    local.get $s
    i32.const 5
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 5
    i32.add
    local.get $s
    i32.const 9
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 9
    i32.add
    local.get $s
    i32.const 13
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 13
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 2
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 2
    i32.add
    local.get $s
    i32.const 10
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 10
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 6
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 6
    i32.add
    local.get $s
    i32.const 14
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 14
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 15
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 15
    i32.add
    local.get $s
    i32.const 11
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 11
    i32.add
    local.get $s
    i32.const 7
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 7
    i32.add
    local.get $s
    i32.const 3
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 3
    i32.add
    local.get $t
    i32.store8)

  (func $inv_shift_rows (param $s i32)
    (local $t i32)
    local.get $s
    i32.const 13
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 13
    i32.add
    local.get $s
    i32.const 9
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 9
    i32.add
    local.get $s
    i32.const 5
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 5
    i32.add
    local.get $s
    i32.const 1
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 1
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 2
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 2
    i32.add
    local.get $s
    i32.const 10
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 10
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 6
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 6
    i32.add
    local.get $s
    i32.const 14
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 14
    i32.add
    local.get $t
    i32.store8
    local.get $s
    i32.const 3
    i32.add
    i32.load8_u
    local.set $t
    local.get $s
    i32.const 3
    i32.add
    local.get $s
    i32.const 7
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 7
    i32.add
    local.get $s
    i32.const 11
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 11
    i32.add
    local.get $s
    i32.const 15
    i32.add
    i32.load8_u
    i32.store8
    local.get $s
    i32.const 15
    i32.add
    local.get $t
    i32.store8)

  (func $m53mix_columns (param $s i32)
    (local $i i32) (local $a i32) (local $b i32) (local $c i32) (local $d i32) (local $h i32)
    block $done
      loop $cols
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $s
        local.get $i
        i32.add
        i32.load8_u
        local.set $a
        local.get $s
        local.get $i
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        local.set $b
        local.get $s
        local.get $i
        i32.const 2
        i32.add
        i32.add
        i32.load8_u
        local.set $c
        local.get $s
        local.get $i
        i32.const 3
        i32.add
        i32.add
        i32.load8_u
        local.set $d
        local.get $a
        local.get $b
        i32.xor
        local.get $c
        i32.xor
        local.get $d
        i32.xor
        local.set $h
        local.get $s
        local.get $i
        i32.add
        local.get $a
        local.get $h
        i32.xor
        local.get $a
        local.get $b
        i32.xor
        call $m53xtime
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 1
        i32.add
        i32.add
        local.get $b
        local.get $h
        i32.xor
        local.get $b
        local.get $c
        i32.xor
        call $m53xtime
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 2
        i32.add
        i32.add
        local.get $c
        local.get $h
        i32.xor
        local.get $c
        local.get $d
        i32.xor
        call $m53xtime
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 3
        i32.add
        i32.add
        local.get $d
        local.get $h
        i32.xor
        local.get $d
        local.get $a
        i32.xor
        call $m53xtime
        i32.xor
        i32.store8
        local.get $i
        i32.const 4
        i32.add
        local.set $i
        br $cols
      end
    end)

  (func $inv_mix_columns (param $s i32)
    (local $i i32) (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    block $done
      loop $cols
        local.get $i
        i32.const 16
        i32.ge_u
        br_if $done
        local.get $s
        local.get $i
        i32.add
        i32.load8_u
        local.set $a
        local.get $s
        local.get $i
        i32.const 1
        i32.add
        i32.add
        i32.load8_u
        local.set $b
        local.get $s
        local.get $i
        i32.const 2
        i32.add
        i32.add
        i32.load8_u
        local.set $c
        local.get $s
        local.get $i
        i32.const 3
        i32.add
        i32.add
        i32.load8_u
        local.set $d
        local.get $s
        local.get $i
        i32.add
        local.get $a
        i32.const 14
        call $gf_mul
        local.get $b
        i32.const 11
        call $gf_mul
        i32.xor
        local.get $c
        i32.const 13
        call $gf_mul
        i32.xor
        local.get $d
        i32.const 9
        call $gf_mul
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 1
        i32.add
        i32.add
        local.get $a
        i32.const 9
        call $gf_mul
        local.get $b
        i32.const 14
        call $gf_mul
        i32.xor
        local.get $c
        i32.const 11
        call $gf_mul
        i32.xor
        local.get $d
        i32.const 13
        call $gf_mul
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 2
        i32.add
        i32.add
        local.get $a
        i32.const 13
        call $gf_mul
        local.get $b
        i32.const 9
        call $gf_mul
        i32.xor
        local.get $c
        i32.const 14
        call $gf_mul
        i32.xor
        local.get $d
        i32.const 11
        call $gf_mul
        i32.xor
        i32.store8
        local.get $s
        local.get $i
        i32.const 3
        i32.add
        i32.add
        local.get $a
        i32.const 11
        call $gf_mul
        local.get $b
        i32.const 13
        call $gf_mul
        i32.xor
        local.get $c
        i32.const 9
        call $gf_mul
        i32.xor
        local.get $d
        i32.const 14
        call $gf_mul
        i32.xor
        i32.store8
        local.get $i
        i32.const 4
        i32.add
        local.set $i
        br $cols
      end
    end)

  (func $encrypt (param $key i32) (param $block i32) (param $out i32) (param $nk i32) (param $nr i32) (result i64)
    (local $round i32)
    local.get $key
    local.get $nk
    local.get $nr
    i32.const 4096
    call $expand_key
    local.get $block
    i32.const 5000
    call $copy_state
    i32.const 5000
    i32.const 4096
    call $m53add_round_key
    i32.const 1
    local.set $round
    block $done
      loop $again
        local.get $round
        local.get $nr
        i32.ge_u
        br_if $done
        i32.const 5000
        call $m53sub_bytes
        i32.const 5000
        call $m53shift_rows
        i32.const 5000
        call $m53mix_columns
        i32.const 5000
        i32.const 4096
        local.get $round
        i32.const 4
        i32.shl
        i32.add
        call $m53add_round_key
        local.get $round
        i32.const 1
        i32.add
        local.set $round
        br $again
      end
    end
    i32.const 5000
    call $m53sub_bytes
    i32.const 5000
    call $m53shift_rows
    i32.const 5000
    i32.const 4096
    local.get $nr
    i32.const 4
    i32.shl
    i32.add
    call $m53add_round_key
    i32.const 5000
    local.get $out
    call $write_state
    i32.const 0
    i32.const 16
    call $pack)

  (func $decrypt128 (param $key i32) (param $block i32) (param $out i32) (result i64)
    (local $round i32)
    local.get $key
    i32.const 4
    i32.const 10
    i32.const 4096
    call $expand_key
    local.get $block
    i32.const 5000
    call $copy_state
    i32.const 5000
    i32.const 4096
    i32.const 160
    i32.add
    call $m53add_round_key
    i32.const 5000
    call $inv_shift_rows
    i32.const 5000
    call $inv_sub_bytes
    i32.const 9
    local.set $round
    block $done
      loop $again
        local.get $round
        i32.eqz
        br_if $done
        i32.const 5000
        i32.const 4096
        local.get $round
        i32.const 4
        i32.shl
        i32.add
        call $m53add_round_key
        i32.const 5000
        call $inv_mix_columns
        i32.const 5000
        call $inv_shift_rows
        i32.const 5000
        call $inv_sub_bytes
        local.get $round
        i32.const 1
        i32.sub
        local.set $round
        br $again
      end
    end
    i32.const 5000
    i32.const 4096
    call $m53add_round_key
    i32.const 5000
    local.get $out
    call $write_state
    i32.const 0
    i32.const 16
    call $pack)

  (func (export "aes128_encrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
    local.get $key
    local.get $key_len
    local.get $block
    local.get $block_len
    local.get $out
    i32.const 16
    call $valid
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $key
    local.get $block
    local.get $out
    i32.const 4
    i32.const 10
    call $encrypt)

  (func (export "aes128_decrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
    local.get $key
    local.get $key_len
    local.get $block
    local.get $block_len
    local.get $out
    i32.const 16
    call $valid
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $key
    local.get $block
    local.get $out
    call $decrypt128)

  (func (export "aes256_encrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
    local.get $key
    local.get $key_len
    local.get $block
    local.get $block_len
    local.get $out
    i32.const 32
    call $valid
    i32.eqz
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $key
    local.get $block
    local.get $out
    i32.const 8
    i32.const 14
    call $encrypt)
)
