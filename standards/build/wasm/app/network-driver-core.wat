(module
  (import "edgerun-core" "memory" (memory 1))
;; Frame-device and RTL8125 NIC semantics plundered from edgerun-network-driver.

  (func (export "network_driver_abi_version") (result i32) i32.const 1)
  (func (export "ethernet_header_len") (result i32) i32.const 14)
  (func (export "ethernet_mtu") (result i32) i32.const 1500)
  (func (export "ethernet_frame_len") (result i32) i32.const 1514)
  (func (export "rtl8125_vendor_id") (result i32) i32.const 4332)
  (func (export "rtl8125_device_id") (result i32) i32.const 33061)

  (func (export "frame_driver_kind_valid") (param $kind i32) (result i32)
    ;; Unknown, InMemory, Rtl8125, StdUdp, Tap, VirtioNet.
    (if (i32.le_u (local.get $kind) (i32.const 5)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "in_memory_push_rx_result") (param $frame_len i32) (param $max_frame i32) (param $rx_len i32) (param $rx_cap i32) (result i32)
    ;; 0 ok, 1 invalid frame length, 2 no descriptor.
    (if (i32.gt_u (local.get $frame_len) (local.get $max_frame)) (then (return (i32.const 1))))
    (if (i32.ge_u (local.get $rx_len) (local.get $rx_cap)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "in_memory_send_result") (param $frame_len i32) (param $max_frame i32) (param $tx_len i32) (param $tx_cap i32) (result i32)
    ;; 0 ok, 1 invalid frame length, 2 no descriptor.
    (if (i32.gt_u (local.get $frame_len) (local.get $max_frame)) (then (return (i32.const 1))))
    (if (i32.ge_u (local.get $tx_len) (local.get $tx_cap)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "in_memory_recv_result") (param $rx_len i32) (param $out_len i32) (param $frame_len i32) (result i32)
    ;; 0 none, 1 copied frame, 2 invalid output buffer.
    (if (i32.eqz (local.get $rx_len)) (then (return (i32.const 0))))
    (if (i32.lt_u (local.get $out_len) (local.get $frame_len)) (then (return (i32.const 2))))
    i32.const 1)

  (func (export "ring_next_index") (param $read i32) (param $cap i32) (result i32)
    (i32.rem_u (i32.add (local.get $read) (i32.const 1)) (local.get $cap)))

  (func (export "virtio_error_map") (param $virtio_error i32) (result i32)
    ;; 1 invalid buffer, 2 invalid frame, 3 no tx descriptor, 4 not initialized, 7 device.
    (if (i32.and (i32.ge_u (local.get $virtio_error) (i32.const 1)) (i32.le_u (local.get $virtio_error) (i32.const 4))) (then (return (local.get $virtio_error))))
    i32.const 7)

  (func (export "std_udp_send_result") (param $has_peer i32) (param $io_ok i32) (result i32)
    ;; 0 ok, 4 not initialized, 7 io/device error.
    (if (i32.eqz (local.get $has_peer)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $io_ok)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "std_udp_recv_result") (param $io_code i32) (result i32)
    ;; 0 some frame, 1 none/would-block/timed-out, 7 io error.
    (if (i32.eq (local.get $io_code) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $io_code) (i32.const 1)) (i32.eq (local.get $io_code) (i32.const 2))) (then (return (i32.const 1))))
    i32.const 7)

  (func (export "tap_open_result") (param $io_open_ok i32) (param $ioctl_ok i32) (result i32)
    ;; 0 ok, 7 device/io error.
    (if (i32.eqz (i32.and (local.get $io_open_ok) (local.get $ioctl_ok))) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "rtl_init_result") (param $mmio_base_nonzero i32) (param $reset_ok i32) (result i32)
    ;; 0 ok, 1 no mmio, 2 reset failed.
    (if (i32.eqz (local.get $mmio_base_nonzero)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $reset_ok)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "rtl_link_up") (param $phy_status i32) (result i32)
    (if (i32.ne (i32.and (local.get $phy_status) (i32.const 2)) (i32.const 0)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "rtl_tx_available") (param $tx_cur i32) (param $tx_dirty i32) (result i32)
    ;; 32 normal-priority descriptors. 1 means a descriptor may be used.
    (if (i32.ge_u (i32.sub (local.get $tx_cur) (local.get $tx_dirty)) (i32.const 32)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "rtl_send_result") (param $len i32) (param $tx_available i32) (param $descriptor_owned i32) (result i32)
    ;; 0 ok, 1 invalid length, 2 no descriptor.
    (if (i32.or (i32.eqz (local.get $len)) (i32.or (i32.gt_u (local.get $len) (i32.const 2048)) (i32.gt_u (local.get $len) (i32.const 16383)))) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $tx_available)) (then (return (i32.const 2))))
    (if (local.get $descriptor_owned) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "rtl_tx_opts1") (param $entry i32) (param $len i32) (result i32)
    ;; OWN | FIRST | LAST | optional RING_END | len.
    (i32.or
      (i32.or (i32.or (i32.const 0x80000000) (i32.const 0x20000000)) (i32.const 0x10000000))
      (i32.or
        (if (result i32) (i32.eq (local.get $entry) (i32.const 31)) (then i32.const 0x40000000) (else i32.const 0))
        (local.get $len))))

  (func (export "rtl_rx_owned_opts1") (param $entry i32) (result i32)
    ;; OWN | optional RING_END | RX_BUF_SIZE.
    (i32.or
      (i32.const 0x80000800)
      (if (result i32) (i32.eq (local.get $entry) (i32.const 31)) (then i32.const 0x40000000) (else i32.const 0))))

  (func (export "rtl_rx_payload_len") (param $status i32) (result i32)
    (local $len i32)
    (local.set $len (i32.and (local.get $status) (i32.const 0x3fff)))
    (if (i32.lt_u (local.get $len) (i32.const 4)) (then (return (i32.const 0))))
    (i32.sub (local.get $len) (i32.const 4)))

  (func (export "rtl_recv_result") (param $status i32) (param $out_len i32) (result i32)
    ;; 0 none/owned by NIC, 1 copied frame, 2 invalid/drop/error.
    (local $len i32)
    (if (i32.ne (i32.and (local.get $status) (i32.const 0x80000000)) (i32.const 0)) (then (return (i32.const 0))))
    (local.set $len (call $rtl_rx_payload_len_internal (local.get $status)))
    (if (i32.ne (i32.and (local.get $status) (i32.const 0x00780000)) (i32.const 0)) (then (return (i32.const 2))))
    (if (i32.ne (i32.and (local.get $status) (i32.const 0x30000000)) (i32.const 0x30000000)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $len)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $out_len)) (then (return (i32.const 2))))
    i32.const 1)

  (func $rtl_rx_payload_len_internal (param $status i32) (result i32)
    (local $len i32)
    (local.set $len (i32.and (local.get $status) (i32.const 0x3fff)))
    (if (i32.lt_u (local.get $len) (i32.const 4)) (then (return (i32.const 0))))
    (i32.sub (local.get $len) (i32.const 4)))

  (func (export "rtl_reap_tx_step") (param $tx_dirty_ne_cur i32) (param $descriptor_owned i32) (result i32)
    ;; 0 stop, 1 reap one descriptor.
    (if (i32.eqz (local.get $tx_dirty_ne_cur)) (then (return (i32.const 0))))
    (if (local.get $descriptor_owned) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "pci_config_io_available") (param $is_x86_or_x86_64 i32) (result i32)
    (if (local.get $is_x86_or_x86_64) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "pci_address") (param $bus i32) (param $slot i32) (param $func i32) (param $offset i32) (result i32)
    (i32.or
      (i32.const 0x80000000)
      (i32.or
        (i32.shl (i32.and (local.get $bus) (i32.const 0xff)) (i32.const 16))
        (i32.or
          (i32.shl (i32.and (local.get $slot) (i32.const 0x1f)) (i32.const 11))
          (i32.or
            (i32.shl (i32.and (local.get $func) (i32.const 0x07)) (i32.const 8))
            (i32.and (local.get $offset) (i32.const 0xfc)))))))

  (func (export "pci_memory_bar_result") (param $bar i32) (result i32)
    ;; 0 usable 32-bit memory BAR, 1 empty/all-ones, 2 io BAR.
    (if (i32.or (i32.eqz (local.get $bar)) (i32.eq (local.get $bar) (i32.const -1))) (then (return (i32.const 1))))
    (if (i32.ne (i32.and (local.get $bar) (i32.const 1)) (i32.const 0)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "pci_bar_offset_step") (param $bar i32) (result i32)
    ;; 64-bit memory BAR consumes two slots.
    (if (i32.eq (i32.and (local.get $bar) (i32.const 6)) (i32.const 4)) (then (return (i32.const 8))))
    i32.const 4)
)