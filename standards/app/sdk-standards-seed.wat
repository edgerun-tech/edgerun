
  ;; Status: 0 pass/found/ok, 1 not found, 2 output short or reject,
  ;; 3 invalid input. Packed i64: low u32 status, high u32 bytes_written.

  (data (i32.const 32768) "edgerun-sdk-standards-seed/v1\00")
  (data (i32.const 32800) "udp-datagram-definition")
  (data (i32.const 32832) "tftp-message-definition")
  (data (i32.const 32864) "udp-rfc768-length-0001")
  (data (i32.const 32900) "tftp-rfc1350-opcode-0001")
  (data (i32.const 32932) "tftp-rfc1350-ack-length-0001")
  (data (i32.const 32968) "tftp-rfc1350-data-length-0001")

  (func $m162is_lower (param $c i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $c) (i32.const 97))
      (i32.le_u (local.get $c) (i32.const 122))))

  (func $is_id_char (param $c i32) (result i32)
    (i32.or
      (i32.or (call $m162is_lower (local.get $c)) (call $is_digit (local.get $c)))
      (i32.eq (local.get $c) (i32.const 45))))

  (func $m162copy (param $src i32) (param $len i32) (param $dst i32)
    (local $i i32)
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8
          (i32.add (local.get $dst) (local.get $i))
          (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (import "edgerun" "string_eq" (func $eq_mem (param i32 i32 i32 i32) (result i32)))

  (func $unit_index (param $ptr i32) (param $len i32) (result i32)
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32800) (i32.const 23))
      (then (return (i32.const 0))))
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32832) (i32.const 23))
      (then (return (i32.const 1))))
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32864) (i32.const 22))
      (then (return (i32.const 2))))
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32900) (i32.const 24))
      (then (return (i32.const 3))))
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32932) (i32.const 28))
      (then (return (i32.const 4))))
    (if (call $eq_mem (local.get $ptr) (local.get $len) (i32.const 32968) (i32.const 29))
      (then (return (i32.const 5))))
    i32.const -1)

  (func $unit_kind (param $index i32) (result i32)
    (if (i32.le_u (local.get $index) (i32.const 1))
      (then (return (i32.const 0))))
    i32.const 1)

  (func $unit_standard_code (param $index i32) (result i32)
    (if
      (i32.or
        (i32.eq (local.get $index) (i32.const 0))
        (i32.eq (local.get $index) (i32.const 2)))
      (then (return (i32.const 1))))
    i32.const 2)

  (func $unit_wasm_export_code (param $index i32) (result i32)
    (if (i32.le_u (local.get $index) (i32.const 1))
      (then (return (i32.const 1))))
    i32.const 2)

  (func $unit_requirement_mask (param $index i32) (result i32)
    (if (i32.eq (local.get $index) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $index) (i32.const 3)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 4)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $index) (i32.const 5)) (then (return (i32.const 8))))
    i32.const 0)

  (func $sdk_seed_id_valid (export "sdk_seed_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $prev_hyphen i32)
    (if
      (i32.or
        (i32.eqz (local.get $len))
        (i32.gt_u (local.get $len) (i32.const 96)))
      (then (return (i32.const 3))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eqz (call $is_id_char (local.get $c)))
          (then (return (i32.const 3))))
        (if (i32.eq (local.get $c) (i32.const 45))
          (then
            (if
              (i32.or
                (i32.or
                  (i32.eqz (local.get $i))
                  (i32.eq (local.get $i) (i32.sub (local.get $len) (i32.const 1))))
                (local.get $prev_hyphen))
              (then (return (i32.const 3))))
            (local.set $prev_hyphen (i32.const 1)))
          (else
            (local.set $prev_hyphen (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    i32.const 0)

  (func $sdk_seed_namespace_valid (export "sdk_seed_namespace_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (local $prev_sep i32)
    (if
      (i32.or
        (i32.eqz (local.get $len))
        (i32.gt_u (local.get $len) (i32.const 128)))
      (then (return (i32.const 3))))
    (local.set $prev_sep (i32.const 1))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.or (i32.eq (local.get $c) (i32.const 46)) (i32.eq (local.get $c) (i32.const 58)))
          (then
            (if (local.get $prev_sep) (then (return (i32.const 3))))
            (local.set $prev_sep (i32.const 1)))
          (else
            (if
              (i32.eqz
                (i32.or
                  (i32.or (call $is_id_char (local.get $c)) (i32.eq (local.get $c) (i32.const 95)))
                  (i32.eq (local.get $c) (i32.const 64))))
              (then (return (i32.const 3))))
            (local.set $prev_sep (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (if (local.get $prev_sep) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "sdk_seed_unit_lookup") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "sdk_seed_unit_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const -2))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const -1))))
    (call $unit_kind (local.get $index)))

  (func (export "sdk_seed_graph_member") (param $ptr i32) (param $len i32) (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $ptr) (local.get $len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $ptr) (local.get $len)))
    (if
      (i32.or
        (i32.eq (local.get $index) (i32.const 1))
        (i32.or
          (i32.eq (local.get $index) (i32.const 3))
          (i32.or
            (i32.eq (local.get $index) (i32.const 4))
            (i32.eq (local.get $index) (i32.const 5)))))
      (then (return (i32.const 0))))
    i32.const 1)

  (func (export "sdk_seed_clause_table_status")
    (param $id_ptr i32) (param $id_len i32)
    (param $byte_len i32) (param $opcode i32)
    (param $udp_length i32) (param $udp_payload_len i32)
    (result i32)
    (local $index i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $id_ptr) (local.get $id_len)) (i32.const 0))
      (then (return (i32.const 3))))
    (local.set $index (call $unit_index (local.get $id_ptr) (local.get $id_len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $index) (i32.const 2))
      (then
        (if
          (i32.and
            (i32.ge_u (local.get $udp_length) (i32.const 8))
            (i32.eq (local.get $udp_length) (i32.add (local.get $udp_payload_len) (i32.const 8))))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 3))
      (then
        (if
          (i32.and
            (i32.ge_u (local.get $byte_len) (i32.const 2))
            (i32.and
              (i32.ge_u (local.get $opcode) (i32.const 1))
              (i32.le_u (local.get $opcode) (i32.const 6))))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 4))
      (then
        (if
          (i32.or
            (i32.ne (local.get $opcode) (i32.const 4))
            (i32.eq (local.get $byte_len) (i32.const 4)))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    (if (i32.eq (local.get $index) (i32.const 5))
      (then
        (if
          (i32.or
            (i32.ne (local.get $opcode) (i32.const 3))
            (i32.ge_u (local.get $byte_len) (i32.const 4)))
          (then (return (i32.const 0))))
        (return (i32.const 2))))
    i32.const 1)

  (func $sdk_seed_unit_preimage (export "sdk_seed_unit_preimage")
    (param $id_ptr i32) (param $id_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $index i32)
    (local $need i32)
    (if (i32.ne (call $sdk_seed_id_valid (local.get $id_ptr) (local.get $id_len)) (i32.const 0))
      (then (return (call $pack (i32.const 3) (i32.const 0)))))
    (local.set $index (call $unit_index (local.get $id_ptr) (local.get $id_len)))
    (if (i32.lt_s (local.get $index) (i32.const 0))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $need (i32.add (i32.const 39) (local.get $id_len)))
    (if (i32.lt_u (local.get $out_cap) (local.get $need))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (call $m162copy (i32.const 32768) (i32.const 30) (local.get $out_ptr))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 30)) (local.get $index))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 31)) (call $unit_kind (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 32)) (call $unit_standard_code (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 33)) (call $unit_wasm_export_code (local.get $index)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 34)) (call $unit_requirement_mask (local.get $index)))
    (i32.store8 (i32.add (local.get $out_ptr) (i32.const 38)) (local.get $id_len))
    (call $m162copy (local.get $id_ptr) (local.get $id_len) (i32.add (local.get $out_ptr) (i32.const 39)))
    (call $pack (i32.const 0) (local.get $need)))

  (func $sdk_seed_shape32 (export "sdk_seed_shape32")
    (param $in_ptr i32) (param $in_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $i i32)
    (local $b i32)
    (local $s0 i32)
    (local $s1 i32)
    (local $s2 i32)
    (local $s3 i32)
    (if (i32.lt_u (local.get $out_cap) (i32.const 32))
      (then (return (call $pack (i32.const 2) (i32.const 0)))))
    (local.set $s0 (i32.const 0x811c9dc5))
    (local.set $s1 (i32.const 0x9e3779b9))
    (local.set $s2 (i32.const 0x85ebca6b))
    (local.set $s3 (i32.const 0xc2b2ae35))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $in_len)))
        (local.set $b (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
        (local.set $s0
          (i32.add
            (i32.mul (i32.rotl (i32.xor (local.get $s0) (local.get $b)) (i32.const 5)) (i32.const 16777619))
            (local.get $i)))
        (local.set $s1
          (i32.add
            (i32.rotl (i32.add (local.get $s1) (local.get $b)) (i32.const 7))
            (i32.const 0x7f4a7c15)))
        (local.set $s2
          (i32.xor
            (i32.rotl (local.get $s2) (i32.const 11))
            (i32.add (i32.shl (local.get $b) (i32.const 16)) (local.get $i))))
        (local.set $s3
          (i32.add
            (i32.xor (local.get $s3) (i32.mul (local.get $b) (i32.const 0x45d9f3b)))
            (i32.rotl (local.get $s0) (i32.const 13))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.store (local.get $out_ptr) (local.get $s0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $s1))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $s2))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $s3))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.xor (local.get $s0) (local.get $s2)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.xor (local.get $s1) (local.get $s3)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 24))
      (i32.add (i32.rotl (local.get $s0) (i32.const 17)) (local.get $s3)))
    (i32.store
      (i32.add (local.get $out_ptr) (i32.const 28))
      (i32.xor (i32.rotl (local.get $s1) (i32.const 3)) (local.get $s2)))
    (call $pack (i32.const 0) (i32.const 32)))

  (func (export "sdk_seed_unit_shape32")
    (param $id_ptr i32) (param $id_len i32)
    (param $out_ptr i32) (param $out_cap i32)
    (result i64)
    (local $r i64)
    (local $status i32)
    (local $written i32)
    (local.set $r
      (call $sdk_seed_unit_preimage
        (local.get $id_ptr)
        (local.get $id_len)
        (i32.const 8192)
        (i32.const 512)))
    (local.set $status (i32.wrap_i64 (local.get $r)))
    (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $r) (i64.const 32))))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (call $pack (local.get $status) (i32.const 0)))))
    (call $sdk_seed_shape32
      (i32.const 8192)
      (local.get $written)
      (local.get $out_ptr)
      (local.get $out_cap)))
