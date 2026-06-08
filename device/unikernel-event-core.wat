(func (export "unikernel_abi_version") (result i32)
    i32.const 1)

  (func (export "unikernel_event_max_data_len") (result i32)
    i32.const 4096)

  (func $event_type_valid (export "unikernel_event_type_valid") (param $event_type i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $event_type) (i32.const 1))
      (i32.le_u (local.get $event_type) (i32.const 3))))

  (func $network_subtype_valid (export "unikernel_network_subtype_valid") (param $subtype i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $subtype) (i32.const 1))
      (i32.le_u (local.get $subtype) (i32.const 4))))

  (func $disk_subtype_valid (export "unikernel_disk_subtype_valid") (param $subtype i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $subtype) (i32.const 1))
      (i32.le_u (local.get $subtype) (i32.const 3))))

  (func (export "unikernel_timer_subtype_code") (param $raw i32) (result i32)
    i32.const 1)

  ;; 0 ok, 1 bad event type, 2 data too large.
  (func (export "unikernel_event_new_result") (param $event_type i32) (param $data_len i32) (result i32)
    (if (i32.eqz (call $event_type_valid (local.get $event_type)))
      (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $data_len) (i32.const 4096))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "unikernel_network_event_len") (param $subtype i32) (param $payload_len i32) (result i32)
    (if (i32.eqz (call $network_subtype_valid (local.get $subtype)))
      (then (return (i32.const 0))))
    (if (i32.eq (local.get $subtype) (i32.const 3))
      (then
        (if (i32.gt_u (local.get $payload_len) (i32.const 4087))
          (then (return (i32.const 0))))
        (return (i32.add (i32.const 9) (local.get $payload_len)))))
    i32.const 5)

  (func (export "unikernel_disk_event_len") (param $subtype i32) (param $payload_len i32) (result i32)
    (if (i32.eqz (call $disk_subtype_valid (local.get $subtype)))
      (then (return (i32.const 0))))
    (if (i32.eq (local.get $subtype) (i32.const 1))
      (then
        (if (i32.gt_u (local.get $payload_len) (i32.const 4091))
          (then (return (i32.const 0))))
        (return (i32.add (i32.const 5) (local.get $payload_len)))))
    i32.const 5)

  (func (export "unikernel_timer_event_len") (result i32)
    i32.const 9)

  (func (export "unikernel_total_len") (param $data_len i32) (result i32)
    (i32.add (i32.const 1) (local.get $data_len)))

  ;; 0 ok, 1 empty input, 2 bad event type, 3 data too large.
  (func (export "unikernel_from_bytes_result") (param $input_len i32) (param $event_type i32) (result i32)
    (if (i32.eqz (local.get $input_len))
      (then (return (i32.const 1))))
    (if (i32.eqz (call $event_type_valid (local.get $event_type)))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.sub (local.get $input_len) (i32.const 1)) (i32.const 4096))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "unikernel_sock_id_available") (param $event_type i32) (param $data_len i32) (result i32)
    (i32.and
      (i32.eq (local.get $event_type) (i32.const 1))
      (i32.ge_u (local.get $data_len) (i32.const 4))))

  ;; 0 payload slice valid, 1 wrong event type, 2 missing or wrong received header, 3 truncated payload.
  (func (export "unikernel_network_payload_result") (param $event_type i32) (param $data_len i32) (param $subtype i32) (param $payload_len i32) (result i32)
    (if (i32.ne (local.get $event_type) (i32.const 1))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.lt_u (local.get $data_len) (i32.const 9))
          (i32.ne (local.get $subtype) (i32.const 3)))
      (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $data_len) (i32.add (i32.const 9) (local.get $payload_len)))
      (then (return (i32.const 3))))
    i32.const 0)

  ;; 0 data slice valid, 1 wrong event type, 2 missing or wrong read-done header.
  (func (export "unikernel_disk_data_result") (param $event_type i32) (param $data_len i32) (param $subtype i32) (result i32)
    (if (i32.ne (local.get $event_type) (i32.const 2))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.lt_u (local.get $data_len) (i32.const 5))
          (i32.ne (local.get $subtype) (i32.const 1)))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "unikernel_timer_id_available") (param $event_type i32) (param $data_len i32) (result i32)
    (i32.and
      (i32.eq (local.get $event_type) (i32.const 3))
      (i32.ge_u (local.get $data_len) (i32.const 9))))

  ;; 0 ok, 1 full, 2 bad event type, 3 data too large.
  (func (export "unikernel_queue_push_result") (param $len i32) (param $capacity i32) (param $event_type i32) (param $data_len i32) (result i32)
    (if (i32.ge_u (local.get $len) (local.get $capacity))
      (then (return (i32.const 1))))
    (if (i32.eqz (call $event_type_valid (local.get $event_type)))
      (then (return (i32.const 2))))
    (if (i32.gt_u (local.get $data_len) (i32.const 4096))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "unikernel_queue_len_after_push") (param $len i32) (param $capacity i32) (param $ok i32) (result i32)
    (select
      (i32.add (local.get $len) (i32.const 1))
      (local.get $len)
      (i32.and (local.get $ok) (i32.lt_u (local.get $len) (local.get $capacity)))))

  (func (export "unikernel_queue_len_after_pop") (param $len i32) (result i32)
    (select
      (i32.sub (local.get $len) (i32.const 1))
      (i32.const 0)
      (i32.gt_u (local.get $len) (i32.const 0))))

  (func (export "unikernel_queue_next_index") (param $index i32) (param $capacity i32) (result i32)
    (i32.rem_u (i32.add (local.get $index) (i32.const 1)) (local.get $capacity)))

  (func (export "unikernel_queue_clear_len") (result i32)
    i32.const 0)

  ;; 0 no event, 1 disconnect event, 2 connect then received, 3 received only, 4 queue full.
  (func (export "unikernel_virtio_rx_result") (param $rx_len i32) (param $pending i32) (param $queue_has_space i32) (result i32)
    (if (i32.eqz (local.get $queue_has_space))
      (then (return (i32.const 4))))
    (if (i32.eqz (local.get $rx_len))
      (then
        (if (local.get $pending)
          (then (return (i32.const 1))))
        (return (i32.const 0))))
    (if (local.get $pending)
      (then (return (i32.const 3))))
    i32.const 2)

  (func (export "unikernel_virtio_pending_after_rx") (param $rx_len i32) (param $pending i32) (result i32)
    (if (i32.eqz (local.get $rx_len))
      (then (return (i32.const 0))))
    i32.const 1)

  (func (export "unikernel_virtio_sock_after_rx") (param $rx_len i32) (param $pending i32) (param $counter i32) (param $current_sock i32) (result i32)
    (if (i32.and (i32.gt_u (local.get $rx_len) (i32.const 0)) (i32.eqz (local.get $pending)))
      (then (return (i32.add (local.get $counter) (i32.const 1)))))
    local.get $current_sock)

  (func (export "unikernel_virtio_record_error_result") (param $queue_has_space i32) (result i32)
    (select (i32.const 1) (i32.const 0) (local.get $queue_has_space)))

  (func (export "unikernel_poll_many_count") (param $max_events i32) (param $productive_polls i32) (result i32)
    (select
      (local.get $max_events)
      (local.get $productive_polls)
      (i32.lt_u (local.get $max_events) (local.get $productive_polls))))

  ;; 1 default linker.ld, 2 linker-xtensa-esp32s3.ld.
  (func (export "unikernel_linker_script_code") (param $target_xtensa i32) (result i32)
    (select (i32.const 2) (i32.const 1) (local.get $target_xtensa)))

  (func $xtensa_wifi_blob_link_items (export "unikernel_xtensa_wifi_blob_link_items") (param $wifi_blob i32) (result i32)
    (select (i32.const 9) (i32.const 0) (local.get $wifi_blob)))

  (func $xtensa_ble_blob_link_items (export "unikernel_xtensa_ble_blob_link_items") (param $ble_blob i32) (result i32)
    (select (i32.const 12) (i32.const 0) (local.get $ble_blob)))

  (func (export "unikernel_build_emit_count") (param $target_os_none i32) (param $target_xtensa i32) (param $wifi_blob i32) (param $ble_blob i32) (result i32)
    (local $count i32)
    (if (i32.eqz (local.get $target_os_none))
      (then (return (i32.const 0))))
    (local.set $count (i32.const 2))
    (if (local.get $target_xtensa)
      (then
        (local.set $count (i32.add (local.get $count) (i32.const 5)))
        (local.set $count (i32.add (local.get $count) (call $xtensa_wifi_blob_link_items (local.get $wifi_blob))))
        (local.set $count (i32.add (local.get $count) (call $xtensa_ble_blob_link_items (local.get $ble_blob))))))
    local.get $count)
