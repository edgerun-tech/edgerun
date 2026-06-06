(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "proto_standard_id") (result i32)
    i32.const 300013)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid, 4 overflow, 5 truncated.
  (func $m184read_u16 (param $ptr i32) (result i32)
    (i32.or
      (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 8))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))))

  (func $m184read_u24 (param $ptr i32) (result i32)
    (i32.or
      (i32.or
        (i32.shl (i32.load8_u (local.get $ptr)) (i32.const 16))
        (i32.shl (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 8)))
      (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))))

  (func $store_span3 (param $out_ptr i32) (param $data_offset i32) (param $data_len i32) (param $next_offset i32)
    (i32.store (local.get $out_ptr) (local.get $data_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $data_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $next_offset)))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u8_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 1))
      (then (return (i32.const 1))))
    (local.set $data_len (i32.load8_u (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 1)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 1)
      (local.get $data_len)
      (i32.add (i32.const 1) (local.get $data_len)))
    (i32.const 0))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u16_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $data_len (call $m184read_u16 (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 2)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 2)
      (local.get $data_len)
      (i32.add (i32.const 2) (local.get $data_len)))
    (i32.const 0))

  ;; Output record: data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_vector_u24_decode") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $data_len i32)
    (if (i32.lt_u (local.get $len) (i32.const 3))
      (then (return (i32.const 1))))
    (local.set $data_len (call $m184read_u24 (local.get $ptr)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (i32.const 3)))
      (then (return (i32.const 5))))
    (call $store_span3
      (local.get $out_ptr)
      (i32.const 3)
      (local.get $data_len)
      (i32.add (i32.const 3) (local.get $data_len)))
    (i32.const 0))

  ;; Iterate a TLS ExtensionList payload.
  ;; Output record: extension_type:u32, data_offset:u32, data_len:u32, next_offset:u32.
  (func (export "tls_extension_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $ext_type i32)
    (local $data_len i32)
    (local $data_offset i32)
    (local $next_offset i32)
    (if (i32.gt_u (local.get $start) (local.get $len))
      (then (return (i32.const 3))))
    (if (i32.lt_u (i32.sub (local.get $len) (local.get $start)) (i32.const 4))
      (then (return (i32.const 1))))
    (local.set $ext_type (call $m184read_u16 (i32.add (local.get $ptr) (local.get $start))))
    (local.set $data_len (call $m184read_u16 (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))))
    (local.set $data_offset (i32.add (local.get $start) (i32.const 4)))
    (if (i32.gt_u (local.get $data_len) (i32.sub (local.get $len) (local.get $data_offset)))
      (then (return (i32.const 5))))
    (local.set $next_offset (i32.add (local.get $data_offset) (local.get $data_len)))
    (i32.store (local.get $out_ptr) (local.get $ext_type))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $data_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $data_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $next_offset))
    (i32.const 0))

  ;; Iterate an RFC 7301 ALPN extension payload.
  ;; start=0 validates the u16 protocol_name_list length and yields the first name.
  ;; For subsequent calls, pass the previous next_offset.
  ;; Output record: proto_offset:u32, proto_len:u32, next_offset:u32, list_len:u32.
  (func (export "tls_alpn_next") (param $ptr i32) (param $len i32) (param $start i32) (param $out_ptr i32) (result i32)
    (local $list_len i32)
    (local $pos i32)
    (local $proto_len i32)
    (local $next_offset i32)
    (if (i32.lt_u (local.get $len) (i32.const 2))
      (then (return (i32.const 1))))
    (local.set $list_len (call $m184read_u16 (local.get $ptr)))
    (if (i32.ne (i32.add (local.get $list_len) (i32.const 2)) (local.get $len))
      (then (return (i32.const 5))))
    (if (i32.eqz (local.get $list_len))
      (then (return (i32.const 3))))
    (if (i32.eqz (local.get $start))
      (then (local.set $pos (i32.const 2)))
      (else
        (local.set $pos (local.get $start))
        (if (i32.or (i32.lt_u (local.get $pos) (i32.const 2)) (i32.ge_u (local.get $pos) (local.get $len)))
          (then (return (i32.const 3))))))
    (local.set $proto_len (i32.load8_u (i32.add (local.get $ptr) (local.get $pos))))
    (if (i32.eqz (local.get $proto_len))
      (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $proto_len) (i32.sub (local.get $len) (i32.add (local.get $pos) (i32.const 1))))
      (then (return (i32.const 5))))
    (local.set $next_offset (i32.add (i32.add (local.get $pos) (i32.const 1)) (local.get $proto_len)))
    (i32.store (local.get $out_ptr) (i32.add (local.get $pos) (i32.const 1)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $proto_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $next_offset))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (local.get $list_len))
    (i32.const 0))
)
