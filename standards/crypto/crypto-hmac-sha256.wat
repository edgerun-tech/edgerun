;; HMAC-SHA256 (RFC 2104) — self-contained with inline SHA-256.
  
  ;; Memory layout:
  ;; 16384-16639: W array (64 × 4 bytes)
  ;; 16640-16671: hash state H (8 × 4 bytes)
  ;; 16672-16895: block buffer (64 bytes)
  ;; 16896+: scratch buffer (for key padding + message)

  (func $ror (param $x i32) (param $n i32) (result i32)
    local.get $x local.get $n i32.shr_u
    local.get $x i32.const 32 local.get $n i32.sub i32.shl
    i32.or)

  (func $K (param $i i32) (result i32)
    local.get $i
    call $m62k)

  ;; SHA-256 compress one 64-byte block (little-endian words in W array)
  (func $m59compress (param $W i32) (param $H i32)
    (local $i i32)
    (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local $e i32) (local $f i32) (local $g i32) (local $h i32)
    (local $t1 i32) (local $t2 i32)
    (local $s0v i32) (local $s1v i32)

    local.get $H i32.load offset=0 local.set $a
    local.get $H i32.load offset=4 local.set $b
    local.get $H i32.load offset=8 local.set $c
    local.get $H i32.load offset=12 local.set $d
    local.get $H i32.load offset=16 local.set $e
    local.get $H i32.load offset=20 local.set $f
    local.get $H i32.load offset=24 local.set $g
    local.get $H i32.load offset=28 local.set $h

    i32.const 0 local.set $i
    block $rnd
    loop $rndl
      local.get $i i32.const 64 i32.ge_u br_if $rnd

      ;; t1 = h + S1(e) + ch(e,f,g) + K[i] + W[i]
      local.get $h
      local.get $e i32.const 6 call $ror
      local.get $e i32.const 11 call $ror i32.xor
      local.get $e i32.const 25 call $ror i32.xor
      i32.add
      local.get $e local.get $f i32.and
      local.get $e i32.const -1 i32.xor local.get $g i32.and
      i32.xor
      i32.add
      local.get $i call $K
      i32.add
      local.get $W local.get $i i32.const 2 i32.shl i32.add i32.load
      i32.add
      local.set $t1

      ;; t2 = S0(a) + maj(a,b,c)
      local.get $a i32.const 2 call $ror
      local.get $a i32.const 13 call $ror i32.xor
      local.get $a i32.const 22 call $ror i32.xor
      local.set $t2
      local.get $t2
      local.get $a local.get $b i32.and
      local.get $a local.get $c i32.and i32.or
      local.get $b local.get $c i32.and i32.or
      i32.add
      local.set $t2

      local.get $g local.set $h
      local.get $f local.set $g
      local.get $e local.set $f
      local.get $d local.get $t1 i32.add local.set $e
      local.get $c local.set $d
      local.get $b local.set $c
      local.get $a local.set $b
      local.get $t1 local.get $t2 i32.add local.set $a

      local.get $i i32.const 1 i32.add local.set $i
      br $rndl
    end
    end

    local.get $H local.get $H i32.load offset=0 local.get $a i32.add i32.store offset=0
    local.get $H local.get $H i32.load offset=4 local.get $b i32.add i32.store offset=4
    local.get $H local.get $H i32.load offset=8 local.get $c i32.add i32.store offset=8
    local.get $H local.get $H i32.load offset=12 local.get $d i32.add i32.store offset=12
    local.get $H local.get $H i32.load offset=16 local.get $e i32.add i32.store offset=16
    local.get $H local.get $H i32.load offset=20 local.get $f i32.add i32.store offset=20
    local.get $H local.get $H i32.load offset=24 local.get $g i32.add i32.store offset=24
    local.get $H local.get $H i32.load offset=28 local.get $h i32.add i32.store offset=28)

  ;; sha256(data, len) — processes data, stores hash in H buffer (16640)
  ;; Returns hash in H as little-endian words (not yet big-endian serialized)
  (func $sha256_process (param $data i32) (param $len i32)
    (local $W i32) (local $H i32) (local $blk i32)
    (local $i i32) (local $rem i32)

    i32.const 16384 local.set $W
    i32.const 16640 local.set $H
    i32.const 16672 local.set $blk

    ;; Init H
    local.get $H i32.const 0x6a09e667 i32.store offset=0
    local.get $H i32.const 0xbb67ae85 i32.store offset=4
    local.get $H i32.const 0x3c6ef372 i32.store offset=8
    local.get $H i32.const 0xa54ff53a i32.store offset=12
    local.get $H i32.const 0x510e527f i32.store offset=16
    local.get $H i32.const 0x9b05688c i32.store offset=20
    local.get $H i32.const 0x1f83d9ab i32.store offset=24
    local.get $H i32.const 0x5be0cd19 i32.store offset=28

    ;; Process full 64-byte blocks
    local.get $len i32.const 6 i32.shr_u local.set $i
    block $full
    loop $full_l
      local.get $i i32.eqz br_if $full

      ;; Load block into W[0..15] big-endian
      local.get $data call $load_block_into_W
      local.get $W local.get $H call $m59compress

      local.get $data i32.const 64 i32.add local.set $data
      local.get $i i32.const 1 i32.sub local.set $i
      br $full_l
    end
    end

    ;; Pad and process final block(s)
    local.get $len i32.const 0x3F i32.and local.set $rem

    ;; Copy remaining bytes to blk
    local.get $blk i32.const 64 i32.const 0 call $m59memset
    local.get $blk local.get $data local.get $rem call $memcpy
    local.get $blk local.get $rem i32.add i32.const 0x80 i32.store8

    ;; Check if we have room for 8-byte length
    local.get $rem i32.const 56 i32.ge_u
    if
      ;; need another block — process current
      local.get $blk call $load_block_into_W
      local.get $W local.get $H call $m59compress
      local.get $blk i32.const 64 i32.const 0 call $m59memset
    end

    ;; Write length (big-endian, last 8 bytes = len * 8 in bits)
    local.get $blk i32.const 63 i32.add local.get $len i32.const 3 i32.shl i32.const 0xFF i32.and i32.store8
    local.get $blk i32.const 62 i32.add local.get $len i32.const 3 i32.shl i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $blk i32.const 61 i32.add local.get $len i32.const 3 i32.shl i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8
    local.get $blk i32.const 60 i32.add local.get $len i32.const 3 i32.shl i32.const 24 i32.shr_u i32.const 0xFF i32.and i32.store8
    ;; Upper 32 bits of length = 0 (since len < 2^29 for our use case)
    local.get $blk i32.const 59 i32.add i32.const 0 i32.store8
    local.get $blk i32.const 58 i32.add i32.const 0 i32.store8
    local.get $blk i32.const 57 i32.add i32.const 0 i32.store8
    local.get $blk i32.const 56 i32.add i32.const 0 i32.store8

    local.get $blk call $load_block_into_W
    local.get $W local.get $H call $m59compress)

  ;; Load 64-byte block from $data into W[0..15] (big-endian decode)
  (func $load_block_into_W (param $data i32)
    (local $i i32) (local $W i32) (local $w2 i32) (local $w7 i32) (local $w15 i32)
    i32.const 16384 local.set $W
    i32.const 0 local.set $i
    block $w0
    loop $w0l
      local.get $i i32.const 16 i32.ge_u br_if $w0
      local.get $W local.get $i i32.const 2 i32.shl i32.add
      local.get $data local.get $i i32.const 2 i32.shl i32.add call $load_be32
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $w0l
    end
    end
    ;; Extend W[16..63]
    i32.const 16 local.set $i
    block $we
    loop $wel
      local.get $i i32.const 64 i32.ge_u br_if $we
      local.get $W local.get $i i32.const 2 i32.sub i32.const 2 i32.shl i32.add i32.load
      local.set $w2
      local.get $W local.get $i i32.const 7 i32.sub i32.const 2 i32.shl i32.add i32.load
      local.set $w7
      local.get $W local.get $i i32.const 15 i32.sub i32.const 2 i32.shl i32.add i32.load
      local.set $w15

      local.get $W local.get $i i32.const 2 i32.shl i32.add
      local.get $w2 i32.const 17 call $ror
      local.get $w2 i32.const 19 call $ror i32.xor
      local.get $w2 i32.const 10 i32.shr_u i32.xor
      local.get $w7
      i32.add
      local.get $w15 i32.const 7 call $ror
      local.get $w15 i32.const 18 call $ror i32.xor
      local.get $w15 i32.const 3 i32.shr_u i32.xor
      i32.add
      local.get $W local.get $i i32.const 16 i32.sub i32.const 2 i32.shl i32.add i32.load
      i32.add
      i32.store
      local.get $i i32.const 1 i32.add local.set $i
      br $wel
    end
    end)

  ;; ── hmac_sha256(key, klen, msg, mlen, out, ocap) -> i32 ──
  (func $hmac_sha256 (export "hmac_sha256")
    (param $key i32) (param $klen i32)
    (param $msg i32) (param $mlen i32)
    (param $out i32) (param $ocap i32)
    (result i32)
    (local $buf i32) (local $H i32) (local $i i32) (local $b i32) (local $blk i32)

    local.get $ocap i32.const 32 i32.lt_u
    if i32.const 2 return end

    i32.const 16896 local.set $buf  ;; scratch buffer for key + message
    i32.const 16640 local.set $H
    i32.const 16992 local.set $blk  ;; inner hash save (after buf+64+32)

    ;; ── 1. Pad key to 64 bytes in buf ──
    local.get $klen i32.const 64 i32.le_u
    if
      ;; Short key: copy and zero-fill
      local.get $buf local.get $key local.get $klen call $memcpy
      local.get $klen local.set $i
      block $zfill
      loop $zfl
        local.get $i i32.const 64 i32.ge_u br_if $zfill
        local.get $buf local.get $i i32.add i32.const 0 i32.store8
        local.get $i i32.const 1 i32.add local.set $i
        br $zfl
      end
      end
    else
      ;; Long key: hash it first → buf[0..31], then zero-fill to 64
      local.get $key local.get $klen call $sha256_process
      ;; Store hash (big-endian) into buf
      local.get $buf local.get $H call $hash_to_buf
      i32.const 32 local.set $i
      block $z32
      loop $z32l
        local.get $i i32.const 64 i32.ge_u br_if $z32
        local.get $buf local.get $i i32.add i32.const 0 i32.store8
        local.get $i i32.const 1 i32.add local.set $i
        br $z32l
      end
      end
    end

    ;; ── 2. Inner hash: SHA256((K' ⊕ 0x36) || msg) ──
    ;; XOR buf[0..63] with 0x36
    i32.const 0 local.set $i
    block $ipad
    loop $ipadl
      local.get $i i32.const 64 i32.ge_u br_if $ipad
      local.get $buf local.get $i i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.const 0x36 i32.xor
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $ipadl
    end
    end

    ;; Concatenate msg after ipad_key (at buf+64)
    local.get $buf i32.const 64 i32.add local.get $msg local.get $mlen call $memcpy

    ;; SHA256 of (buf, 64 + mlen)
    local.get $buf i32.const 64 local.get $mlen i32.add call $sha256_process

    ;; Save inner hash from H to blk (big-endian, 32 bytes)
    ;; NOTE: must not overwrite buf[0..63] — key material needed for restore/opad
    ;; blk is at fixed address 16672 (defined in sha256_process)
    i32.const 16672 local.get $H call $hash_to_buf

    ;; ── 3. Restore key in buf (undo ipad) ──
    i32.const 0 local.set $i
    block $restore
    loop $restorel
      local.get $i i32.const 64 i32.ge_u br_if $restore
      local.get $buf local.get $i i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.const 0x36 i32.xor
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $restorel
    end
    end

    ;; ── 4. Outer hash: SHA256((K' ⊕ 0x5C) || inner_hash) ──
    ;; XOR buf[0..63] with 0x5C
    i32.const 0 local.set $i
    block $opad
    loop $opadl
      local.get $i i32.const 64 i32.ge_u br_if $opad
      local.get $buf local.get $i i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.const 0x5C i32.xor
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $opadl
    end
    end

    ;; Copy inner hash from blk (16672) to buf+64
    local.get $buf i32.const 64 i32.add i32.const 16672 i32.const 32 call $memcpy
    ;; SHA256 of (buf, 64+32 = 96)
    local.get $buf i32.const 96 call $sha256_process

    ;; ── 5. Write final HMAC to output ──
    local.get $out local.get $H call $hash_to_buf

    ;; Undo opad in buf (restore key)
    i32.const 0 local.set $i
    block $restore2
    loop $restorel2
      local.get $i i32.const 64 i32.ge_u br_if $restore2
      local.get $buf local.get $i i32.add
      local.get $buf local.get $i i32.add i32.load8_u i32.const 0x5C i32.xor
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $restorel2
    end
    end

    i32.const 0)

  ;; Write hash state (H) to buf as 32 big-endian bytes
  (func $hash_to_buf (param $buf i32) (param $H i32)
    local.get $buf i32.const 0 i32.add local.get $H i32.load offset=0 call $store_be32
    local.get $buf i32.const 4 i32.add local.get $H i32.load offset=4 call $store_be32
    local.get $buf i32.const 8 i32.add local.get $H i32.load offset=8 call $store_be32
    local.get $buf i32.const 12 i32.add local.get $H i32.load offset=12 call $store_be32
    local.get $buf i32.const 16 i32.add local.get $H i32.load offset=16 call $store_be32
    local.get $buf i32.const 20 i32.add local.get $H i32.load offset=20 call $store_be32
    local.get $buf i32.const 24 i32.add local.get $H i32.load offset=24 call $store_be32
    local.get $buf i32.const 28 i32.add local.get $H i32.load offset=28 call $store_be32)

  ;; ── memset ──
  (func $m59memset (param $dst i32) (param $len i32) (param $val i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $i i32.add local.get $val i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)
