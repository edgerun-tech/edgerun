(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300105)

  (func $ascii_lower (param $c i32) (result i32)
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 65))
        (i32.le_u (local.get $c) (i32.const 90)))
      (then (return (i32.add (local.get $c) (i32.const 32)))))
    local.get $c)

  (func $fnv_lower (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $h i32)
    (local.set $h (i32.const 0x811c9dc5))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $h
          (i32.mul
            (i32.xor
              (local.get $h)
              (call $ascii_lower (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
            (i32.const 0x01000193)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $h)

  (func (export "node_hw_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $fnv_lower (local.get $ptr) (local.get $len)))

  (func $hex_nibble (param $c i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (then (return (i32.sub (local.get $c) (i32.const 48)))))
    (local.set $c (call $ascii_lower (local.get $c)))
    (if (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 102)))
      (then (return (i32.add (i32.sub (local.get $c) (i32.const 97)) (i32.const 10)))))
    i32.const -1)

  (func (export "node_hw_pci_address_valid") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.ne (local.get $len) (i32.const 12)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 58)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 58)) (then (return (i32.const 0))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 10))) (i32.const 46)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (local.get $ptr))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 1)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 2)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 3)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 5)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 6)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 8)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 9)))) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.lt_s (call $hex_nibble (i32.load8_u (i32.add (local.get $ptr) (i32.const 11)))) (i32.const 0)) (then (return (i32.const 0))))
    i32.const 1)

  (func $node_hw_gpu_class (export "node_hw_gpu_class") (param $class_code i32) (result i32)
    (local $base i32)
    (local.set $base (i32.shr_u (local.get $class_code) (i32.const 16)))
    (if (i32.eq (local.get $base) (i32.const 0x03)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $base) (i32.const 0x12)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "node_hw_gpu_vendor") (param $vendor_id i32) (result i32)
    (if (i32.eq (local.get $vendor_id) (i32.const 0x1002)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $vendor_id) (i32.const 0x8086)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $vendor_id) (i32.const 0x10de)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $vendor_id) (i32.const 0x1af4)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $vendor_id) (i32.const 0x13b5)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $vendor_id) (i32.const 0x5143)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "node_hw_gpu_capabilities")
    (param $vendor i32)
    (param $class_code i32)
    (param $drm_cards i32)
    (param $render_nodes i32)
    (param $connectors i32)
    (result i32)
    (local $caps i32)
    (if (i32.or (i32.gt_u (local.get $drm_cards) (i32.const 0)) (i32.gt_u (local.get $render_nodes) (i32.const 0)))
      (then (local.set $caps (i32.or (local.get $caps) (i32.const 1)))))
    (if
      (i32.or
        (i32.eq (call $node_hw_gpu_class (local.get $class_code)) (i32.const 1))
        (i32.or (i32.gt_u (local.get $drm_cards) (i32.const 0)) (i32.gt_u (local.get $connectors) (i32.const 0))))
      (then (local.set $caps (i32.or (local.get $caps) (i32.const 2)))))
    (if
      (i32.or
        (i32.gt_u (local.get $render_nodes) (i32.const 0))
        (i32.and (i32.ge_u (local.get $vendor) (i32.const 1)) (i32.le_u (local.get $vendor) (i32.const 6))))
      (then (local.set $caps (i32.or (local.get $caps) (i32.const 4)))))
    local.get $caps)

  (func (export "node_hw_connector_status") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1424938192)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 908767658)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 49525662)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $h) (i32.const 871591685)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "node_hw_display_mode_parse")
    (param $ptr i32)
    (param $len i32)
    (param $out_ptr i32)
    (result i32)
    (local $i i32)
    (local $w i32)
    (local $h i32)
    (local $refresh i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $x
      (loop $wloop
        (br_if $x (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eq (local.get $c) (i32.const 120)) (then (br $x)))
        (if (i32.eqz (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))) (then (return (i32.const 0))))
        (local.set $w (i32.add (i32.mul (local.get $w) (i32.const 10)) (i32.sub (local.get $c) (i32.const 48))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $wloop)))
    (if (i32.eqz (local.get $w)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 0))))
    (local.set $i (i32.add (local.get $i) (i32.const 1)))
    (block $tail
      (loop $hloop
        (br_if $tail (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.or (i32.eq (local.get $c) (i32.const 64)) (i32.or (i32.eq (local.get $c) (i32.const 105)) (i32.eq (local.get $c) (i32.const 112)))) (then (br $tail)))
        (if (i32.eqz (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))) (then (return (i32.const 0))))
        (local.set $h (i32.add (i32.mul (local.get $h) (i32.const 10)) (i32.sub (local.get $c) (i32.const 48))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $hloop)))
    (if (i32.eqz (local.get $h)) (then (return (i32.const 0))))
    (local.set $refresh (i32.const 60000))
    (if (i32.lt_u (local.get $i) (local.get $len))
      (then
        (if (i32.or (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 105)) (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 112)))
          (then (local.set $i (i32.add (local.get $i) (i32.const 1)))))
        (if (i32.lt_u (local.get $i) (local.get $len))
          (then
            (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 64)) (then (return (i32.const 0))))
            (local.set $i (i32.add (local.get $i) (i32.const 1)))
            (local.set $refresh (i32.const 0))
            (block $rdone
              (loop $rloop
                (br_if $rdone (i32.ge_u (local.get $i) (local.get $len)))
                (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
                (if (i32.eqz (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))) (then (return (i32.const 0))))
                (local.set $refresh (i32.add (i32.mul (local.get $refresh) (i32.const 10)) (i32.sub (local.get $c) (i32.const 48))))
                (local.set $i (i32.add (local.get $i) (i32.const 1)))
                (br $rloop)))
            (local.set $refresh (i32.mul (local.get $refresh) (i32.const 1000)))))))
    (if (i32.eqz (local.get $refresh)) (then (return (i32.const 0))))
    (i32.store (local.get $out_ptr) (local.get $w))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $h))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $refresh))
    i32.const 1)

  (func (export "node_hw_npu_driver") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 3186769420)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 2150824133)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 746761889)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "node_health_route")
    (param $method_ptr i32)
    (param $method_len i32)
    (param $path_ptr i32)
    (param $path_len i32)
    (result i32)
    (local $m i32)
    (local $p i32)
    (local.set $m (call $fnv_lower (local.get $method_ptr) (local.get $method_len)))
    (local.set $p (call $fnv_lower (local.get $path_ptr) (local.get $path_len)))
    (if (i32.eq (local.get $m) (i32.const 4012403877)) (then (return (i32.const 200))))
    (if
      (i32.and
        (i32.eq (local.get $m) (i32.const 1410115415))
        (i32.or (i32.eq (local.get $p) (i32.const 1923151932)) (i32.eq (local.get $p) (i32.const 705468254))))
      (then (return (i32.const 200))))
    (if
      (i32.and
        (i32.eq (local.get $m) (i32.const 1410115415))
        (i32.or
          (i32.eq (local.get $p) (i32.const 3753781494))
          (i32.or
            (i32.eq (local.get $p) (i32.const 3219592631))
            (i32.or
              (i32.eq (local.get $p) (i32.const 2514957849))
              (i32.eq (local.get $p) (i32.const 2016387753))))))
      (then (return (i32.const 200))))
    (if
      (i32.and
        (i32.eq (local.get $m) (i32.const 2313681479))
        (i32.eq (local.get $p) (i32.const 2129929303)))
      (then (return (i32.const 200))))
    i32.const 404)
)
