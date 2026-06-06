;; Hidden-service introduction-point relay surface.
  ;; Registers ESTABLISH_INTRO auth keys and forwards exact INTRODUCE1 bodies as
  ;; INTRODUCE2 payloads when the key matches an active introduction circuit.
  (global $m16STANDARD_ID i32 (i32.const 300217))
  (global $m16ABI_VERSION i32 (i32.const 2))
  (global $m16OK i32 (i32.const 0))
  (global $m16NOT_FOUND i32 (i32.const 1))
  (global $m16ERR_INVALID i32 (i32.const -1))
  (global $m16ERR_BOUNDS i32 (i32.const -2))
  (global $ERR_AUTH i32 (i32.const -3))

  (global $MAX_INTROS i32 (i32.const 16))
  (global $REC_SIZE i32 (i32.const 96))
  (global $REC_BASE i32 (i32.const 4096))
  (global $AUTH_KEY_LEN i32 (i32.const 32))
  (global $INTRODUCE1_MAX i32 (i32.const 490))
  (global $ESTABLISH_MIN i32 (i32.const 134))
  (global $INTRO_PREFIX_MIN i32 (i32.const 56))
  (global $m16RELAY_INTRODUCE2 i32 (i32.const 35))
  (global $m16RELAY_INTRODUCE_ACK i32 (i32.const 40))

  ;; Record layout:
  ;; 0 active, 4 intro_circ_id, 8 service_circ_id, 12 intro_count,
  ;; 16 auth_key32, 48 last_body_len, 52 last_status, 56 reserved.

  (func $rec_ptr (param $index i32) (result i32)
    (i32.add (global.get $REC_BASE) (i32.mul (local.get $index) (global.get $REC_SIZE))))

  (func $m16u16be (param $p i32) (result i32)
    (i32.or (i32.shl (i32.load8_u (local.get $p)) (i32.const 8)) (i32.load8_u (i32.add (local.get $p) (i32.const 1)))))

  (func $m16put_u16be (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (local.get $v)))

  (func $m16mem_eq (param $a i32) (param $b i32) (param $len i32) (result i32)
    (local $i i32) (local $acc i32)
    (local.set $i (i32.const 0))
    (local.set $acc (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $acc
          (i32.or (local.get $acc)
            (i32.xor
              (i32.load8_u (i32.add (local.get $a) (local.get $i)))
              (i32.load8_u (i32.add (local.get $b) (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.eqz (local.get $acc)))

  (func $m16copy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $extensions_end (param $p i32) (param $len i32) (param $ext_off i32) (result i32)
    (local $pos i32) (local $n i32) (local $l i32)
    (if (i32.gt_u (local.get $ext_off) (local.get $len)) (then (return (i32.const -1))))
    (local.set $pos (local.get $ext_off))
    (local.set $n (i32.load8_u (i32.add (local.get $p) (local.get $pos))))
    (local.set $pos (i32.add (local.get $pos) (i32.const 1)))
    (block $done
      (loop $loop
        (br_if $done (i32.eqz (local.get $n)))
        (if (i32.gt_u (i32.add (local.get $pos) (i32.const 2)) (local.get $len)) (then (return (i32.const -1))))
        (local.set $l (i32.load8_u (i32.add (i32.add (local.get $p) (local.get $pos)) (i32.const 1))))
        (local.set $pos (i32.add (local.get $pos) (i32.const 2)))
        (if (i32.gt_u (i32.add (local.get $pos) (local.get $l)) (local.get $len)) (then (return (i32.const -1))))
        (local.set $pos (i32.add (local.get $pos) (local.get $l)))
        (local.set $n (i32.sub (local.get $n) (i32.const 1)))
        (br $loop)))
    (local.get $pos))

  (func $find_by_intro_circ (param $circ_id i32) (result i32)
    (local $i i32) (local $r i32)
    (local.set $i (i32.const 0))
    (block $nf
      (loop $loop
        (br_if $nf (i32.ge_u (local.get $i) (global.get $MAX_INTROS)))
        (local.set $r (call $rec_ptr (local.get $i)))
        (if (i32.and (i32.load (local.get $r)) (i32.eq (i32.load offset=4 (local.get $r)) (local.get $circ_id)))
          (then (return (local.get $r))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 0))

  (func $find_by_auth (param $auth_key i32) (result i32)
    (local $i i32) (local $r i32)
    (local.set $i (i32.const 0))
    (block $nf
      (loop $loop
        (br_if $nf (i32.ge_u (local.get $i) (global.get $MAX_INTROS)))
        (local.set $r (call $rec_ptr (local.get $i)))
        (if (i32.and (i32.load (local.get $r)) (call $m16mem_eq (i32.add (local.get $r) (i32.const 16)) (local.get $auth_key) (global.get $AUTH_KEY_LEN)))
          (then (return (local.get $r))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 0))

  (func $alloc_record (result i32)
    (local $i i32) (local $r i32)
    (local.set $i (i32.const 0))
    (block $nf
      (loop $loop
        (br_if $nf (i32.ge_u (local.get $i) (global.get $MAX_INTROS)))
        (local.set $r (call $rec_ptr (local.get $i)))
        (if (i32.eqz (i32.load (local.get $r))) (then (return (local.get $r))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 0))

  (func (export "proto_standard_id") (result i32) (global.get $m16STANDARD_ID))
  (func (export "proto_abi_version") (result i32) (global.get $m16ABI_VERSION))
  (func (export "simd_capabilities") (result i32) (i32.const 1))
  (func (export "tor_hs_intro_relay_max_body_len") (result i32) (global.get $INTRODUCE1_MAX))
  (func (export "tor_hs_intro_relay_record_size") (result i32) (global.get $REC_SIZE))

  (func (export "tor_hs_intro_relay_init") (result i32)
    (local $i i32) (local $r i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (global.get $MAX_INTROS)))
        (local.set $r (call $rec_ptr (local.get $i)))
        (i32.store (local.get $r) (i32.const 0))
        (i32.store offset=4 (local.get $r) (i32.const 0))
        (i32.store offset=8 (local.get $r) (i32.const 0))
        (i32.store offset=12 (local.get $r) (i32.const 0))
        (i32.store offset=48 (local.get $r) (i32.const 0))
        (i32.store offset=52 (local.get $r) (i32.const 0))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (global.get $m16OK))

  (func $parse_establish_intro_auth (export "tor_hs_intro_relay_parse_establish_intro_auth")
    (param $body i32) (param $body_len i32) (param $out_auth32 i32) (result i32)
    (local $ext_end i32) (local $sig_len_off i32)
    (if (i32.lt_u (local.get $body_len) (global.get $ESTABLISH_MIN)) (then (return (global.get $m16ERR_BOUNDS))))
    (if (i32.or (i32.ne (i32.load8_u (local.get $body)) (i32.const 2)) (i32.ne (call $m16u16be (i32.add (local.get $body) (i32.const 1))) (i32.const 32)))
      (then (return (global.get $m16ERR_INVALID))))
    (local.set $ext_end (call $extensions_end (local.get $body) (local.get $body_len) (i32.const 35)))
    (if (i32.lt_s (local.get $ext_end) (i32.const 0)) (then (return (global.get $m16ERR_INVALID))))
    (local.set $sig_len_off (i32.add (local.get $ext_end) (i32.const 32)))
    (if (i32.gt_u (i32.add (local.get $sig_len_off) (i32.const 2)) (local.get $body_len)) (then (return (global.get $m16ERR_INVALID))))
    (if (i32.ne (i32.add (i32.add (local.get $sig_len_off) (i32.const 2)) (call $m16u16be (i32.add (local.get $body) (local.get $sig_len_off)))) (local.get $body_len))
      (then (return (global.get $m16ERR_INVALID))))
    (call $m16copy (local.get $out_auth32) (i32.add (local.get $body) (i32.const 3)) (i32.const 32))
    (global.get $m16OK))

  (func (export "tor_hs_intro_relay_register")
    (param $intro_circ_id i32) (param $service_circ_id i32) (param $establish_body i32) (param $body_len i32) (result i32)
    (local $r i32)
    (if (i32.eqz (local.get $intro_circ_id)) (then (return (global.get $m16ERR_INVALID))))
    (local.set $r (call $find_by_intro_circ (local.get $intro_circ_id)))
    (if (i32.eqz (local.get $r)) (then (local.set $r (call $alloc_record))))
    (if (i32.eqz (local.get $r)) (then (return (global.get $m16ERR_BOUNDS))))
    (if (i32.ne (call $parse_establish_intro_auth (local.get $establish_body) (local.get $body_len) (i32.add (local.get $r) (i32.const 16))) (global.get $m16OK))
      (then (return (global.get $m16ERR_INVALID))))
    (i32.store (local.get $r) (i32.const 1))
    (i32.store offset=4 (local.get $r) (local.get $intro_circ_id))
    (i32.store offset=8 (local.get $r) (local.get $service_circ_id))
    (i32.store offset=48 (local.get $r) (local.get $body_len))
    (i32.store offset=52 (local.get $r) (global.get $m16OK))
    (global.get $m16OK))

  (func (export "tor_hs_intro_relay_validate_introduce1")
    (param $body i32) (param $body_len i32) (param $out_service_circ i32) (result i32)
    (local $i i32) (local $ext_end i32) (local $r i32)
    (if (i32.or (i32.lt_u (local.get $body_len) (global.get $INTRO_PREFIX_MIN)) (i32.gt_u (local.get $body_len) (global.get $INTRODUCE1_MAX)))
      (then (return (global.get $m16ERR_BOUNDS))))
    (local.set $i (i32.const 0))
    (block $zeros_done
      (loop $zeros
        (br_if $zeros_done (i32.ge_u (local.get $i) (i32.const 20)))
        (if (i32.ne (i32.load8_u (i32.add (local.get $body) (local.get $i))) (i32.const 0)) (then (return (global.get $m16ERR_INVALID))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $zeros)))
    (if (i32.or (i32.ne (i32.load8_u (i32.add (local.get $body) (i32.const 20))) (i32.const 2)) (i32.ne (call $m16u16be (i32.add (local.get $body) (i32.const 21))) (i32.const 32)))
      (then (return (global.get $m16ERR_INVALID))))
    (local.set $ext_end (call $extensions_end (local.get $body) (local.get $body_len) (i32.const 55)))
    (if (i32.lt_s (local.get $ext_end) (i32.const 0)) (then (return (global.get $m16ERR_INVALID))))
    (if (i32.ge_u (local.get $ext_end) (local.get $body_len)) (then (return (global.get $m16ERR_INVALID))))
    (local.set $r (call $find_by_auth (i32.add (local.get $body) (i32.const 23))))
    (if (i32.eqz (local.get $r)) (then (return (global.get $ERR_AUTH))))
    (i32.store (local.get $out_service_circ) (i32.load offset=8 (local.get $r)))
    (i32.store offset=12 (local.get $r) (i32.add (i32.load offset=12 (local.get $r)) (i32.const 1)))
    (i32.store offset=48 (local.get $r) (local.get $body_len))
    (i32.store offset=52 (local.get $r) (global.get $m16OK))
    (global.get $m16OK))

  (func (export "tor_hs_intro_relay_build_introduce2")
    (param $out i32) (param $introduce1_body i32) (param $body_len i32) (result i32)
    (if (i32.gt_u (local.get $body_len) (global.get $INTRODUCE1_MAX)) (then (return (global.get $m16ERR_BOUNDS))))
    (call $m16copy (local.get $out) (local.get $introduce1_body) (local.get $body_len))
    (local.get $body_len))

  (func (export "tor_hs_intro_relay_build_introduce_ack")
    (param $out i32) (param $status i32) (result i32)
    (call $m16put_u16be (local.get $out) (local.get $status))
    (i32.store8 (i32.add (local.get $out) (i32.const 2)) (i32.const 0))
    (i32.const 3))

  (func (export "tor_hs_intro_relay_build_relay_header")
    (param $out i32) (param $cmd i32) (param $stream i32) (param $body_len i32) (result i32)
    (i32.store8 (local.get $out) (local.get $cmd))
    (call $m16put_u16be (i32.add (local.get $out) (i32.const 1)) (local.get $stream))
    (i32.store (i32.add (local.get $out) (i32.const 3)) (i32.const 0))
    (call $m16put_u16be (i32.add (local.get $out) (i32.const 9)) (local.get $body_len))
    (i32.const 11))

  (func (export "tor_hs_intro_relay_record_ptr") (param $intro_circ_id i32) (result i32)
    (call $find_by_intro_circ (local.get $intro_circ_id)))