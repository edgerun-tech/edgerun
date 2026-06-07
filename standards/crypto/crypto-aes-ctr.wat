;; AES-128-CTR mode — self-contained AES-128 key expansion + block encrypt + CTR XOR.
  ;; Exports: aes128_ctr_xor(out, in, len, key[16], counter[16]) -> 0
  
  ;; AES S-box (0-255) and Rcon (256-265) — see crypto-aes-sbox-data.wat for shared definition

  ;; Memory layout:
  ;; 0-255: S-box
  ;; 256-265: Rcon
  ;; 16384-16607: round keys (224 bytes — 11 × 16 + spare)
  ;; 16608-16623: counter buffer
  ;; 16624-16639: keystream block
  ;; 16640+: scratch

  ;; ── Helper: load big-endian word ──
  (func $m54load_be32 (param $p i32) (result i32)
    local.get $p i32.load8_u i32.const 24 i32.shl
    local.get $p i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $p i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $p i32.const 3 i32.add i32.load8_u i32.or)

  ;; ── Helper: store big-endian word ──
  (func $m54store_be32 (param $p i32) (param $v i32)
    local.get $p i32.const 0 i32.add local.get $v i32.const 24 i32.shr_u i32.store8
    local.get $p i32.const 1 i32.add local.get $v i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $p i32.const 2 i32.add local.get $v i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $p i32.const 3 i32.add local.get $v i32.const 0xFF i32.and i32.store8)

  ;; ── SubWord: S-box applied to each byte of a little-endian word ──
  (func $ctr_sub_word (param $w i32) (result i32)
    local.get $w i32.const 0xFF i32.and i32.load8_u
    local.get $w i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.load8_u
    i32.const 8 i32.shl i32.or
    local.get $w i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.load8_u
    i32.const 16 i32.shl i32.or
    local.get $w i32.const 24 i32.shr_u i32.load8_u
    i32.const 24 i32.shl i32.or)

  ;; ── RotWord: left-rotate word by one byte ──
  (func $ctr_rot_word (param $w i32) (result i32)
    local.get $w i32.const 8 i32.shl
    local.get $w i32.const 24 i32.shr_u
    i32.or)

  ;; ── aes128_key_expand(key_ptr, rk_ptr) — expand 16-byte key to 176 bytes ──
  (func $m54aes128_key_expand (param $key i32) (param $rk i32)
    (local $i i32) (local $w i32) (local $t i32) (local $rcon_idx i32)

    ;; Copy first 16 bytes (4 words) directly
    local.get $rk i32.const 0 i32.add local.get $key i32.const 0 i32.add call $m54load_be32 call $m54store_be32
    local.get $rk i32.const 4 i32.add local.get $key i32.const 4 i32.add call $m54load_be32 call $m54store_be32
    local.get $rk i32.const 8 i32.add local.get $key i32.const 8 i32.add call $m54load_be32 call $m54store_be32
    local.get $rk i32.const 12 i32.add local.get $key i32.const 12 i32.add call $m54load_be32 call $m54store_be32

    i32.const 4 local.set $i
    i32.const 0 local.set $rcon_idx
    block $done
    loop $loop
      local.get $i i32.const 44 i32.ge_u br_if $done  ;; 44 words = 176 bytes

      ;; Previous word (W[i-1])
      local.get $rk local.get $i i32.const 1 i32.sub i32.const 2 i32.shl i32.add call $m54load_be32
      local.set $w

      local.get $i i32.const 4 i32.rem_s i32.eqz
      if
        ;; t = SubWord(RotWord(w)) ^ Rcon[rcon_idx]
        local.get $w call $ctr_rot_word call $ctr_sub_word
        i32.const 256 local.get $rcon_idx i32.add i32.load8_u i32.const 24 i32.shl
        i32.xor
        local.set $t
        local.get $rcon_idx i32.const 1 i32.add local.set $rcon_idx
      else
        local.get $w local.set $t
      end

      ;; W[i] = W[i-4] ^ t
      local.get $rk local.get $i i32.const 2 i32.shl i32.add
      local.get $rk local.get $i i32.const 4 i32.sub i32.const 2 i32.shl i32.add call $m54load_be32
      local.get $t i32.xor
      call $m54store_be32

      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── SubBytes (in-place on 16-byte block) ──
  (func $m54sub_bytes (param $b i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $b local.get $i i32.add
      local.get $b local.get $i i32.add i32.load8_u
      i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── ShiftRows (in-place on 16-byte block) ──
  ;; Block layout: b[0..15] = [0 4 8 12][1 5 9 13][2 6 10 14][3 7 11 15]
  ;; After ShiftRows: row 0 shifts 0, row 1 shifts 1, row 2 shifts 2, row 3 shifts 3
  (func $m54shift_rows (param $b i32)
    (local $t i32)
    ;; Row 1 (bytes 1,5,9,13): shift left by 1
    local.get $b i32.load offset=1 local.set $t
    local.get $b i32.const 1 i32.add local.get $b i32.load offset=5 i32.store8
    local.get $b i32.const 5 i32.add local.get $b i32.load offset=9 i32.store8
    local.get $b i32.const 9 i32.add local.get $b i32.load offset=13 i32.store8
    local.get $b i32.const 13 i32.add local.get $t i32.store8
    ;; Row 2 (bytes 2,6,10,14): shift left by 2
    local.get $b i32.load offset=2 local.set $t
    local.get $b i32.const 2 i32.add local.get $b i32.load offset=10 i32.store8
    local.get $b i32.const 10 i32.add local.get $t i32.store8
    local.get $b i32.load offset=6 local.set $t
    local.get $b i32.const 6 i32.add local.get $b i32.load offset=14 i32.store8
    local.get $b i32.const 14 i32.add local.get $t i32.store8
    ;; Row 3 (bytes 3,7,11,15): shift left by 3
    local.get $b i32.load offset=3 local.set $t
    local.get $b i32.const 3 i32.add local.get $b i32.load offset=15 i32.store8
    local.get $b i32.const 15 i32.add local.get $b i32.load offset=11 i32.store8
    local.get $b i32.const 11 i32.add local.get $b i32.load offset=7 i32.store8
    local.get $b i32.const 7 i32.add local.get $t i32.store8)

  ;; ── xtime: multiply by 2 in GF(2^8) ──
  (func $m54xtime (param $b i32) (result i32)
    (local $t i32)
    local.get $b i32.const 1 i32.shl local.set $t
    local.get $b i32.const 0x80 i32.and
    if local.get $t i32.const 0x1b i32.xor local.set $t end
    local.get $t i32.const 0xFF i32.and)

  ;; ── MixColumns (in-place on 16-byte block) ──
  (func $m54mix_columns (param $b i32)
    (local $i i32) (local $c0 i32) (local $c1 i32) (local $c2 i32) (local $c3 i32)
    (local $t i32) (local $u i32)

    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $b local.get $i i32.add i32.load8_u local.set $c0
      local.get $b local.get $i i32.const 1 i32.add i32.add i32.load8_u local.set $c1
      local.get $b local.get $i i32.const 2 i32.add i32.add i32.load8_u local.set $c2
      local.get $b local.get $i i32.const 3 i32.add i32.add i32.load8_u local.set $c3

      ;; t = c0 ^ c1 ^ c2 ^ c3
      local.get $c0 local.get $c1 i32.xor local.get $c2 i32.xor local.get $c3 i32.xor local.set $t

      ;; u = c0 ^ c1 ; v = c1 ^ c2 ; w = c2 ^ c3 ; z = c3 ^ c0
      ;; new_c0 = c0 ^ t ^ xtime(c0 ^ c1)
      ;; new_c1 = c1 ^ t ^ xtime(c1 ^ c2)
      ;; new_c2 = c2 ^ t ^ xtime(c2 ^ c3)
      ;; new_c3 = c3 ^ t ^ xtime(c3 ^ c0)

      local.get $b local.get $i i32.add
      local.get $c0 local.get $t i32.xor
      local.get $c0 local.get $c1 i32.xor call $m54xtime i32.xor
      i32.store8

      local.get $b local.get $i i32.const 1 i32.add i32.add
      local.get $c1 local.get $t i32.xor
      local.get $c1 local.get $c2 i32.xor call $m54xtime i32.xor
      i32.store8

      local.get $b local.get $i i32.const 2 i32.add i32.add
      local.get $c2 local.get $t i32.xor
      local.get $c2 local.get $c3 i32.xor call $m54xtime i32.xor
      i32.store8

      local.get $b local.get $i i32.const 3 i32.add i32.add
      local.get $c3 local.get $t i32.xor
      local.get $c3 local.get $c0 i32.xor call $m54xtime i32.xor
      i32.store8

      local.get $i i32.const 4 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── AddRoundKey (in-place XOR) ──
  (func $m54add_round_key (param $b i32) (param $rk i32)
    local.get $b local.get $b i32.load offset=0 local.get $rk i32.load offset=0 i32.xor i32.store offset=0
    local.get $b local.get $b i32.load offset=4 local.get $rk i32.load offset=4 i32.xor i32.store offset=4
    local.get $b local.get $b i32.load offset=8 local.get $rk i32.load offset=8 i32.xor i32.store offset=8
    local.get $b local.get $b i32.load offset=12 local.get $rk i32.load offset=12 i32.xor i32.store offset=12)

  ;; ── aes128_encrypt_block(block_ptr, rk_ptr) — encrypt one block in place ──
  (func $ctr_aes128_encrypt_block (param $b i32) (param $rk i32)
    (local $round i32)

    ;; Initial AddRoundKey with round key 0
    local.get $b local.get $rk call $m54add_round_key

    i32.const 1 local.set $round
    block $done
    loop $loop
      local.get $round i32.const 11 i32.ge_u br_if $done

      local.get $b call $m54sub_bytes
      local.get $b call $m54shift_rows

      ;; MixColumns for rounds 1-9 (skip for round 10)
      local.get $round i32.const 10 i32.ne
      if
        local.get $b call $m54mix_columns
      end

      local.get $b
      local.get $rk local.get $round i32.const 4 i32.shl i32.add
      call $m54add_round_key

      local.get $round i32.const 1 i32.add local.set $round
      br $loop
    end
    end)

  ;; ── aes128_ctr_xor(out, in, len, key[16], counter[16]) -> status ──
  (func $aes128_ctr_xor (export "aes128_ctr_xor")
    (param $out i32) (param $in i32) (param $len i32)
    (param $key i32) (param $ctr i32)
    (result i32)
    (local $rk i32) (local $cbuf i32) (local $ks i32)
    (local $i i32) (local $rem i32) (local $blk i32)
    (local $ctr_lo i32)

    i32.const 16384 local.set $rk
    i32.const 16608 local.set $cbuf
    i32.const 16624 local.set $ks

    local.get $key local.get $rk call $m54aes128_key_expand
    local.get $cbuf local.get $ctr i32.const 16 call $memcpy
    local.get $len local.set $rem

    block $done
    loop $ctr_loop
      local.get $rem i32.eqz br_if $done

      i32.const 16 local.set $blk
      local.get $rem i32.const 16 i32.lt_u
      if local.get $rem local.set $blk end

      local.get $ks local.get $cbuf i32.const 16 call $memcpy
      local.get $ks local.get $rk call $ctr_aes128_encrypt_block

      i32.const 0 local.set $i
      block $xor_done
      loop $xor_loop
        local.get $i local.get $blk i32.ge_u br_if $xor_done
        local.get $out local.get $i i32.add
        local.get $in local.get $i i32.add i32.load8_u
        local.get $ks local.get $i i32.add i32.load8_u
        i32.xor i32.store8
        local.get $i i32.const 1 i32.add local.set $i
        br $xor_loop
      end
      end

      local.get $out local.get $blk i32.add local.set $out
      local.get $in local.get $blk i32.add local.set $in
      local.get $rem local.get $blk i32.sub local.set $rem

      ;; Increment counter (big-endian, low 64 bits = bytes 8-15)
      local.get $cbuf i32.const 12 i32.add
      local.get $cbuf i32.const 12 i32.add call $m54load_be32 i32.const 1 i32.add
      call $m54store_be32
      local.get $cbuf i32.const 12 i32.add call $m54load_be32 i32.eqz
      if
        local.get $cbuf i32.const 8 i32.add
        local.get $cbuf i32.const 8 i32.add call $m54load_be32 i32.const 1 i32.add
        call $m54store_be32
      end

      br $ctr_loop
    end
    end

    i32.const 0)
