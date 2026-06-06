
  (func (export "proto_standard_id") (result i32)
    i32.const 300095)

  (func $m93ascii_lower (param $c i32) (result i32)
    (if
      (i32.and
        (i32.ge_u (local.get $c) (i32.const 65))
        (i32.le_u (local.get $c) (i32.const 90)))
      (then (return (i32.add (local.get $c) (i32.const 32)))))
    local.get $c)

  (func $m93fnv_lower (param $ptr i32) (param $len i32) (result i32)
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
              (call $m93ascii_lower
                (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
            (i32.const 0x01000193)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $h)

  (func $provider_id_from_hash (param $h i32) (result i32)
    (if (i32.eq (local.get $h) (i32.const 1676413470)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $h) (i32.const 2065698207)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $h) (i32.const 4120678169)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "exchange_provider_code")
    (param $ptr i32)
    (param $len i32)
    (result i32)
    (call $provider_id_from_hash (call $m93fnv_lower (local.get $ptr) (local.get $len))))

  (func (export "exchange_provider_features") (param $provider i32) (result i32)
    (if (i32.eq (local.get $provider) (i32.const 1))
      (then (return (i32.or (i32.const 7) (i32.shl (i32.const 3) (i32.const 8))))))
    (if (i32.eq (local.get $provider) (i32.const 2))
      (then (return (i32.or (i32.const 15) (i32.shl (i32.const 2) (i32.const 8))))))
    (if (i32.eq (local.get $provider) (i32.const 3))
      (then (return (i32.or (i32.const 7) (i32.shl (i32.const 1) (i32.const 8))))))
    i32.const 0)

  (func (export "exchange_map_provider_status")
    (param $provider_ptr i32)
    (param $provider_len i32)
    (param $status_ptr i32)
    (param $status_len i32)
    (result i32)
    (local $provider i32)
    (local $status i32)
    (local.set $provider
      (call $provider_id_from_hash
        (call $m93fnv_lower (local.get $provider_ptr) (local.get $provider_len))))
    (local.set $status (call $m93fnv_lower (local.get $status_ptr) (local.get $status_len)))

    (if (i32.eq (local.get $provider) (i32.const 1))
      (then
        (if (i32.eq (local.get $status) (i32.const 2579463996)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 837645499)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 814672450)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 3209629253)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 3376168477)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 2921832239)) (then (return (i32.const 6))))
        (if (i32.eq (local.get $status) (i32.const 2281165435)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 3260912582)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 1398023497)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 15))))
        (if (i32.eq (local.get $status) (i32.const 440867849)) (then (return (i32.const 11))))
        (if (i32.eq (local.get $status) (i32.const 928465625)) (then (return (i32.const 12))))
        (if (i32.eq (local.get $status) (i32.const 1658595102)) (then (return (i32.const 13))))
        (if (i32.eq (local.get $status) (i32.const 864990770)) (then (return (i32.const 14))))
        (if (i32.eq (local.get $status) (i32.const 1150267360)) (then (return (i32.const 17))))
        (if (i32.eq (local.get $status) (i32.const 2220750876)) (then (return (i32.const 18))))))

    (if (i32.eq (local.get $provider) (i32.const 2))
      (then
        (if (i32.eq (local.get $status) (i32.const 681154065)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 4007265960)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 2845129065)) (then (return (i32.const 5))))
        (if (i32.eq (local.get $status) (i32.const 2281165435)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 1398023497)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 2917234339)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 9))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 15))))
        (if (i32.eq (local.get $status) (i32.const 1658595102)) (then (return (i32.const 13))))
        (if (i32.eq (local.get $status) (i32.const 864990770)) (then (return (i32.const 14))))))

    (if (i32.eq (local.get $provider) (i32.const 3))
      (then
        (if (i32.eq (local.get $status) (i32.const 2579463996)) (then (return (i32.const 3))))
        (if (i32.eq (local.get $status) (i32.const 814672450)) (then (return (i32.const 4))))
        (if (i32.eq (local.get $status) (i32.const 3260912582)) (then (return (i32.const 6))))
        (if (i32.eq (local.get $status) (i32.const 4101275922)) (then (return (i32.const 7))))
        (if (i32.eq (local.get $status) (i32.const 3769421748)) (then (return (i32.const 8))))
        (if (i32.eq (local.get $status) (i32.const 2220750876)) (then (return (i32.const 9))))))

    i32.const 17)

  (func $asset_hash_known_pair (param $settlement i32) (param $pay i32) (result i32)
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 585249028)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 1339528032)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1618751625))
               (i32.eq (local.get $pay) (i32.const 2191266556)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 1339528032))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 2191266556))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $settlement) (i32.const 585249028))
               (i32.eq (local.get $pay) (i32.const 1618751625)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_supports_pair_hash")
    (param $provider i32)
    (param $settlement_symbol_hash i32)
    (param $pay_symbol_hash i32)
    (result i32)
    (if (call $asset_hash_known_pair (local.get $settlement_symbol_hash) (local.get $pay_symbol_hash))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.eq (local.get $provider) (i32.const 2))
        (i32.or
          (i32.and
            (i32.eq (local.get $settlement_symbol_hash) (i32.const 1618751625))
            (i32.eq (local.get $pay_symbol_hash) (i32.const 3795205537)))
          (i32.and
            (i32.eq (local.get $settlement_symbol_hash) (i32.const 3795205537))
            (i32.eq (local.get $pay_symbol_hash) (i32.const 1618751625)))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $m93fnv_lower (local.get $ptr) (local.get $len)))

  (func (export "exchange_event_stream_type") (param $event_kind i32) (result i32)
    (if (i32.eq (local.get $event_kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 2)) (then (return (i32.const 2))))
    (if
      (i32.and
        (i32.ge_u (local.get $event_kind) (i32.const 3))
        (i32.le_u (local.get $event_kind) (i32.const 8)))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "exchange_event_has_order_id") (param $event_kind i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $event_kind) (i32.const 2))
      (i32.le_u (local.get $event_kind) (i32.const 8))))

  (func (export "exchange_event_terminal") (param $event_kind i32) (result i32)
    (if (i32.eq (local.get $event_kind) (i32.const 6)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 7)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $event_kind) (i32.const 8)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_projection_terminal_status") (param $status i32) (result i32)
    (if (i32.eq (local.get $status) (i32.const 9)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 13)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 15)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 16)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 17)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 18)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "exchange_terminal_event_for_status") (param $status i32) (result i32)
    (if (i32.eq (local.get $status) (i32.const 9)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $status) (i32.const 15)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $status) (i32.const 16)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $status) (i32.const 18)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "exchange_provider_contradiction")
    (param $canonical_terminal i32)
    (param $canonical_status i32)
    (param $provider_status i32)
    (result i32)
    (if
      (i32.and
        (local.get $canonical_terminal)
        (i32.ne (local.get $canonical_status) (local.get $provider_status)))
      (then (return (i32.const 17))))
    local.get $provider_status)

  (func (export "exchange_settlement_command_valid")
    (param $command_id_len i32)
    (param $target_node_len i32)
    (param $command_type i32)
    (param $idempotency_len i32)
    (param $app_id_len i32)
    (param $payload_len i32)
    (param $issued_ms i64)
    (param $expires_ms i64)
    (result i32)
    (if (i32.eqz (local.get $command_id_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $target_node_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $command_type)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $idempotency_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $app_id_len)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $payload_len)) (then (return (i32.const 0))))
    (if
      (i32.and
        (i64.ne (local.get $expires_ms) (i64.const 0))
        (i64.gt_u (local.get $issued_ms) (local.get $expires_ms)))
      (then (return (i32.const 0))))
    i32.const 1))
