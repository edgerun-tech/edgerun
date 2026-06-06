(func (export "dhcpv6_message_type") (param $id i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $id) (i32.const 1)) (i32.le_u (local.get $id) (i32.const 13)))
      (then (return (local.get $id))))
    i32.const 0)

  (func (export "dhcpv6_message_role") (param $id i32) (result i32)
    (if (i32.or (i32.eq (local.get $id) (i32.const 1)) (i32.or (i32.eq (local.get $id) (i32.const 3)) (i32.or (i32.eq (local.get $id) (i32.const 5)) (i32.or (i32.eq (local.get $id) (i32.const 6)) (i32.or (i32.eq (local.get $id) (i32.const 8)) (i32.or (i32.eq (local.get $id) (i32.const 9)) (i32.eq (local.get $id) (i32.const 11))))))))
      (then (return (i32.const 1)))) ;; client to server
    (if (i32.or (i32.eq (local.get $id) (i32.const 2)) (i32.or (i32.eq (local.get $id) (i32.const 7)) (i32.eq (local.get $id) (i32.const 10))))
      (then (return (i32.const 2)))) ;; server to client
    (if (i32.or (i32.eq (local.get $id) (i32.const 12)) (i32.eq (local.get $id) (i32.const 13)))
      (then (return (i32.const 3)))) ;; relay
    i32.const 0)

  (func (export "dhcpv6_transaction_id") (param $b0 i32) (param $b1 i32) (param $b2 i32) (result i32)
    (i32.or
      (i32.shl (i32.and (local.get $b0) (i32.const 255)) (i32.const 16))
      (i32.or
        (i32.shl (i32.and (local.get $b1) (i32.const 255)) (i32.const 8))
        (i32.and (local.get $b2) (i32.const 255)))))

  (func (export "dhcpv6_option_class") (param $code i32) (result i32)
    (if (i32.or (i32.eq (local.get $code) (i32.const 1)) (i32.eq (local.get $code) (i32.const 2))) (then (return (i32.const 1)))) ;; identifiers
    (if (i32.or (i32.eq (local.get $code) (i32.const 3)) (i32.or (i32.eq (local.get $code) (i32.const 4)) (i32.or (i32.eq (local.get $code) (i32.const 5)) (i32.or (i32.eq (local.get $code) (i32.const 25)) (i32.eq (local.get $code) (i32.const 26)))))) (then (return (i32.const 2)))) ;; identity/address/prefix
    (if (i32.or (i32.eq (local.get $code) (i32.const 6)) (i32.or (i32.eq (local.get $code) (i32.const 23)) (i32.or (i32.eq (local.get $code) (i32.const 24)) (i32.eq (local.get $code) (i32.const 31))))) (then (return (i32.const 3)))) ;; requested config
    (if (i32.or (i32.eq (local.get $code) (i32.const 8)) (i32.or (i32.eq (local.get $code) (i32.const 13)) (i32.or (i32.eq (local.get $code) (i32.const 14)) (i32.eq (local.get $code) (i32.const 20))))) (then (return (i32.const 4)))) ;; control/status
    (if (i32.eq (local.get $code) (i32.const 9)) (then (return (i32.const 5)))) ;; relay message
    i32.const 0)

  (func (export "dhcpv6_option_length_status") (param $code i32) (param $len i32) (result i32)
    (if (i32.eq (local.get $code) (i32.const 1)) (then (return (i32.ge_u (local.get $len) (i32.const 4)))))
    (if (i32.eq (local.get $code) (i32.const 2)) (then (return (i32.ge_u (local.get $len) (i32.const 4)))))
    (if (i32.or (i32.eq (local.get $code) (i32.const 3)) (i32.eq (local.get $code) (i32.const 25))) (then (return (i32.ge_u (local.get $len) (i32.const 12)))))
    (if (i32.eq (local.get $code) (i32.const 5)) (then (return (i32.ge_u (local.get $len) (i32.const 24)))))
    (if (i32.eq (local.get $code) (i32.const 26)) (then (return (i32.ge_u (local.get $len) (i32.const 25)))))
    (if (i32.eq (local.get $code) (i32.const 6)) (then (return (i32.eqz (i32.rem_u (local.get $len) (i32.const 2))))))
    (if (i32.eq (local.get $code) (i32.const 8)) (then (return (i32.eq (local.get $len) (i32.const 2)))))
    (if (i32.eq (local.get $code) (i32.const 13)) (then (return (i32.ge_u (local.get $len) (i32.const 2)))))
    (if (i32.eq (local.get $code) (i32.const 23)) (then (return (i32.eqz (i32.rem_u (local.get $len) (i32.const 16))))))
    i32.const 1)

  (func (export "dhcpv6_status_code") (param $code i32) (result i32)
    (if (i32.le_u (local.get $code) (i32.const 6)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $code) (i32.const 7)) (i32.le_u (local.get $code) (i32.const 9))) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "dhcpv6_duid_type") (param $duid_type i32) (param $wire_len i32) (result i32)
    (if (i32.lt_u (local.get $wire_len) (i32.const 4)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $duid_type) (i32.const 1)) (then (return (i32.ge_u (local.get $wire_len) (i32.const 8)))))
    (if (i32.eq (local.get $duid_type) (i32.const 2)) (then (return (i32.ge_u (local.get $wire_len) (i32.const 8)))))
    (if (i32.eq (local.get $duid_type) (i32.const 3)) (then (return (i32.ge_u (local.get $wire_len) (i32.const 4)))))
    (if (i32.eq (local.get $duid_type) (i32.const 4)) (then (return (i32.eq (local.get $wire_len) (i32.const 18)))))
    i32.const 0)

  (func (export "dhcpv6_server_response") (param $msg_type i32) (param $has_clientid i32) (param $has_iaid i32) (param $rapid_commit i32) (result i32)
    (if (i32.eq (local.get $msg_type) (i32.const 1))
      (then
        (if (i32.eqz (i32.and (local.get $has_clientid) (local.get $has_iaid))) (then (return (i32.const 0))))
        (if (local.get $rapid_commit) (then (return (i32.const 7))))
        (return (i32.const 2))))
    (if (i32.or (i32.eq (local.get $msg_type) (i32.const 3)) (i32.or (i32.eq (local.get $msg_type) (i32.const 5)) (i32.or (i32.eq (local.get $msg_type) (i32.const 6)) (i32.eq (local.get $msg_type) (i32.const 11)))))
      (then (return (i32.const 7))))
    i32.const 0)

  (func (export "dhcpv6_default_lifetime") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 3600))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 7200))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 86400))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 1800))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 2700))))
    i32.const 0)
