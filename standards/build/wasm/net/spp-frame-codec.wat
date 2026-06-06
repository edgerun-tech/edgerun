  (global $spp_abi_version i32 (i32.const 1))
  (global $spp_mtu_min i32 (i32.const 48))
  (global $spp_mtu_default i32 (i32.const 128))

  (func (export "spp_abi_version") (result i32)
    global.get $spp_abi_version
  )

  ;; Validate an RFCOMM frame header at offset for given length
  ;; Returns 0 on success, error code on failure
  (func (export "spp_validate_frame")
    (param $offset i32)
    (param $length i32)
    (result i32)
    (local $address i32)
    (local $control i32)
    (local $length_hi i32)
    (local $frame_len i32)
    (if (i32.lt_s (local.get $length) (i32.const 3))
      (then (return (i32.const 1))) ;; too short for frame header
    )
    (local.set $address (i32.load8_u offset=0 (local.get $offset)))
    (local.set $control (i32.load8_u offset=1 (local.get $offset)))
    (local.set $length_hi (i32.load8_u offset=2 (local.get $offset)))
    (local.set $frame_len
      (i32.add (i32.and (local.get $length_hi) (i32.const 0x7F)) (i32.const 3))
    )
    (if (i32.gt_s (local.get $frame_len) (local.get $length))
      (then (return (i32.const 2))) ;; frame extends past buffer
    )
    (i32.const 0) ;; valid
  )

  ;; Check if control byte indicates UIH (Unnumbered Info with Header check)
  (func (export "spp_is_uih_frame")
    (param $control i32)
    (result i32)
    (i32.eq (i32.and (local.get $control) (i32.const 0xEF)) (i32.const 0xEF))
  )

  ;; Extract RFCOMM channel from address byte
  (func (export "spp_channel_from_address")
    (param $address i32)
    (result i32)
    (i32.shr_u (i32.and (local.get $address) (i32.const 0xFC)) (i32.const 2))
  )

  ;; Maximum payload size for a given MTU
  (func (export "spp_max_payload") (param $mtu i32) (result i32)
    (i32.sub (local.get $mtu) (i32.const 3))
  )