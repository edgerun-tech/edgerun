(module
  (memory 2)

  ;; Canonical local proof record writer for edgerun.local-tor-delivery.v0.
  ;;
  ;; Input at fields_ptr is 21 contiguous 32-byte fields:
  ;; app_id, commit_event, source_event, relay_transit_receipt,
  ;; relay_delivery_receipt, hidden_service_registration_receipt,
  ;; hsdir_fetch_request, hsdir_publish_header, descriptor_armor,
  ;; contact_frame, message_frame, wat_contact_state, wat_message_state,
  ;; CREATE2, CREATED2, EXTEND2, EXTENDED2, BEGIN, DATA, END,
  ;; circuit_receipt.
  ;;
  ;; Output layout:
  ;; u32 magic 'ERDP'
  ;; u32 version 0
  ;; u32 payload_bytes
  ;; u32 payload_sealed
  ;; u32 route_identity_len
  ;; u32 hidden_service_identity_len
  ;; bytes route_identity_ascii[16]
  ;; bytes hidden_service_identity_ascii[16]
  ;; bytes fields[21][32]
  (func (export "er_local_tor_delivery_proof_write")
    (param $out_ptr i32)
    (param $fields_ptr i32)
    (param $route_identity_ptr i32)
    (param $hidden_service_identity_ptr i32)
    (param $payload_bytes i32)
    (param $payload_sealed i32)
    (result i32)
    (local $i i32)

    local.get $out_ptr
    i32.const 0x45524450
    i32.store

    local.get $out_ptr
    i32.const 4
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $payload_bytes
    i32.store

    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $payload_sealed
    i32.store

    local.get $out_ptr
    i32.const 16
    i32.add
    i32.const 16
    i32.store

    local.get $out_ptr
    i32.const 20
    i32.add
    i32.const 16
    i32.store

    i32.const 0
    local.set $i
    block $done_route
      loop $copy_route
        local.get $i
        i32.const 16
        i32.eq
        br_if $done_route
        local.get $out_ptr
        i32.const 24
        i32.add
        local.get $i
        i32.add
        local.get $route_identity_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_route
      end
    end

    i32.const 0
    local.set $i
    block $done_service
      loop $copy_service
        local.get $i
        i32.const 16
        i32.eq
        br_if $done_service
        local.get $out_ptr
        i32.const 40
        i32.add
        local.get $i
        i32.add
        local.get $hidden_service_identity_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_service
      end
    end

    i32.const 0
    local.set $i
    block $done_fields
      loop $copy_fields
        local.get $i
        i32.const 672
        i32.eq
        br_if $done_fields
        local.get $out_ptr
        i32.const 56
        i32.add
        local.get $i
        i32.add
        local.get $fields_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_fields
      end
    end

    i32.const 728)
)
