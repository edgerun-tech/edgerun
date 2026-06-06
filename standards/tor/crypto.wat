;; Tor crypto WAT slice from pre-erobj commit 1b243c8d6a843585bc54b4366f814cf5f56615b1.
  ;; Memory is exported so callers can place inputs/contexts directly.
  ;; Public layout constants.
  (global $SHA256_CTX_SIZE (export "SHA256_CTX_SIZE") i32 (i32.const 108))
  (global $SHA256_CTX_H (export "SHA256_CTX_H") i32 (i32.const 0))
  (global $SHA256_CTX_COUNT_LO (export "SHA256_CTX_COUNT_LO") i32 (i32.const 32))
  (global $SHA256_CTX_COUNT_HI (export "SHA256_CTX_COUNT_HI") i32 (i32.const 36))
  (global $SHA256_CTX_BUF (export "SHA256_CTX_BUF") i32 (i32.const 40))
  (global $SHA256_CTX_BUFLEN (export "SHA256_CTX_BUFLEN") i32 (i32.const 104))
  (global $TOR_WAT_SHA256_W (export "TOR_WAT_SHA256_W") i32 (i32.const 4096))
  (global $TOR_WAT_HMAC_KEY_BLOCK (export "TOR_WAT_HMAC_KEY_BLOCK") i32 (i32.const 4608))
  (global $TOR_WAT_HMAC_INNER_DIGEST (export "TOR_WAT_HMAC_INNER_DIGEST") i32 (i32.const 4672))
  (global $TOR_WAT_WORK_SHA_CTX (export "TOR_WAT_WORK_SHA_CTX") i32 (i32.const 4864))
  (global $TOR_WAT_AES_ROUND_KEYS (export "TOR_WAT_AES_ROUND_KEYS") i32 (i32.const 5120))
  (global $TOR_WAT_AES_COUNTER (export "TOR_WAT_AES_COUNTER") i32 (i32.const 5312))
  (global $TOR_WAT_AES_STREAM (export "TOR_WAT_AES_STREAM") i32 (i32.const 5328))
  (global $TOR_WAT_AES_TMP (export "TOR_WAT_AES_TMP") i32 (i32.const 5344))

  (data (i32.const 0)
    "\98\2f\8a\42\91\44\37\71\cf\fb\c0\b5\a5\db\b5\e9\5b\c2\56\39\f1\11\f1\59\a4\f2\3f\92\d5\5e\1c\ab"
    "\98\aa\07\d8\01\5b\83\12\be\85\31\24\c3\7d\0c\55\74\5d\be\72\fe\b1\de\80\a7\06\dc\9b\74\f1\9b\c1"
    "\c1\69\9b\e4\86\47\be\ef\c6\9d\c1\0f\cc\a1\0c\24\6f\2c\e9\2d\aa\84\74\4a\dc\a9\b0\5c\da\88\f9\76"
    "\52\51\3e\98\6d\c6\31\a8\c8\27\03\b0\c7\7f\59\bf\f3\0b\e0\c6\47\91\a7\d5\51\63\ca\06\67\29\29\14"
    "\85\0a\b7\27\38\21\1b\2e\fc\6d\2c\4d\13\0d\38\53\54\73\0a\65\bb\0a\6a\76\2e\c9\c2\81\85\2c\72\92"
    "\a1\e8\bf\a2\4b\66\1a\a8\70\8b\4b\c2\a3\51\6c\c7\19\e8\92\d1\24\06\99\d6\85\35\0e\f4\70\a0\6a\10"
    "\16\c1\a4\19\08\6c\37\1e\4c\77\48\27\b5\bc\b0\34\b3\0c\1c\39\4a\aa\d8\4e\4f\ca\9c\5b\f3\6f\2e\68"
    "\ee\82\8f\74\6f\63\a5\78\14\78\c8\84\08\02\c7\8c\fa\ff\be\90\eb\6c\50\a4\f7\9f\f9\be\f2\78\71\c6")
  (data (i32.const 256) "\67\e6\09\6a\85\ae\67\bb\72\f3\6e\3c\3a\f5\4f\a5\7f\52\0e\51\8c\68\05\9b\ab\d9\83\1f\19\cd\e0\5b")
  (data (i32.const 512)
    "\63\7c\77\7b\f2\6b\6f\c5\30\01\67\2b\fe\d7\ab\76\ca\82\c9\7d\fa\59\47\f0\ad\d4\a2\af\9c\a4\72\c0"
    "\b7\fd\93\26\36\3f\f7\cc\34\a5\e5\f1\71\d8\31\15\04\c7\23\c3\18\96\05\9a\07\12\80\e2\eb\27\b2\75"
    "\09\83\2c\1a\1b\6e\5a\a0\52\3b\d6\b3\29\e3\2f\84\53\d1\00\ed\20\fc\b1\5b\6a\cb\be\39\4a\4c\58\cf"
    "\d0\ef\aa\fb\43\4d\33\85\45\f9\02\7f\50\3c\9f\a8\51\a3\40\8f\92\9d\38\f5\bc\b6\da\21\10\ff\f3\d2"
    "\cd\0c\13\ec\5f\97\44\17\c4\a7\7e\3d\64\5d\19\73\60\81\4f\dc\22\2a\90\88\46\ee\b8\14\de\5e\0b\db"
    "\e0\32\3a\0a\49\06\24\5c\c2\d3\ac\62\91\95\e4\79\e7\c8\37\6d\8d\d5\4e\a9\6c\56\f4\ea\65\7a\ae\08"
    "\ba\78\25\2e\1c\a6\b4\c6\e8\dd\74\1f\4b\bd\8b\8a\70\3e\b5\66\48\03\f6\0e\61\35\57\b9\86\c1\1d\9e"
    "\e1\f8\98\11\69\d9\8e\94\9b\1e\87\e9\ce\55\28\df\8c\a1\89\0d\bf\e6\42\68\41\99\2d\0f\b0\54\bb\16")
  (data (i32.const 768) "\01\02\04\08\10\20\40\80\1b\36")

  (func $m24rotr (param $x i32) (param $n i32) (result i32)
    (i32.or (i32.shr_u (local.get $x) (local.get $n))
            (i32.shl (local.get $x) (i32.sub (i32.const 32) (local.get $n)))))

  (func $load32be (param $p i32) (result i32)
    (i32.or
      (i32.or (i32.shl (i32.load8_u (local.get $p)) (i32.const 24))
              (i32.shl (i32.load8_u (i32.add (local.get $p) (i32.const 1))) (i32.const 16)))
      (i32.or (i32.shl (i32.load8_u (i32.add (local.get $p) (i32.const 2))) (i32.const 8))
              (i32.load8_u (i32.add (local.get $p) (i32.const 3))))))




  (func $sha256_compress (param $ctx i32) (param $block i32)
    (local $t i32) (local $x i32) (local $s0 i32) (local $s1 i32)
    (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local $e i32) (local $f i32) (local $g i32) (local $h i32)
    (local $ch i32) (local $maj i32) (local $t1 i32) (local $t2 i32)
    (local.set $t (i32.const 0))
    (block $load_done
      (loop $load
        (br_if $load_done (i32.ge_u (local.get $t) (i32.const 16)))
        (i32.store (i32.add (i32.const 4096) (i32.shl (local.get $t) (i32.const 2)))
          (call $load32be (i32.add (local.get $block) (i32.shl (local.get $t) (i32.const 2)))))
        (local.set $t (i32.add (local.get $t) (i32.const 1)))
        (br $load)))
    (block $ext_done
      (loop $ext
        (br_if $ext_done (i32.ge_u (local.get $t) (i32.const 64)))
        (local.set $x (i32.load (i32.add (i32.const 4096) (i32.shl (i32.sub (local.get $t) (i32.const 15)) (i32.const 2)))))
        (local.set $s0 (i32.xor (i32.xor (call $m24rotr (local.get $x) (i32.const 7)) (call $m24rotr (local.get $x) (i32.const 18))) (i32.shr_u (local.get $x) (i32.const 3))))
        (local.set $x (i32.load (i32.add (i32.const 4096) (i32.shl (i32.sub (local.get $t) (i32.const 2)) (i32.const 2)))))
        (local.set $s1 (i32.xor (i32.xor (call $m24rotr (local.get $x) (i32.const 17)) (call $m24rotr (local.get $x) (i32.const 19))) (i32.shr_u (local.get $x) (i32.const 10))))
        (i32.store (i32.add (i32.const 4096) (i32.shl (local.get $t) (i32.const 2)))
          (i32.add (i32.add (i32.add
            (i32.load (i32.add (i32.const 4096) (i32.shl (i32.sub (local.get $t) (i32.const 16)) (i32.const 2))))
            (local.get $s0))
            (i32.load (i32.add (i32.const 4096) (i32.shl (i32.sub (local.get $t) (i32.const 7)) (i32.const 2)))))
            (local.get $s1)))
        (local.set $t (i32.add (local.get $t) (i32.const 1)))
        (br $ext)))
    (local.set $a (i32.load (i32.add (local.get $ctx) (i32.const 0))))
    (local.set $b (i32.load (i32.add (local.get $ctx) (i32.const 4))))
    (local.set $c (i32.load (i32.add (local.get $ctx) (i32.const 8))))
    (local.set $d (i32.load (i32.add (local.get $ctx) (i32.const 12))))
    (local.set $e (i32.load (i32.add (local.get $ctx) (i32.const 16))))
    (local.set $f (i32.load (i32.add (local.get $ctx) (i32.const 20))))
    (local.set $g (i32.load (i32.add (local.get $ctx) (i32.const 24))))
    (local.set $h (i32.load (i32.add (local.get $ctx) (i32.const 28))))
    (local.set $t (i32.const 0))
    (block $round_done
      (loop $round
        (br_if $round_done (i32.ge_u (local.get $t) (i32.const 64)))
        (local.set $s1 (i32.xor (i32.xor (call $m24rotr (local.get $e) (i32.const 6)) (call $m24rotr (local.get $e) (i32.const 11))) (call $m24rotr (local.get $e) (i32.const 25))))
        (local.set $ch (i32.xor (i32.and (local.get $e) (local.get $f)) (i32.and (i32.xor (local.get $e) (i32.const -1)) (local.get $g))))
        (local.set $s0 (i32.xor (i32.xor (call $m24rotr (local.get $a) (i32.const 2)) (call $m24rotr (local.get $a) (i32.const 13))) (call $m24rotr (local.get $a) (i32.const 22))))
        (local.set $maj (i32.xor (i32.xor (i32.and (local.get $a) (local.get $b)) (i32.and (local.get $a) (local.get $c))) (i32.and (local.get $b) (local.get $c))))
        (local.set $t1 (i32.add (local.get $h) (local.get $s1)))
        (local.set $t1 (i32.add (local.get $t1) (local.get $ch)))
        (local.set $t1 (i32.add (local.get $t1) (i32.load (i32.add (i32.const 0) (i32.shl (local.get $t) (i32.const 2))))))
        (local.set $t1 (i32.add (local.get $t1) (i32.load (i32.add (i32.const 4096) (i32.shl (local.get $t) (i32.const 2))))))
        (local.set $t2 (i32.add (local.get $s0) (local.get $maj)))
        (local.set $h (local.get $g))
        (local.set $g (local.get $f))
        (local.set $f (local.get $e))
        (local.set $e (i32.add (local.get $d) (local.get $t1)))
        (local.set $d (local.get $c))
        (local.set $c (local.get $b))
        (local.set $b (local.get $a))
        (local.set $a (i32.add (local.get $t1) (local.get $t2)))
        (local.set $t (i32.add (local.get $t) (i32.const 1)))
        (br $round)))
    (i32.store (i32.add (local.get $ctx) (i32.const 0)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 0))) (local.get $a)))
    (i32.store (i32.add (local.get $ctx) (i32.const 4)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 4))) (local.get $b)))
    (i32.store (i32.add (local.get $ctx) (i32.const 8)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 8))) (local.get $c)))
    (i32.store (i32.add (local.get $ctx) (i32.const 12)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 12))) (local.get $d)))
    (i32.store (i32.add (local.get $ctx) (i32.const 16)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 16))) (local.get $e)))
    (i32.store (i32.add (local.get $ctx) (i32.const 20)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 20))) (local.get $f)))
    (i32.store (i32.add (local.get $ctx) (i32.const 24)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 24))) (local.get $g)))
    (i32.store (i32.add (local.get $ctx) (i32.const 28)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 28))) (local.get $h))))

  (func $er_sha256_init (export "er_sha256_init") (param $ctx i32) (result i32)
    (if (i32.eqz (local.get $ctx)) (then (return (i32.const 0))))
    (drop (call $m26memcpy (local.get $ctx) (i32.const 256) (i32.const 32)))
    (i32.store (i32.add (local.get $ctx) (i32.const 32)) (i32.const 0))
    (i32.store (i32.add (local.get $ctx) (i32.const 36)) (i32.const 0))
    (call $m26memset (i32.add (local.get $ctx) (i32.const 40)) (i32.const 0) (i32.const 64))
    (i32.store (i32.add (local.get $ctx) (i32.const 104)) (i32.const 0))
    (local.get $ctx))

  (func $er_sha256_update (export "er_sha256_update") (param $ctx i32) (param $data i32) (param $len i32) (result i32)
    (local $pos i32) (local $take i32) (local $lo i32)
    (if (i32.eqz (local.get $ctx)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $len)) (then (return (local.get $ctx))))
    (if (i32.eqz (local.get $data)) (then (return (i32.const 0))))
    (local.set $lo (i32.load (i32.add (local.get $ctx) (i32.const 32))))
    (i32.store (i32.add (local.get $ctx) (i32.const 32)) (i32.add (local.get $lo) (local.get $len)))
    (if (i32.lt_u (i32.add (local.get $lo) (local.get $len)) (local.get $lo))
      (then (i32.store (i32.add (local.get $ctx) (i32.const 36)) (i32.add (i32.load (i32.add (local.get $ctx) (i32.const 36))) (i32.const 1)))))
    (local.set $pos (i32.load (i32.add (local.get $ctx) (i32.const 104))))
    (if (local.get $pos)
      (then
        (local.set $take (i32.sub (i32.const 64) (local.get $pos)))
        (if (i32.lt_u (local.get $len) (local.get $take)) (then (local.set $take (local.get $len))))
        (drop (call $m26memcpy (i32.add (i32.add (local.get $ctx) (i32.const 40)) (local.get $pos)) (local.get $data) (local.get $take)))
        (local.set $pos (i32.add (local.get $pos) (local.get $take)))
        (local.set $data (i32.add (local.get $data) (local.get $take)))
        (local.set $len (i32.sub (local.get $len) (local.get $take)))
        (i32.store (i32.add (local.get $ctx) (i32.const 104)) (local.get $pos))
        (if (i32.eq (local.get $pos) (i32.const 64))
          (then
            (call $sha256_compress (local.get $ctx) (i32.add (local.get $ctx) (i32.const 40)))
            (i32.store (i32.add (local.get $ctx) (i32.const 104)) (i32.const 0))))))
    (block $blocks_done
      (loop $blocks
        (br_if $blocks_done (i32.lt_u (local.get $len) (i32.const 64)))
        (call $sha256_compress (local.get $ctx) (local.get $data))
        (local.set $data (i32.add (local.get $data) (i32.const 64)))
        (local.set $len (i32.sub (local.get $len) (i32.const 64)))
        (br $blocks)))
    (if (local.get $len)
      (then
        (drop (call $m26memcpy (i32.add (local.get $ctx) (i32.const 40)) (local.get $data) (local.get $len)))
        (i32.store (i32.add (local.get $ctx) (i32.const 104)) (local.get $len))))
    (local.get $ctx))

  (func $er_sha256_final (export "er_sha256_final") (param $ctx i32) (param $out i32) (result i32)
    (local $pos i32) (local $bits_lo i32) (local $bits_hi i32) (local $i i32)
    (if (i32.eqz (local.get $ctx)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $out)) (then (return (i32.const 0))))
    (local.set $pos (i32.load (i32.add (local.get $ctx) (i32.const 104))))
    (i32.store8 (i32.add (i32.add (local.get $ctx) (i32.const 40)) (local.get $pos)) (i32.const 128))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (if (i32.gt_u (local.get $pos) (i32.const 56))
      (then
        (call $m26memset (i32.add (i32.add (local.get $ctx) (i32.const 40)) (local.get $pos)) (i32.const 0) (i32.sub (i32.const 64) (local.get $pos)))
        (call $sha256_compress (local.get $ctx) (i32.add (local.get $ctx) (i32.const 40)))
        (call $m26memset (i32.add (local.get $ctx) (i32.const 40)) (i32.const 0) (i32.const 56)))
      (else
        (call $m26memset (i32.add (i32.add (local.get $ctx) (i32.const 40)) (local.get $pos)) (i32.const 0) (i32.sub (i32.const 56) (local.get $pos)))))
    (local.set $bits_lo (i32.shl (i32.load (i32.add (local.get $ctx) (i32.const 32))) (i32.const 3)))
    (local.set $bits_hi (i32.or (i32.shl (i32.load (i32.add (local.get $ctx) (i32.const 36))) (i32.const 3))
                                (i32.shr_u (i32.load (i32.add (local.get $ctx) (i32.const 32))) (i32.const 29))))
    (call $store_u32_be (i32.add (local.get $ctx) (i32.const 96)) (local.get $bits_hi))
    (call $store_u32_be (i32.add (local.get $ctx) (i32.const 100)) (local.get $bits_lo))
    (call $sha256_compress (local.get $ctx) (i32.add (local.get $ctx) (i32.const 40)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (i32.const 8)))
        (call $store_u32_be (i32.add (local.get $out) (i32.shl (local.get $i) (i32.const 2)))
          (i32.load (i32.add (local.get $ctx) (i32.shl (local.get $i) (i32.const 2)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $out))

  (func $er_tor_sha256 (export "er_tor_sha256") (param $data i32) (param $len i32) (param $out i32) (result i32)
    (if (i32.eqz (call $er_sha256_init (i32.const 4864))) (then (return (i32.const 0))))
    (if (i32.eqz (call $er_sha256_update (i32.const 4864) (local.get $data) (local.get $len))) (then (return (i32.const 0))))
    (if (i32.eqz (call $er_sha256_final (i32.const 4864) (local.get $out))) (then (return (i32.const 0))))
    (i32.const 32))

  (func $er_tor_hmac_sha256 (export "er_tor_hmac_sha256") (param $key i32) (param $key_len i32) (param $msg i32) (param $msg_len i32) (param $out i32) (result i32)
    (local $i i32) (local $b i32) (local $kptr i32) (local $klen i32)
    (if (i32.eqz (local.get $out)) (then (return (i32.const 0))))
    (local.set $kptr (local.get $key))
    (local.set $klen (local.get $key_len))
    (if (i32.gt_u (local.get $key_len) (i32.const 64))
      (then
        (if (i32.ne (call $er_tor_sha256 (local.get $key) (local.get $key_len) (i32.const 4672)) (i32.const 32)) (then (return (i32.const 0))))
        (local.set $kptr (i32.const 4672))
        (local.set $klen (i32.const 32))))
    (call $m26memset (i32.const 4608) (i32.const 0) (i32.const 64))
    (if (local.get $klen) (then (drop (call $m26memcpy (i32.const 4608) (local.get $kptr) (local.get $klen)))))
    (local.set $i (i32.const 0))
    (block $ipad_done
      (loop $ipad
        (br_if $ipad_done (i32.ge_u (local.get $i) (i32.const 64)))
        (local.set $b (i32.xor (i32.load8_u (i32.add (i32.const 4608) (local.get $i))) (i32.const 54)))
        (i32.store8 (i32.add (i32.const 4608) (local.get $i)) (local.get $b))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $ipad)))
    (drop (call $er_sha256_init (i32.const 4864)))
    (drop (call $er_sha256_update (i32.const 4864) (i32.const 4608) (i32.const 64)))
    (if (local.get $msg_len) (then (drop (call $er_sha256_update (i32.const 4864) (local.get $msg) (local.get $msg_len)))))
    (drop (call $er_sha256_final (i32.const 4864) (i32.const 4672)))
    (call $m26memset (i32.const 4608) (i32.const 0) (i32.const 64))
    (if (local.get $klen) (then (drop (call $m26memcpy (i32.const 4608) (local.get $kptr) (local.get $klen)))))
    (local.set $i (i32.const 0))
    (block $opad_done
      (loop $opad
        (br_if $opad_done (i32.ge_u (local.get $i) (i32.const 64)))
        (i32.store8 (i32.add (i32.const 4608) (local.get $i))
          (i32.xor (i32.load8_u (i32.add (i32.const 4608) (local.get $i))) (i32.const 92)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $opad)))
    (drop (call $er_sha256_init (i32.const 4864)))
    (drop (call $er_sha256_update (i32.const 4864) (i32.const 4608) (i32.const 64)))
    (drop (call $er_sha256_update (i32.const 4864) (i32.const 4672) (i32.const 32)))
    (drop (call $er_sha256_final (i32.const 4864) (local.get $out)))
    (i32.const 32))

  (func $m24xtime (param $x i32) (result i32)
    (i32.and
      (i32.xor (i32.shl (local.get $x) (i32.const 1))
               (i32.and (i32.const 27) (i32.sub (i32.const 0) (i32.shr_u (local.get $x) (i32.const 7)))))
      (i32.const 255)))

  (func $sub_byte (param $x i32) (result i32)
    (i32.load8_u (i32.add (i32.const 512) (i32.and (local.get $x) (i32.const 255)))))

  (func $m24aes128_key_expand (param $key i32) (param $rk i32)
    (local $bytes i32) (local $i i32) (local $t0 i32) (local $t1 i32) (local $t2 i32) (local $t3 i32)
    (drop (call $m26memcpy (local.get $rk) (local.get $key) (i32.const 16)))
    (local.set $bytes (i32.const 16))
    (local.set $i (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $bytes) (i32.const 176)))
        (local.set $t0 (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 4)))))
        (local.set $t1 (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 3)))))
        (local.set $t2 (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 2)))))
        (local.set $t3 (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 1)))))
        (local.set $t0 (i32.xor (call $sub_byte (local.get $t1)) (i32.load8_u (i32.add (i32.const 767) (local.get $i)))))
        (local.set $t1 (call $sub_byte (local.get $t2)))
        (local.set $t2 (call $sub_byte (local.get $t3)))
        (local.set $t3 (call $sub_byte (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 4))))))
        (i32.store8 (i32.add (local.get $rk) (local.get $bytes)) (i32.xor (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 16)))) (local.get $t0)))
        (i32.store8 (i32.add (local.get $rk) (i32.add (local.get $bytes) (i32.const 1))) (i32.xor (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 15)))) (local.get $t1)))
        (i32.store8 (i32.add (local.get $rk) (i32.add (local.get $bytes) (i32.const 2))) (i32.xor (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 14)))) (local.get $t2)))
        (i32.store8 (i32.add (local.get $rk) (i32.add (local.get $bytes) (i32.const 3))) (i32.xor (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 13)))) (local.get $t3)))
        (local.set $bytes (i32.add (local.get $bytes) (i32.const 4)))
        (block $word_done
          (loop $word
            (br_if $word_done (i32.eqz (i32.rem_u (local.get $bytes) (i32.const 16))))
            (local.set $t0 (i32.xor (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 16)))) (i32.load8_u (i32.add (local.get $rk) (i32.sub (local.get $bytes) (i32.const 4))))))
            (i32.store8 (i32.add (local.get $rk) (local.get $bytes)) (local.get $t0))
            (local.set $bytes (i32.add (local.get $bytes) (i32.const 1)))
            (br $word)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $m24add_round_key (param $state i32) (param $rk i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (i32.const 16)))
        (i32.store8 (i32.add (local.get $state) (local.get $i))
          (i32.xor (i32.load8_u (i32.add (local.get $state) (local.get $i)))
                   (i32.load8_u (i32.add (local.get $rk) (local.get $i)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $aes_shift_rows (param $s i32)
    (drop (call $m26memcpy (i32.const 5344) (local.get $s) (i32.const 16)))
    (i32.store8 (i32.add (local.get $s) (i32.const 1)) (i32.load8_u (i32.const 5349)))
    (i32.store8 (i32.add (local.get $s) (i32.const 5)) (i32.load8_u (i32.const 5353)))
    (i32.store8 (i32.add (local.get $s) (i32.const 9)) (i32.load8_u (i32.const 5357)))
    (i32.store8 (i32.add (local.get $s) (i32.const 13)) (i32.load8_u (i32.const 5345)))
    (i32.store8 (i32.add (local.get $s) (i32.const 2)) (i32.load8_u (i32.const 5354)))
    (i32.store8 (i32.add (local.get $s) (i32.const 6)) (i32.load8_u (i32.const 5358)))
    (i32.store8 (i32.add (local.get $s) (i32.const 10)) (i32.load8_u (i32.const 5346)))
    (i32.store8 (i32.add (local.get $s) (i32.const 14)) (i32.load8_u (i32.const 5350)))
    (i32.store8 (i32.add (local.get $s) (i32.const 3)) (i32.load8_u (i32.const 5359)))
    (i32.store8 (i32.add (local.get $s) (i32.const 7)) (i32.load8_u (i32.const 5347)))
    (i32.store8 (i32.add (local.get $s) (i32.const 11)) (i32.load8_u (i32.const 5351)))
    (i32.store8 (i32.add (local.get $s) (i32.const 15)) (i32.load8_u (i32.const 5355))))

  (func $aes_mix_columns (param $s i32)
    (local $col i32) (local $a i32) (local $b i32) (local $c i32) (local $d i32)
    (local $x i32) (local $y i32) (local $z i32) (local $w i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $col) (i32.const 16)))
        (local.set $a (i32.load8_u (i32.add (local.get $s) (local.get $col))))
        (local.set $b (i32.load8_u (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 1)))))
        (local.set $c (i32.load8_u (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 2)))))
        (local.set $d (i32.load8_u (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 3)))))
        (local.set $x (call $m24xtime (local.get $a)))
        (local.set $y (call $m24xtime (local.get $b)))
        (local.set $z (call $m24xtime (local.get $c)))
        (local.set $w (call $m24xtime (local.get $d)))
        (i32.store8 (i32.add (local.get $s) (local.get $col)) (i32.xor (i32.xor (i32.xor (local.get $x) (i32.xor (local.get $y) (local.get $b))) (local.get $c)) (local.get $d)))
        (i32.store8 (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 1))) (i32.xor (i32.xor (i32.xor (local.get $a) (local.get $y)) (i32.xor (local.get $z) (local.get $c))) (local.get $d)))
        (i32.store8 (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 2))) (i32.xor (i32.xor (i32.xor (local.get $a) (local.get $b)) (local.get $z)) (i32.xor (local.get $w) (local.get $d))))
        (i32.store8 (i32.add (local.get $s) (i32.add (local.get $col) (i32.const 3))) (i32.xor (i32.xor (i32.xor (i32.xor (local.get $x) (local.get $a)) (local.get $b)) (local.get $c)) (local.get $w)))
        (local.set $col (i32.add (local.get $col) (i32.const 4)))
        (br $loop))))

  (func $aes_encrypt_block (param $block i32) (param $rk i32)
    (local $round i32) (local $i i32)
    (call $m24add_round_key (local.get $block) (local.get $rk))
    (local.set $round (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.gt_u (local.get $round) (i32.const 10)))
        (local.set $i (i32.const 0))
        (block $sub_done
          (loop $sub
            (br_if $sub_done (i32.ge_u (local.get $i) (i32.const 16)))
            (i32.store8 (i32.add (local.get $block) (local.get $i)) (call $sub_byte (i32.load8_u (i32.add (local.get $block) (local.get $i)))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $sub)))
        (call $aes_shift_rows (local.get $block))
        (if (i32.lt_u (local.get $round) (i32.const 10)) (then (call $aes_mix_columns (local.get $block))))
        (call $m24add_round_key (local.get $block) (i32.add (local.get $rk) (i32.mul (local.get $round) (i32.const 16))))
        (local.set $round (i32.add (local.get $round) (i32.const 1)))
        (br $loop))))

  (func $ctr_inc_be_128 (param $ctr i32)
    (local $i i32) (local $v i32)
    (local.set $i (i32.const 15))
    (block $done
      (loop $loop
        (local.set $v (i32.add (i32.load8_u (i32.add (local.get $ctr) (local.get $i))) (i32.const 1)))
        (i32.store8 (i32.add (local.get $ctr) (local.get $i)) (local.get $v))
        (br_if $done (i32.le_u (local.get $v) (i32.const 255)))
        (br_if $done (i32.eqz (local.get $i)))
        (local.set $i (i32.sub (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $er_tor_aes128_ctr (export "er_tor_aes128_ctr") (param $out i32) (param $in i32) (param $len i32) (param $key i32) (param $iv i32) (result i32)
    (local $n i32) (local $i i32)
    (if (i32.eqz (local.get $out)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $in)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $key)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $iv)) (then (return (i32.const 0))))
    (call $m24aes128_key_expand (local.get $key) (i32.const 5120))
    (drop (call $m26memcpy (i32.const 5312) (local.get $iv) (i32.const 16)))
    (block $done
      (loop $blocks
        (br_if $done (i32.eqz (local.get $len)))
        (local.set $n (select (i32.const 16) (local.get $len) (i32.ge_u (local.get $len) (i32.const 16))))
        (drop (call $m26memcpy (i32.const 5328) (i32.const 5312) (i32.const 16)))
        (call $aes_encrypt_block (i32.const 5328) (i32.const 5120))
        (local.set $i (i32.const 0))
        (block $xor_done
          (loop $xor
            (br_if $xor_done (i32.ge_u (local.get $i) (local.get $n)))
            (i32.store8 (i32.add (local.get $out) (local.get $i))
              (i32.xor (i32.load8_u (i32.add (local.get $in) (local.get $i)))
                       (i32.load8_u (i32.add (i32.const 5328) (local.get $i)))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (br $xor)))
        (local.set $out (i32.add (local.get $out) (local.get $n)))
        (local.set $in (i32.add (local.get $in) (local.get $n)))
        (local.set $len (i32.sub (local.get $len) (local.get $n)))
        (call $ctr_inc_be_128 (i32.const 5312))
        (br $blocks)))
    (drop (call $m26memcpy (local.get $iv) (i32.const 5312) (i32.const 16)))
    (i32.const 1))

  (func $er_tor_aes_ctr (export "er_tor_aes_ctr") (param $out i32) (param $in i32) (param $len i32) (param $key i32) (param $iv i32) (result i32)
    (call $er_tor_aes128_ctr (local.get $out) (local.get $in) (local.get $len) (local.get $key) (local.get $iv)))
