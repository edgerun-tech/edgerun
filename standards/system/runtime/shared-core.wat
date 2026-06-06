;; Shared runtime core for edgerun unified system.
;; Provides character classification LUTs, SIMD stubs, and common helpers.
;; All functions use unfolded (flat) WAT style.

(memory (export "memory") 256)

;; Character classification LUT at 0x1000 (256 bytes)
;; bit 0: digit, bit 1: uppercase, bit 2: lowercase, bit 3: tchar,
;; bit 4: hex, bit 5: ws, bit 6: scheme, bit 7: dns-label
${lut_class}

;; Lowercase mapping LUT at 0x2000 (256 bytes)
${lut_lower}

;; Character classification helpers

(func $char_class (param $b i32) (result i32)
  (i32.load8_u offset=0x1000 (local.get $b)))

(func $is_digit (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 1)))

(func $is_upper (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 2)))

(func $is_lower (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 4)))

(func $is_alpha (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 6)))

(func $is_tchar (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 8)))

(func $is_hex (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 16)))

(func $is_ws (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 32)))

(func $is_scheme_byte (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 64)))

(func $is_label_byte (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 128)))

(func $to_lower (param $b i32) (result i32)
  (local $cl i32)
  (local.set $cl (call $char_class (local.get $b)))
  (if (i32.and (local.get $cl) (i32.const 2))
    (then (return (i32.or (local.get $b) (i32.const 32)))))
  (if (i32.and (local.get $cl) (i32.const 4))
    (then (return (local.get $b))))
  (i32.const 0))

;; Pack/unpack helpers

(func $pack (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
  (i32.or
    (i32.or
      (i32.or
        (local.get $d)
        (i32.shl (local.get $c) (i32.const 8)))
      (i32.shl (local.get $b) (i32.const 16)))
    (i32.shl (local.get $a) (i32.const 24))))

(func $pack_u16 (param $a i32) (param $b i32) (result i32)
  (i32.or
    (local.get $b)
    (i32.shl (local.get $a) (i32.const 8))))

(func $byte (param $v i32) (param $i i32) (result i32)
  (i32.and
    (i32.shr_u (local.get $v) (i32.shl (local.get $i) (i32.const 3)))
    (i32.const 0xFF)))

(func $has (param $v i32) (param $mask i32) (result i32)
  (i32.ne (i32.and (local.get $v) (local.get $mask)) (i32.const 0)))

(func $is_cont (param $b i32) (result i32)
  (i32.eq (i32.and (local.get $b) (i32.const 0xC0)) (i32.const 0x80)))

;; Emit helpers (for compiler code emission)

(func $emit_byte (param $addr i32) (param $b i32)
  (i32.store8 (local.get $addr) (local.get $b)))

(func $emit_dword (param $addr i32) (param $v i32)
  (i32.store (local.get $addr) (local.get $v)))

(func $emit_modrm (param $addr i32) (param $mod i32) (param $reg i32) (param $rm i32)
  (i32.store8 (local.get $addr)
    (i32.or
      (i32.or
        (i32.shl (local.get $mod) (i32.const 6))
        (i32.shl (local.get $reg) (i32.const 3)))
      (local.get $rm))))

;; SIMD stubs

(func $simd_memchr (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
  (local $i i32) (local $vec v128) (local $cmp v128) (local $mask i32)
  (local.set $i (local.get $ptr))
  (block $done
    (loop $loop
      (br_if $done
        (i32.lt_u (local.get $len) (i32.const 16)))
      (local.set $vec (v128.load (local.get $i)))
      (local.set $cmp
        (i8x16.eq (local.get $vec)
          (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
      (local.set $mask (i32x4.bitmask (local.get $cmp)))
      (if (local.get $mask)
        (then
          (return (i32.add (local.get $i)
            (i32.ctz (local.get $mask))))))
      (local.set $i (i32.add (local.get $i) (i32.const 16)))
      (local.set $len (i32.sub (local.get $len) (i32.const 16)))
      (br $loop)))
  (block $r_done
    (loop $r_loop
      (br_if $r_done
        (i32.eqz (local.get $len)))
      (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
        (then (return (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (local.set $len (i32.sub (local.get $len) (i32.const 1)))
      (br $r_loop)))
  (i32.const -1))

(func $simd_memrchr (param $ptr i32) (param $len i32) (param $byte i32) (result i32)
  (local $i i32) (local $end i32) (local $vec v128) (local $cmp v128) (local $mask i32)
  (local.set $end (i32.add (local.get $ptr) (local.get $len)))
  (local.set $i (i32.sub (local.get $end) (i32.const 16)))
  (block $done
    (loop $loop
      (br_if $done
        (i32.lt_u (local.get $i) (local.get $ptr)))
      (local.set $vec (v128.load (local.get $i)))
      (local.set $cmp
        (i8x16.eq (local.get $vec)
          (i8x16.splat (i32.wrap_i64 (i64.extend_i32_u (local.get $byte))))))
      (local.set $mask (i32x4.bitmask (local.get $cmp)))
      (if (local.get $mask)
        (then
          (return (i32.add (local.get $i)
            (i32.ctz (local.get $mask))))))
      (local.set $i (i32.sub (local.get $i) (i32.const 16)))
      (br $loop)))
  (local.set $i (i32.sub (local.get $end) (i32.const 1)))
  (block $r_done
    (loop $r_loop
      (br_if $r_done
        (i32.lt_u (local.get $i) (local.get $ptr)))
      (if (i32.eq (i32.load8_u (local.get $i)) (local.get $byte))
        (then (return (local.get $i))))
      (local.set $i (i32.sub (local.get $i) (i32.const 1)))
      (br $r_loop)))
  (i32.const -1))

(func $is_alnum (param $b i32) (result i32)
  (i32.and (call $char_class (local.get $b)) (i32.const 7)))

(func $is_print (param $b i32) (result i32)
  (i32.and
    (i32.ge_u (local.get $b) (i32.const 32))
    (i32.le_u (local.get $b) (i32.const 126))))

(func $to_upper (param $b i32) (result i32)
  (if (i32.and (call $char_class (local.get $b)) (i32.const 4))
    (then (return (i32.sub (local.get $b) (i32.const 32)))))
  (local.get $b))

;; System call stubs (replace with host imports in production)

(func $sock_open (param $sock_type i32) (param $cfg i32) (param $cfg_len i32) (result i32)
  (i32.const -1))

(func $sock_send (param $fd i32) (param $buf i32) (param $len i32) (result i32)
  (i32.const -1))

(func $sock_recv (param $fd i32) (param $buf i32) (param $max_len i32) (result i32)
  (i32.const -1))

(func $sock_close (param $fd i32) (result i32)
  (i32.const 0))

(func $host_relay_digest_update20_sha1 (param $slot i32) (param $payload i32) (param $payload_len i32) (param $out i32) (result i32)
  (i32.const -1))

(func $host_aes_ctr_crypt (param $key i32) (param $iv i32) (param $in i32) (param $len i32) (param $out i32) (result i32)
  (i32.const -1))

(func $host_ntor_server_handshake_seeded (param $handshake i32) (param $node_id i32) (param $pub i32) (param $sec i32) (param $y i32) (param $out i32) (param $out_end i32) (result i32)
  (i32.const -1))

(func $host_ntor_v3_server_handshake_seeded (param $handshake i32) (param $hs_len i32) (param $node_id i32) (param $pub i32) (param $sec i32) (param $y i32) (param $arg7 i32) (param $arg8 i32) (param $server_msg i32) (param $server_msg_len i32) (result i32)
  (i32.const -1))

(func $host_relay_connect (param $addr i32) (param $port i32) (param $key i32) (param $flags i32) (result i32)
  (i32.const -1))

(func $host_send_cell (param $circuit i32) (param $cell i32) (param $len i32) (result i32)
  (i32.const -1))

;; ABI exports

(func (export "proto_abi_version") (result i32)
  (i32.const 2))

(func (export "proto_standard_id") (result i32)
  (i32.const 0))

(func (export "simd_capabilities") (result i32)
  (i32.const 1))
