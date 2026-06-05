(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300039)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  (func $read_u16 (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  ;; Walk a TLS ExtensionList body: repeated type:u16, len:u16, payload bytes.
  ;;
  ;; Output record:
  ;; extension_count:u32,
  ;; first_sni_data_offset:u32, first_sni_data_len:u32,
  ;; alpn_present:u32,
  ;; final_offset:u32.
  ;;
  ;; Offsets are relative to ptr. first_sni_data_offset/len are zero when SNI is absent.
  (func (export "tls_extension_walk") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $pos i32)
    (local $remaining i32)
    (local $ext_type i32)
    (local $data_len i32)
    (local $data_offset i32)
    (local $next_offset i32)
    (local $count i32)
    (local $sni_offset i32)
    (local $sni_len i32)
    (local $alpn_present i32)

    (loop $scan
      (if (i32.eq (local.get $pos) (local.get $len))
        (then
          (i32.store (local.get $out_ptr) (local.get $count))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $sni_offset))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $sni_len))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $alpn_present))
          (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (local.get $pos))
          (return (i32.const 0))))

      (local.set $remaining (i32.sub (local.get $len) (local.get $pos)))
      (if (i32.lt_u (local.get $remaining) (i32.const 4))
        (then (return (i32.const 5))))

      (local.set $ext_type (call $read_u16 (i32.add (local.get $ptr) (local.get $pos))))
      (local.set $data_len (call $read_u16 (i32.add (i32.add (local.get $ptr) (local.get $pos)) (i32.const 2))))
      (local.set $data_offset (i32.add (local.get $pos) (i32.const 4)))
      (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (local.get $data_offset)))
        (then (return (i32.const 5))))

      (local.set $next_offset (i32.add (local.get $data_offset) (local.get $data_len)))
      (local.set $count (i32.add (local.get $count) (i32.const 1)))

      ;; server_name extension type 0. Keep only the first SNI payload span.
      (if (i32.and
            (i32.eq (local.get $ext_type) (i32.const 0))
            (i32.eqz (local.get $sni_offset)))
        (then
          (local.set $sni_offset (local.get $data_offset))
          (local.set $sni_len (local.get $data_len))))

      ;; application_layer_protocol_negotiation extension type 16.
      (if (i32.eq (local.get $ext_type) (i32.const 16))
        (then (local.set $alpn_present (i32.const 1))))

      (local.set $pos (local.get $next_offset))
      br $scan)
    (i32.const 3))
)
