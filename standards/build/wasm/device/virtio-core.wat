;; Virtio PCI/MMIO and split-queue semantics captured from edgerun-virtio.
  ;;
  ;; Status bits: acknowledge=1 driver=2 driver_ok=4 features_ok=8 failed=128.
  ;; Device types: net=1 blk=2 console=3 rng=4.
  ;; Queue constants: split queue size=16, net header=12, net buffer=2048,
  ;; sector size=512, console/rng chunk=256.

  (func $m214bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $m214min (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_s
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (export "virtio_modern_device_type" (func $virtio_modern_device_type))
  (func $virtio_modern_device_type (param $device_id i32) (result i32)
    local.get $device_id
    i32.const 0x1041
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $device_id
      i32.const 0x1042
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $device_id
        i32.const 0x1043
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $device_id
          i32.const 0x1044
          i32.eq
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "virtio_interrupt_flags" (func $virtio_interrupt_flags))
  (func $virtio_interrupt_flags (param $raw i32) (result i32)
    ;; Low bit: used ring. Second bit: config change.
    local.get $raw
    i32.const 3
    i32.and)

  (export "virtio_mmio_valid" (func $virtio_mmio_valid))
  (func $virtio_mmio_valid
    (param $base_nonzero i32)
    (param $magic i32)
    (param $version i32)
    (result i32)
    local.get $base_nonzero
    call $m214bool
    local.get $magic
    i32.const 0x74726976
    i32.eq
    i32.and
    local.get $version
    i32.const 2
    i32.eq
    i32.and)

  (export "virtio_mmio_device_info_valid" (func $virtio_mmio_device_info_valid))
  (func $virtio_mmio_device_info_valid
    (param $mmio_valid i32)
    (param $device_type i32)
    (result i32)
    local.get $mmio_valid
    call $m214bool
    local.get $device_type
    i32.const 0
    i32.ne
    i32.and)

  (export "virtio_open_device_result" (func $virtio_open_device_result))
  (func $virtio_open_device_result
    (param $actual_type i32)
    (param $requested_type i32)
    (param $transport_found i32)
    (param $claimed i32)
    (result i32)
    ;; ok=0 wrong-type=1 not-found=2 already-claimed=3.
    local.get $actual_type
    local.get $requested_type
    i32.ne
    if (result i32)
      i32.const 1
    else
      local.get $transport_found
      call $m214bool
      i32.eqz
      if (result i32)
        i32.const 2
      else
        local.get $claimed
        call $m214bool
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "virtio_modern_pci_transport_valid" (func $virtio_modern_pci_transport_valid))
  (func $virtio_modern_pci_transport_valid
    (param $has_common i32)
    (param $has_notify i32)
    (result i32)
    local.get $has_common
    call $m214bool
    local.get $has_notify
    call $m214bool
    i32.and)

  (export "virtio_negotiate_features" (func $virtio_negotiate_features))
  (func $virtio_negotiate_features
    (param $host_low i32)
    (param $host_high i32)
    (param $supported_low i32)
    (param $supported_high i32)
    (param $features_ok_persisted i32)
    (result i32)
    ;; Returns final status on success/failure. VERSION_1 is bit 32.
    local.get $host_high
    local.get $supported_high
    i32.and
    i32.const 1
    i32.and
    i32.eqz
    if (result i32)
      i32.const 128
    else
      local.get $features_ok_persisted
      call $m214bool
      if (result i32)
        i32.const 11
      else
        i32.const 139
      end
    end)

  (export "virtio_driver_feature_low" (func $virtio_driver_feature_low))
  (func $virtio_driver_feature_low
    (param $host_low i32)
    (param $supported_low i32)
    (result i32)
    local.get $host_low
    local.get $supported_low
    i32.and)

  (export "virtio_driver_feature_high" (func $virtio_driver_feature_high))
  (func $virtio_driver_feature_high
    (param $host_high i32)
    (param $supported_high i32)
    (result i32)
    local.get $host_high
    local.get $supported_high
    i32.and)

  (export "virtio_queue_configured_size" (func $virtio_queue_configured_size))
  (func $virtio_queue_configured_size
    (param $host_queue_size i32)
    (param $max_queue_size i32)
    (param $m214min_queue_size i32)
    (result i32)
    (local $queue_size i32)
    local.get $host_queue_size
    local.get $max_queue_size
    call $m214min
    local.tee $queue_size
    local.get $m214min_queue_size
    i32.lt_s
    if (result i32)
      i32.const 0
    else
      local.get $queue_size
    end)

  (export "virtio_post_avail_next_idx" (func $virtio_post_avail_next_idx))
  (func $virtio_post_avail_next_idx
    (param $queue_size i32)
    (param $idx i32)
    (result i32)
    local.get $queue_size
    i32.const 0
    i32.eq
    if (result i32)
      local.get $idx
    else
      local.get $idx
      i32.const 1
      i32.add
      i32.const 0xffff
      i32.and
    end)

  (export "virtio_post_avail_ring_slot" (func $virtio_post_avail_ring_slot))
  (func $virtio_post_avail_ring_slot
    (param $queue_size i32)
    (param $idx i32)
    (result i32)
    local.get $queue_size
    i32.const 0
    i32.eq
    if (result i32)
      i32.const -1
    else
      local.get $idx
      local.get $queue_size
      i32.rem_u
    end)

  (export "virtio_next_used_result" (func $virtio_next_used_result))
  (func $virtio_next_used_result
    (param $used_idx i32)
    (param $last_used_idx i32)
    (result i32)
    ;; none=0, some-next-last in low 16 with high marker=1.
    local.get $used_idx
    local.get $last_used_idx
    i32.eq
    if (result i32)
      i32.const 0
    else
      i32.const 0x10000
      local.get $last_used_idx
      i32.const 1
      i32.add
      i32.const 0xffff
      i32.and
      i32.or
    end)

  (export "virtio_single_used_result" (func $virtio_single_used_result))
  (func $virtio_single_used_result
    (param $used_idx i32)
    (param $last_used_idx i32)
    (result i32)
    ;; none=0, ok=1, invalid-jump=2.
    local.get $used_idx
    local.get $last_used_idx
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $used_idx
      local.get $last_used_idx
      i32.const 1
      i32.add
      i32.const 0xffff
      i32.and
      i32.eq
      if (result i32)
        i32.const 1
      else
        i32.const 2
      end
    end)

  (export "virtio_wait_completion_result" (func $virtio_wait_completion_result))
  (func $virtio_wait_completion_result
    (param $used_idx i32)
    (param $last_used_idx i32)
    (param $spin_count i32)
    (result i32)
    ;; pending=0 ready=1 timeout=2.
    local.get $used_idx
    local.get $last_used_idx
    i32.ne
    if (result i32)
      i32.const 1
    else
      local.get $spin_count
      i32.const 5000
      i32.gt_u
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "virtio_net_tx_frame_len" (func $virtio_net_tx_frame_len))
  (func $virtio_net_tx_frame_len (param $payload_len i32) (result i32)
    (local $frame_len i32)
    local.get $payload_len
    i32.const 0
    i32.le_s
    if (result i32)
      i32.const 0
    else
      local.get $payload_len
      i32.const 12
      i32.add
      local.tee $frame_len
      i32.const 2048
      i32.gt_s
      if (result i32)
        i32.const 0
      else
        local.get $frame_len
      end
    end)

  (export "virtio_net_rx_payload_len" (func $virtio_net_rx_payload_len))
  (func $virtio_net_rx_payload_len (param $frame_len i32) (result i32)
    local.get $frame_len
    i32.const 12
    i32.lt_s
    local.get $frame_len
    i32.const 2048
    i32.gt_s
    i32.or
    if (result i32)
      i32.const -1
    else
      local.get $frame_len
      i32.const 12
      i32.sub
    end)

  (export "virtio_tx_take_descriptor" (func $virtio_tx_take_descriptor))
  (func $virtio_tx_take_descriptor (param $free_mask i32) (result i32)
    ;; Returns descriptor id, or -1 when no bit is free.
    local.get $free_mask
    i32.const 0
    i32.eq
    if (result i32)
      i32.const -1
    else
      local.get $free_mask
      i32.ctz
    end)

  (export "virtio_tx_free_after_reap" (func $virtio_tx_free_after_reap))
  (func $virtio_tx_free_after_reap
    (param $free_mask i32)
    (param $used_id i32)
    (result i32)
    local.get $used_id
    i32.const 16
    i32.lt_u
    if (result i32)
      local.get $free_mask
      i32.const 1
      local.get $used_id
      i32.shl
      i32.or
    else
      local.get $free_mask
    end)

  (export "virtio_blk_chunk_sector_count" (func $virtio_blk_chunk_sector_count))
  (func $virtio_blk_chunk_sector_count (param $len i32) (result i32)
    local.get $len
    i32.const 512
    i32.eq
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end)

  (export "virtio_blk_range_in_bounds" (func $virtio_blk_range_in_bounds))
  (func $virtio_blk_range_in_bounds
    (param $start_sector i64)
    (param $sector_count i64)
    (param $sectors i64)
    (result i32)
    (local $end_sector i64)
    local.get $start_sector
    local.get $sector_count
    i64.add
    local.tee $end_sector
    local.get $start_sector
    i64.lt_u
    if (result i32)
      i32.const 0
    else
      local.get $end_sector
      local.get $sectors
      i64.le_u
    end)

  (export "virtio_blk_request_valid" (func $virtio_blk_request_valid))
  (func $virtio_blk_request_valid
    (param $initialized i32)
    (param $read_only i32)
    (param $write_request i32)
    (param $buffer_len i32)
    (param $in_bounds i32)
    (result i32)
    ;; ok=0 not-init=1 invalid-buffer=2 out-of-range=3 read-only=4.
    local.get $initialized
    call $m214bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $buffer_len
      i32.const 512
      i32.ne
      if (result i32)
        i32.const 2
      else
        local.get $in_bounds
        call $m214bool
        i32.eqz
        if (result i32)
          i32.const 3
        else
          local.get $read_only
          call $m214bool
          local.get $write_request
          call $m214bool
          i32.and
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "virtio_blk_descriptor_chain" (func $virtio_blk_descriptor_chain))
  (func $virtio_blk_descriptor_chain
    (param $data_len i32)
    (param $read i32)
    (result i32)
    ;; bit pack: header_next low8 | data_flags next8 | status_flags next8.
    local.get $data_len
    i32.const 512
    i32.gt_s
    if (result i32)
      i32.const 0
    else
      local.get $data_len
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 2
        i32.const 2
        i32.const 16
        i32.shl
        i32.or
      else
        i32.const 1
        local.get $read
        call $m214bool
        if (result i32)
          i32.const 3
        else
          i32.const 1
        end
        i32.const 8
        i32.shl
        i32.or
        i32.const 2
        i32.const 16
        i32.shl
        i32.or
      end
    end)

  (export "virtio_blk_init_result" (func $virtio_blk_init_result))
  (func $virtio_blk_init_result
    (param $features_ok i32)
    (param $sectors_nonzero i32)
    (param $block_size i32)
    (param $queue_size i32)
    (result i32)
    ;; ok=0 features=1 out-of-range=2 unsupported-block=3 queue=4.
    local.get $features_ok
    call $m214bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $sectors_nonzero
      call $m214bool
      i32.eqz
      if (result i32)
        i32.const 2
      else
        local.get $block_size
        i32.const 512
        i32.ne
        if (result i32)
          i32.const 3
        else
          local.get $queue_size
          i32.const 3
          i32.lt_s
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "virtio_rng_request_len" (func $virtio_rng_request_len))
  (func $virtio_rng_request_len (param $remaining_len i32) (result i32)
    local.get $remaining_len
    i32.const 256
    call $m214min)

  (export "virtio_rng_completion_valid" (func $virtio_rng_completion_valid))
  (func $virtio_rng_completion_valid
    (param $elem_id i32)
    (param $elem_len i32)
    (param $request_len i32)
    (result i32)
    ;; ok length, or -1 for invalid descriptor/length.
    local.get $elem_id
    i32.const 0
    i32.ne
    local.get $elem_len
    i32.const 0
    i32.le_s
    i32.or
    if (result i32)
      i32.const -1
    else
      local.get $elem_len
      local.get $request_len
      call $m214min
    end)

  (export "virtio_console_rx_len" (func $virtio_console_rx_len))
  (func $virtio_console_rx_len (param $len i32) (result i32)
    local.get $len
    i32.const 256
    i32.gt_s
    if (result i32)
      i32.const -1
    else
      local.get $len
    end)

  (export "virtio_console_write_chunk_len" (func $virtio_console_write_chunk_len))
  (func $virtio_console_write_chunk_len (param $remaining_len i32) (result i32)
    local.get $remaining_len
    i32.const 256
    call $m214min)

  (export "virtio_console_tx_completion_result" (func $virtio_console_tx_completion_result))
  (func $virtio_console_tx_completion_result
    (param $elem_id i32)
    (param $chunk_len i32)
    (result i32)
    local.get $elem_id
    i32.const 0
    i32.eq
    if (result i32)
      local.get $chunk_len
    else
      i32.const -1
    end)

  (export "virtio_pci_address" (func $virtio_pci_address))
  (func $virtio_pci_address
    (param $bus i32)
    (param $slot i32)
    (param $func i32)
    (param $offset i32)
    (result i32)
    i32.const 0x80000000
    local.get $bus
    i32.const 16
    i32.shl
    i32.or
    local.get $slot
    i32.const 11
    i32.shl
    i32.or
    local.get $func
    i32.const 8
    i32.shl
    i32.or
    local.get $offset
    i32.const 0xfc
    i32.and
    i32.or)

  (export "virtio_pci_bar_base_valid" (func $virtio_pci_bar_base_valid))
  (func $virtio_pci_bar_base_valid (param $raw i32) (result i32)
    local.get $raw
    i32.const 0
    i32.eq
    local.get $raw
    i32.const -1
    i32.eq
    i32.or
    local.get $raw
    i32.const 1
    i32.and
    i32.or
    i32.eqz)

  (export "virtio_scan_count_next" (func $virtio_scan_count_next))
  (func $virtio_scan_count_next
    (param $found i32)
    (param $vendor i32)
    (param $device_id i32)
    (result i32)
    local.get $vendor
    i32.const 0x1af4
    i32.eq
    local.get $device_id
    call $virtio_modern_device_type
    i32.const 0
    i32.ne
    i32.and
    if (result i32)
      local.get $found
      i32.const 1
      i32.add
    else
      local.get $found
    end)