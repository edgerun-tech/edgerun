;; AES-128-GCM AEAD — self-contained AES-128 key expansion + block encrypt +
  ;; GHASH (GF(2^128) software multiplication) + GCM encrypt/decrypt.
  ;; Exports:
  ;;   aes128_gcm_encrypt(out, in, len, aad, aad_len, key, iv12, tag16) -> i32 (0=ok, -1=error)
  ;;   aes128_gcm_decrypt(out, in, len, aad, aad_len, key, iv12, tag16) -> i32 (0=ok, -1=error);; ── S-box (0-255) ──
  
  
  (data (i32.const 0) "\63\7c\77\7b\f2\6b\6f\c5\30\01\67\2b\fe\d7\ab\76")
  (data (i32.const 16) "\ca\82\c9\7d\fa\59\47\f0\ad\d4\a2\af\9c\a4\72\c0")
  (data (i32.const 32) "\b7\fd\93\26\36\3f\f7\cc\34\a5\e5\f1\71\d8\31\15")
  (data (i32.const 48) "\04\c7\23\c3\18\96\05\9a\07\12\80\e2\eb\27\b2\75")
  (data (i32.const 64) "\09\83\2c\1a\1b\6e\5a\a0\52\3b\d6\b3\29\e3\2f\84")
  (data (i32.const 80) "\53\d1\00\ed\20\fc\b1\5b\6a\cb\be\39\4a\4c\58\cf")
  (data (i32.const 96) "\d0\ef\aa\fb\43\4d\33\85\45\f9\02\7f\50\3c\9f\a8")
  (data (i32.const 112) "\51\a3\40\8f\92\9d\38\f5\bc\b6\da\21\10\ff\f3\d2")
  (data (i32.const 128) "\cd\0c\13\ec\5f\97\44\17\c4\a7\7e\3d\64\5d\19\73")
  (data (i32.const 144) "\60\81\4f\dc\22\2a\90\88\46\ee\b8\14\de\5e\0b\db")
  (data (i32.const 160) "\e0\32\3a\0a\49\06\24\5c\c2\d3\ac\62\91\95\e4\79")
  (data (i32.const 176) "\e7\c8\37\6d\8d\d5\4e\a9\6c\56\f4\ea\65\7a\ae\08")
  (data (i32.const 192) "\ba\78\25\2e\1c\a6\b4\c6\e8\dd\74\1f\4b\bd\8b\8a")
  (data (i32.const 208) "\70\3e\b5\66\48\03\f6\0e\61\35\57\b9\86\c1\1d\9e")
  (data (i32.const 224) "\e1\f8\98\11\69\d9\8e\94\9b\1e\87\e9\ce\55\28\df")
  (data (i32.const 240) "\8c\a1\89\0d\bf\e6\42\68\41\99\2d\0f\b0\54\bb\16")

  ;; ── Rcon (256-265) ──
  (data (i32.const 256) "\01\02\04\08\10\20\40\80\1b\36")

  ;; ── Exports ──
  ;; ── Memory layout ──
  ;; 0-255:    S-box
  ;; 256-265:  Rcon
  ;; 16384-16607: round keys (224 bytes — 11 × 16 + spare)
  ;; 16608-16623: counter buffer
  ;; 16624-16639: keystream block
  ;; 16640-16655: H (GHASH subkey)
  ;; 16656-16671: X (GHASH accumulator)
  ;; 16672-16687: Z (GHASH mul accumulator)
  ;; 16688-16703: V (GHASH mul shift register)
  ;; 16704-16719: J0 / tagmask encrypt input
  ;; 16720-16735: tagmask (AES_K(J0))
  ;; 16736-16751: len_block (16 bytes)
  ;; 16752-16767: tmp / partial block
  ;; 16768+:      spare

  (global $RK i32 (i32.const 16384))
  (global $CTR i32 (i32.const 16608))
  (global $KS i32 (i32.const 16624))
  (global $H i32 (i32.const 16640))
  (global $X i32 (i32.const 16656))
  (global $Z i32 (i32.const 16672))
  (global $V i32 (i32.const 16688))
  (global $J0 i32 (i32.const 16704))
  (global $TAGMASK i32 (i32.const 16720))
  (global $LEN_BLOCK i32 (i32.const 16736))
  (global $TMP i32 (i32.const 16752))

  ;; ── Helper: load big-endian 32-bit word ──
  (func $load_be32 (param $p i32) (result i32)
    local.get $p i32.load8_u i32.const 24 i32.shl
    local.get $p i32.const 1 i32.add i32.load8_u i32.const 16 i32.shl i32.or
    local.get $p i32.const 2 i32.add i32.load8_u i32.const 8 i32.shl i32.or
    local.get $p i32.const 3 i32.add i32.load8_u i32.or)

  ;; ── Helper: store big-endian 32-bit word ──
  (func $store_be32 (param $p i32) (param $v i32)
    local.get $p i32.const 0 i32.add local.get $v i32.const 24 i32.shr_u i32.store8
    local.get $p i32.const 1 i32.add local.get $v i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $p i32.const 2 i32.add local.get $v i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $p i32.const 3 i32.add local.get $v i32.const 0xFF i32.and i32.store8)

  ;; ── Helper: store big-endian 64-bit value ──
  (func $store_be64 (param $p i32) (param $v i64)
    local.get $p i32.const 0 i32.add local.get $v i64.const 56 i64.shr_u i64.store8
    local.get $p i32.const 1 i32.add local.get $v i64.const 48 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 2 i32.add local.get $v i64.const 40 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 3 i32.add local.get $v i64.const 32 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 4 i32.add local.get $v i64.const 24 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 5 i32.add local.get $v i64.const 16 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 6 i32.add local.get $v i64.const 8 i64.shr_u i64.const 0xFF i64.and i64.store8
    local.get $p i32.const 7 i32.add local.get $v i64.const 0xFF i64.and i64.store8)

  ;; ── SubWord: S-box applied to each byte of a little-endian word ──
  (func $sub_word (param $w i32) (result i32)
    local.get $w i32.const 0xFF i32.and i32.load8_u
    local.get $w i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.load8_u i32.const 8 i32.shl i32.or
    local.get $w i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.load8_u i32.const 16 i32.shl i32.or
    local.get $w i32.const 24 i32.shr_u i32.load8_u i32.const 24 i32.shl i32.or)

  ;; ── RotWord: left-rotate word by one byte ──
  (func $rot_word (param $w i32) (result i32)
    local.get $w i32.const 8 i32.shl
    local.get $w i32.const 24 i32.shr_u i32.or)

  ;; ── aes128_key_expand(key_ptr, rk_ptr) ──
  (func $aes128_key_expand (param $key i32) (param $rk i32)
    (local $i i32) (local $w i32) (local $t i32) (local $rcon_idx i32)
    local.get $rk i32.const 0 i32.add local.get $key i32.const 0 i32.add call $load_be32 call $store_be32
    local.get $rk i32.const 4 i32.add local.get $key i32.const 4 i32.add call $load_be32 call $store_be32
    local.get $rk i32.const 8 i32.add local.get $key i32.const 8 i32.add call $load_be32 call $store_be32
    local.get $rk i32.const 12 i32.add local.get $key i32.const 12 i32.add call $load_be32 call $store_be32
    i32.const 4 local.set $i
    i32.const 0 local.set $rcon_idx
    block $done
    loop $loop
      local.get $i i32.const 44 i32.ge_u br_if $done
      local.get $rk local.get $i i32.const 1 i32.sub i32.const 2 i32.shl i32.add call $load_be32 local.set $w
      local.get $i i32.const 4 i32.rem_s i32.eqz
      if
        local.get $w call $rot_word call $sub_word
        i32.const 256 local.get $rcon_idx i32.add i32.load8_u i32.const 24 i32.shl i32.xor local.set $t
        local.get $rcon_idx i32.const 1 i32.add local.set $rcon_idx
      else
        local.get $w local.set $t
      end
      local.get $rk local.get $i i32.const 2 i32.shl i32.add
      local.get $rk local.get $i i32.const 4 i32.sub i32.const 2 i32.shl i32.add call $load_be32
      local.get $t i32.xor call $store_be32
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── SubBytes (in-place on 16-byte block) ──
  (func $sub_bytes (param $b i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $b local.get $i i32.add
      local.get $b local.get $i i32.add i32.load8_u i32.load8_u i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── ShiftRows (in-place on 16-byte block) ──
  (func $shift_rows (param $b i32)
    (local $t i32)
    local.get $b i32.load offset=1 local.set $t
    local.get $b i32.const 1 i32.add local.get $b i32.load offset=5 i32.store8
    local.get $b i32.const 5 i32.add local.get $b i32.load offset=9 i32.store8
    local.get $b i32.const 9 i32.add local.get $b i32.load offset=13 i32.store8
    local.get $b i32.const 13 i32.add local.get $t i32.store8
    local.get $b i32.load offset=2 local.set $t
    local.get $b i32.const 2 i32.add local.get $b i32.load offset=10 i32.store8
    local.get $b i32.const 10 i32.add local.get $t i32.store8
    local.get $b i32.load offset=6 local.set $t
    local.get $b i32.const 6 i32.add local.get $b i32.load offset=14 i32.store8
    local.get $b i32.const 14 i32.add local.get $t i32.store8
    local.get $b i32.load offset=3 local.set $t
    local.get $b i32.const 3 i32.add local.get $b i32.load offset=15 i32.store8
    local.get $b i32.const 15 i32.add local.get $b i32.load offset=11 i32.store8
    local.get $b i32.const 11 i32.add local.get $b i32.load offset=7 i32.store8
    local.get $b i32.const 7 i32.add local.get $t i32.store8)

  ;; ── xtime: multiply by 2 in GF(2^8) ──
  (func $xtime (param $b i32) (result i32)
    (local $t i32)
    local.get $b i32.const 1 i32.shl local.set $t
    local.get $b i32.const 0x80 i32.and
    if local.get $t i32.const 0x1b i32.xor local.set $t end
    local.get $t i32.const 0xFF i32.and)

  ;; ── MixColumns (in-place on 16-byte block) ──
  (func $mix_columns (param $b i32)
    (local $i i32) (local $c0 i32) (local $c1 i32) (local $c2 i32) (local $c3 i32) (local $t i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $b local.get $i i32.add i32.load8_u local.set $c0
      local.get $b local.get $i i32.const 1 i32.add i32.add i32.load8_u local.set $c1
      local.get $b local.get $i i32.const 2 i32.add i32.add i32.load8_u local.set $c2
      local.get $b local.get $i i32.const 3 i32.add i32.add i32.load8_u local.set $c3
      local.get $c0 local.get $c1 i32.xor local.get $c2 i32.xor local.get $c3 i32.xor local.set $t
      local.get $b local.get $i i32.add
      local.get $c0 local.get $t i32.xor local.get $c0 local.get $c1 i32.xor call $xtime i32.xor i32.store8
      local.get $b local.get $i i32.const 1 i32.add i32.add
      local.get $c1 local.get $t i32.xor local.get $c1 local.get $c2 i32.xor call $xtime i32.xor i32.store8
      local.get $b local.get $i i32.const 2 i32.add i32.add
      local.get $c2 local.get $t i32.xor local.get $c2 local.get $c3 i32.xor call $xtime i32.xor i32.store8
      local.get $b local.get $i i32.const 3 i32.add i32.add
      local.get $c3 local.get $t i32.xor local.get $c3 local.get $c0 i32.xor call $xtime i32.xor i32.store8
      local.get $i i32.const 4 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── AddRoundKey (in-place XOR) ──
  (func $add_round_key (param $b i32) (param $rk i32)
    local.get $b local.get $b i32.load offset=0 local.get $rk i32.load offset=0 i32.xor i32.store offset=0
    local.get $b local.get $b i32.load offset=4 local.get $rk i32.load offset=4 i32.xor i32.store offset=4
    local.get $b local.get $b i32.load offset=8 local.get $rk i32.load offset=8 i32.xor i32.store offset=8
    local.get $b local.get $b i32.load offset=12 local.get $rk i32.load offset=12 i32.xor i32.store offset=12)

  ;; ── aes128_encrypt_block(block_ptr, rk_ptr) — encrypt one block in place ──
  (func $aes128_encrypt_block (param $b i32) (param $rk i32)
    (local $round i32)
    local.get $b local.get $rk call $add_round_key
    i32.const 1 local.set $round
    block $done
    loop $loop
      local.get $round i32.const 11 i32.ge_u br_if $done
      local.get $b call $sub_bytes
      local.get $b call $shift_rows
      local.get $round i32.const 10 i32.ne
      if local.get $b call $mix_columns end
      local.get $b local.get $rk local.get $round i32.const 4 i32.shl i32.add call $add_round_key
      local.get $round i32.const 1 i32.add local.set $round
      br $loop
    end
    end)

  ;; ── xor16(dst, src) — XOR 16 bytes from src into dst ──
  (func $xor16 (param $dst i32) (param $src i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      local.get $dst local.get $i i32.add
      local.get $dst local.get $i i32.add i32.load8_u
      local.get $src local.get $i i32.add i32.load8_u
      i32.xor i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── crypto_xor8(dst, src, len) — XOR len bytes from src into dst ──
  (func $crypto_xor (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $i i32.add
      local.get $dst local.get $i i32.add i32.load8_u
      local.get $src local.get $i i32.add i32.load8_u
      i32.xor i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ══════════════════════════════════════════════════════════════
  ;; GHASH: GF(2^128) multiply and absorb
  ;; ══════════════════════════════════════════════════════════════

  ;; ── shift_v — shift V (16 bytes at global $V) right by 1 bit ──
  ;; After shift, if the LSB of the original V was 1, XOR byte[0] with 0xE1
  (func $shift_v
    (local $i i32) (local $byte i32) (local $carry i32) (local $lsb i32)
    ;; Save LSB of byte 15 before modifications
    global.get $V i32.const 15 i32.add i32.load8_u
    i32.const 1 i32.and
    local.set $lsb

    i32.const 0 local.set $carry
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 16 i32.ge_u br_if $done
      global.get $V local.get $i i32.add i32.load8_u local.set $byte
      local.get $byte i32.const 1 i32.and local.set $byte  ;; save carry-out (LSB of original)
      global.get $V local.get $i i32.add
      global.get $V local.get $i i32.add i32.load8_u
      i32.const 1 i32.shr_u
      local.get $carry i32.const 7 i32.shl i32.or
      i32.store8
      local.get $byte local.set $carry
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end

    local.get $lsb i32.eqz
    if return end
    global.get $V i32.const 0 i32.add
    global.get $V i32.load8_u i32.const 0xE1 i32.xor
    i32.store8)

  ;; ── ghash_mul — multiply X by H in GF(2^128) ──
  ;; Operates on global $X (in/out) and $H (input)
  (func $ghash_mul
    (local $byte_idx i32) (local $bit_mask i32) (local $byte_val i32)
    (local $i i32) (local $tmp i32)

    ;; Z = 0
    global.get $Z i32.const 0 i32.const 16 call $memset

    ;; V = H
    global.get $V global.get $H i32.const 16 call $memcpy

    i32.const 0 local.set $byte_idx
    block $byte_done
    loop $byte_loop
      local.get $byte_idx i32.const 16 i32.ge_u br_if $byte_done

      global.get $X local.get $byte_idx i32.add i32.load8_u local.set $byte_val
      i32.const 0x80 local.set $bit_mask

      block $bit_done
      loop $bit_loop
        local.get $bit_mask i32.eqz br_if $bit_done

        local.get $byte_val local.get $bit_mask i32.and
        if
          ;; Z = Z XOR V
          i32.const 0 local.set $i
          block $xor_done
          loop $xor_loop
            local.get $i i32.const 16 i32.ge_u br_if $xor_done
            global.get $Z local.get $i i32.add
            global.get $Z local.get $i i32.add i32.load8_u
            global.get $V local.get $i i32.add i32.load8_u
            i32.xor i32.store8
            local.get $i i32.const 1 i32.add local.set $i
            br $xor_loop
          end
          end
        end

        call $shift_v
        local.get $bit_mask i32.const 1 i32.shr_u local.set $bit_mask
        br $bit_loop
      end
      end

      local.get $byte_idx i32.const 1 i32.add local.set $byte_idx
      br $byte_loop
    end
    end

    ;; X = Z
    global.get $X global.get $Z i32.const 16 call $memcpy)

  ;; ── ghash_absorb(data_ptr, data_len) — absorb data into GHASH ──
  ;; Operates on global $X (in/out) and $H (input)
  (func $ghash_absorb (param $data i32) (param $len i32)
    (local $blk i32)
    block $done
    loop $loop
      local.get $len i32.eqz br_if $done

      i32.const 16 local.set $blk
      local.get $len i32.const 16 i32.lt_u
      if
        ;; Partial block: copy to TMP, zero pad to 16
        local.get $len local.set $blk
        global.get $TMP i32.const 0 i32.const 16 call $memset
        global.get $TMP local.get $data local.get $blk call $memcpy

        ;; XOR TMP into X, then multiply
        global.get $X global.get $TMP call $xor16
        call $ghash_mul
      else
        ;; Full block: XOR data directly into X
        global.get $X local.get $data call $xor16
        call $ghash_mul
      end

      local.get $data local.get $blk i32.add local.set $data
      local.get $len local.get $blk i32.sub local.set $len
      br $loop
    end
    end)

  ;; ══════════════════════════════════════════════════════════════
  ;; AES-CTR keystream XOR (for GCM encryption/decryption)
  ;; ══════════════════════════════════════════════════════════════

  ;; ── aes_ctr_xor(out, in, len, rk, ctr) — XOR data with AES-CTR keystream ──
  (func $aes_ctr_xor (param $out i32) (param $in i32) (param $len i32)
                      (param $rk i32) (param $ctr i32)
    (local $rem i32) (local $blk i32) (local $i i32)
    local.get $len local.set $rem
    block $done
    loop $loop
      local.get $rem i32.eqz br_if $done

      i32.const 16 local.set $blk
      local.get $rem i32.const 16 i32.lt_u
      if local.get $rem local.set $blk end

      ;; KS = AES(ctr)
      global.get $KS local.get $ctr i32.const 16 call $memcpy
      global.get $KS local.get $rk call $aes128_encrypt_block

      ;; XOR blk bytes
      i32.const 0 local.set $i
      block $xor_done
      loop $xor_loop
        local.get $i local.get $blk i32.ge_u br_if $xor_done
        local.get $out local.get $i i32.add
        local.get $in local.get $i i32.add i32.load8_u
        global.get $KS local.get $i i32.add i32.load8_u
        i32.xor i32.store8
        local.get $i i32.const 1 i32.add local.set $i
        br $xor_loop
      end
      end

      local.get $out local.get $blk i32.add local.set $out
      local.get $in local.get $blk i32.add local.set $in
      local.get $rem local.get $blk i32.sub local.set $rem

      ;; Increment counter (big-endian inc32 on low 32 bits)
      local.get $ctr i32.const 12 i32.add
      local.get $ctr i32.const 12 i32.add call $load_be32 i32.const 1 i32.add
      call $store_be32
      br $loop
    end
    end)

  ;; ══════════════════════════════════════════════════════════════
  ;; AES-128-GCM encrypt
  ;; ══════════════════════════════════════════════════════════════

  (func $aes128_gcm_encrypt (export "aes128_gcm_encrypt")
    (param $out i32) (param $in i32) (param $len i32)
    (param $aad i32) (param $aad_len i32)
    (param $key i32) (param $iv i32) (param $tag i32)
    (result i32)

    local.get $key global.get $RK call $aes128_key_expand

    ;; 1. H = AES_K(0^128)
    global.get $H i32.const 0 i32.const 16 call $memset
    global.get $H global.get $RK call $aes128_encrypt_block

    ;; 2. J0 = IV || 0x00000001
    global.get $J0 local.get $iv i32.const 12 call $memcpy
    global.get $J0 i32.const 12 i32.add i32.const 0x00000001 call $store_be32

    ;; 3. tagmask = AES_K(J0)
    global.get $TAGMASK global.get $J0 i32.const 16 call $memcpy
    global.get $TAGMASK global.get $RK call $aes128_encrypt_block

    ;; 4. CTR encrypt with counter = IV || 0x00000002
    global.get $CTR local.get $iv i32.const 12 call $memcpy
    global.get $CTR i32.const 12 i32.add i32.const 0x00000002 call $store_be32
    local.get $out local.get $in local.get $len global.get $RK global.get $CTR call $aes_ctr_xor

    ;; 5. GHASH over AAD || ciphertext || len_block
    ;; Reset X = 0
    global.get $X i32.const 0 i32.const 16 call $memset

    ;; Absorb AAD
    local.get $aad local.get $aad_len call $ghash_absorb

    ;; Absorb ciphertext
    local.get $out local.get $len call $ghash_absorb

    ;; Finalize: absorb len_block = uint64_be(aad_len * 8) || uint64_be(len * 8)
    global.get $LEN_BLOCK
    local.get $aad_len i64.extend_i32_u i64.const 3 i64.shl
    call $store_be64
    global.get $LEN_BLOCK i32.const 8 i32.add
    local.get $len i64.extend_i32_u i64.const 3 i64.shl
    call $store_be64
    global.get $LEN_BLOCK i32.const 16 call $ghash_absorb

    ;; Tag = X XOR tagmask
    local.get $tag global.get $X i32.const 16 call $memcpy
    local.get $tag global.get $TAGMASK i32.const 16 call $crypto_xor

    i32.const 0)

  ;; ══════════════════════════════════════════════════════════════
  ;; AES-128-GCM decrypt
  ;; ══════════════════════════════════════════════════════════════

  (func $aes128_gcm_decrypt (export "aes128_gcm_decrypt")
    (param $out i32) (param $in i32) (param $len i32)
    (param $aad i32) (param $aad_len i32)
    (param $key i32) (param $iv i32) (param $tag i32)
    (result i32)
    (local $i i32) (local $ok i32)

    ;; Expand key
    local.get $key global.get $RK call $aes128_key_expand

    ;; 1. H = AES_K(0^128)
    global.get $H i32.const 0 i32.const 16 call $memset
    global.get $H global.get $RK call $aes128_encrypt_block

    ;; 2. J0 = IV || 0x00000001
    global.get $J0 local.get $iv i32.const 12 call $memcpy
    global.get $J0 i32.const 12 i32.add i32.const 0x00000001 call $store_be32

    ;; 3. tagmask = AES_K(J0)
    global.get $TAGMASK global.get $J0 i32.const 16 call $memcpy
    global.get $TAGMASK global.get $RK call $aes128_encrypt_block

    ;; 4. Compute GHASH over AAD || ciphertext (input) || len_block
    global.get $X i32.const 0 i32.const 16 call $memset

    ;; Absorb AAD
    local.get $aad local.get $aad_len call $ghash_absorb

    ;; Absorb ciphertext (from input, not output)
    local.get $in local.get $len call $ghash_absorb

    ;; Finalize: absorb len_block
    global.get $LEN_BLOCK
    local.get $aad_len i64.extend_i32_u i64.const 3 i64.shl
    call $store_be64
    global.get $LEN_BLOCK i32.const 8 i32.add
    local.get $len i64.extend_i32_u i64.const 3 i64.shl
    call $store_be64
    global.get $LEN_BLOCK i32.const 16 call $ghash_absorb

    ;; Expected tag = X XOR tagmask
    global.get $TMP global.get $X i32.const 16 call $memcpy
    global.get $TMP global.get $TAGMASK i32.const 16 call $crypto_xor

    ;; Constant-time compare tag == expected
    i32.const 0 local.set $ok
    i32.const 0 local.set $i
    block $cmp_done
    loop $cmp_loop
      local.get $i i32.const 16 i32.ge_u br_if $cmp_done
      local.get $tag local.get $i i32.add i32.load8_u
      global.get $TMP local.get $i i32.add i32.load8_u
      i32.xor
      local.get $ok i32.or
      local.set $ok
      local.get $i i32.const 1 i32.add local.set $i
      br $cmp_loop
    end
    end

    block $mismatch
      local.get $ok br_if $mismatch

      ;; Tag matches: decrypt ciphertext to plaintext
      global.get $CTR local.get $iv i32.const 12 call $memcpy
      global.get $CTR i32.const 12 i32.add i32.const 0x00000002 call $store_be32
      local.get $out local.get $in local.get $len global.get $RK global.get $CTR call $aes_ctr_xor
      i32.const 0
      return
    end

    i32.const -1)
