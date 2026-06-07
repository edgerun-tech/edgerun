;; AES S-box, Inv S-box, and Rcon data — see aes-sbox-data.wat for shared definition

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
    i32.const 0
    local.get $x
    i32.const 255
    i32.and
    i32.add
    i32.load8_u)

  (func $inv_sbox (param $x i32) (result i32)
    i32.const 266
    local.get $x
    i32.const 255
    i32.and
    i32.add
    i32.load8_u)

  (func $rcon (param $x i32) (result i32)
    i32.const 256
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

  (func $aes128_encrypt (export "aes128_encrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
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

  (func $aes128_decrypt (export "aes128_decrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
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

  (func $aes256_encrypt (export "aes256_encrypt") (param $key i32) (param $key_len i32) (param $block i32) (param $block_len i32) (param $out i32) (result i64)
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
