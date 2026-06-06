;; Status values: 0 ok, 1 unsupported, 2 short, 3 invalid.
  ;; Domains: 1 WiFi, 2 BLE.
  ;; Region kinds:
  ;;   1 blob absolute BSS, 2 GPIO, 3 IO_MUX, 4 RTC control, 5 PBUS/tx power,
  ;;   6 RF/frequency, 7 WiFi PHY, 8 WiFi modem/APB, 9 SPI2, 10 WiFi MAC,
  ;;   11 SYSTEM.
  ;; Blob section kinds:
  ;;   1 WiFi absolute BSS, 2 WiFi AP config, 3 WiFi init config,
  ;;   4 WiFi OS adapter, 5 BLE version, 6 BLE controller config,
  ;;   7 BLE OS adapter, 8 BLE advertising payload.

  (func (export "proto_standard_id") (result i32)
    i32.const 300094)

  (func (export "esp32s3_wifi_domain_id") (result i32)
    i32.const 1)

  (func (export "esp32s3_ble_domain_id") (result i32)
    i32.const 2)

  (func (export "esp32s3_domain_valid") (param $domain i32) (result i32)
    (i32.or
      (i32.eq (local.get $domain) (i32.const 1))
      (i32.eq (local.get $domain) (i32.const 2))))

  (func $range_contains (param $addr i32) (param $len i32) (param $start i32) (param $end i32) (result i32)
    (local $last i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 0))))
    (local.set $last
      (i32.add
        (local.get $addr)
        (i32.sub (local.get $len) (i32.const 1))))
    (if (i32.lt_u (local.get $last) (local.get $addr))
      (then (return (i32.const 0))))
    (i32.and
      (i32.ge_u (local.get $addr) (local.get $start))
      (i32.lt_u (local.get $last) (local.get $end))))

  (func $esp32s3_region_kind (export "esp32s3_region_kind") (param $addr i32) (result i32)
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x3fcef800) (i32.const 0x3fcf0000))
      (then (return (i32.const 1))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60004000) (i32.const 0x60004600))
      (then (return (i32.const 2))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60009000) (i32.const 0x6000a000))
      (then (return (i32.const 3))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60008000) (i32.const 0x60008100))
      (then (return (i32.const 4))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x600060c8) (i32.const 0x600061c0))
      (then (return (i32.const 5))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x6000e000) (i32.const 0x6000e180))
      (then (return (i32.const 6))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x6001c000) (i32.const 0x6001d000))
      (then (return (i32.const 7))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60026000) (i32.const 0x60027000))
      (then (return (i32.const 8))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60024000) (i32.const 0x60025000))
      (then (return (i32.const 9))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x60033000) (i32.const 0x60035200))
      (then (return (i32.const 10))))
    (if (call $range_contains (local.get $addr) (i32.const 1) (i32.const 0x600c0000) (i32.const 0x600c0100))
      (then (return (i32.const 11))))
    i32.const 0)

  (func $esp32s3_region_contains (export "esp32s3_region_contains") (param $kind i32) (param $addr i32) (param $len i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x3fcef800) (i32.const 0x3fcf0000)))))
    (if (i32.eq (local.get $kind) (i32.const 2))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60004000) (i32.const 0x60004600)))))
    (if (i32.eq (local.get $kind) (i32.const 3))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60009000) (i32.const 0x6000a000)))))
    (if (i32.eq (local.get $kind) (i32.const 4))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60008000) (i32.const 0x60008100)))))
    (if (i32.eq (local.get $kind) (i32.const 5))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x600060c8) (i32.const 0x600061c0)))))
    (if (i32.eq (local.get $kind) (i32.const 6))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x6000e000) (i32.const 0x6000e180)))))
    (if (i32.eq (local.get $kind) (i32.const 7))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x6001c000) (i32.const 0x6001d000)))))
    (if (i32.eq (local.get $kind) (i32.const 8))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60026000) (i32.const 0x60027000)))))
    (if (i32.eq (local.get $kind) (i32.const 9))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60024000) (i32.const 0x60025000)))))
    (if (i32.eq (local.get $kind) (i32.const 10))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x60033000) (i32.const 0x60035200)))))
    (if (i32.eq (local.get $kind) (i32.const 11))
      (then (return (call $range_contains (local.get $addr) (local.get $len) (i32.const 0x600c0000) (i32.const 0x600c0100)))))
    i32.const 0)

  (func $esp32s3_mmio_addr_valid (export "esp32s3_mmio_addr_valid") (param $addr i32) (param $len i32) (result i32)
    (local $kind i32)
    (if (i32.eq (local.get $len) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 4096))
      (then (return (i32.const 0))))
    (local.set $kind (call $esp32s3_region_kind (local.get $addr)))
    (if (i32.eqz (local.get $kind))
      (then (return (i32.const 0))))
    (call $esp32s3_region_contains (local.get $kind) (local.get $addr) (local.get $len)))

  (func (export "esp32s3_mmio_reg32_valid") (param $addr i32) (result i32)
    (if (i32.ne (i32.and (local.get $addr) (i32.const 3)) (i32.const 0))
      (then (return (i32.const 0))))
    (call $esp32s3_mmio_addr_valid (local.get $addr) (i32.const 4)))

  (func (export "esp32s3_blob_section_valid") (param $kind i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $kind) (i32.const 1))
      (i32.le_u (local.get $kind) (i32.const 8))))

  (func (export "esp32s3_blob_section_domain") (param $kind i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 4)))
      (then (return (i32.const 1))))
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 5)) (i32.le_u (local.get $kind) (i32.const 8)))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "esp32s3_blob_section_meta") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 0x800))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 256))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 0x1f2f3f4f))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 0xdeadbeaf))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 64))))
    (if (i32.eq (local.get $kind) (i32.const 6)) (then (return (i32.const 0x5a5aa5a5))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 0xfadebead))))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then (return (i32.const 32))))
    i32.const 0)

  (func (export "esp32s3_wifi_ap_config_field_offset") (param $field i32) (result i32)
    (if (i32.eq (local.get $field) (i32.const 1)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $field) (i32.const 2)) (then (return (i32.const 96))))
    (if (i32.eq (local.get $field) (i32.const 3)) (then (return (i32.const 97))))
    (if (i32.eq (local.get $field) (i32.const 4)) (then (return (i32.const 100))))
    (if (i32.eq (local.get $field) (i32.const 5)) (then (return (i32.const 105))))
    (if (i32.eq (local.get $field) (i32.const 6)) (then (return (i32.const 106))))
    (if (i32.eq (local.get $field) (i32.const 7)) (then (return (i32.const 109))))
    i32.const -1)

  (func (export "esp32s3_wifi_ap_config_valid") (param $ssid_len i32) (param $channel i32) (result i32)
    (i32.and
      (i32.and
        (i32.ge_u (local.get $ssid_len) (i32.const 1))
        (i32.le_u (local.get $ssid_len) (i32.const 32)))
      (i32.and
        (i32.ge_u (local.get $channel) (i32.const 1))
        (i32.le_u (local.get $channel) (i32.const 14)))))

  (func (export "esp32s3_wifi_init_status_kind") (param $status i32) (result i32)
    (if (i32.eq (local.get $status) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $status) (i32.const -1)) (then (return (i32.const 1))))
    (if (i32.and (i32.ge_s (local.get $status) (i32.const 10000)) (i32.lt_s (local.get $status) (i32.const 20000)))
      (then (return (i32.const 2))))
    (if (i32.and (i32.ge_s (local.get $status) (i32.const 20000)) (i32.lt_s (local.get $status) (i32.const 30000)))
      (then (return (i32.const 3))))
    (if (i32.and (i32.ge_s (local.get $status) (i32.const 30000)) (i32.lt_s (local.get $status) (i32.const 40000)))
      (then (return (i32.const 4))))
    (if (i32.and (i32.ge_s (local.get $status) (i32.const 40000)) (i32.lt_s (local.get $status) (i32.const 50000)))
      (then (return (i32.const 5))))
    i32.const 6)

  (func (export "esp32s3_ble_status_field_valid") (param $field i32) (param $value i32) (result i32)
    (if (i32.and (i32.ge_u (local.get $field) (i32.const 1)) (i32.le_u (local.get $field) (i32.const 4)))
      (then
        (return
          (i32.or
            (i32.eq (local.get $value) (i32.const 0))
            (i32.eq (local.get $value) (i32.const -2147483648))))))
    (if (i32.eq (local.get $field) (i32.const 5))
      (then (return (i32.ge_s (local.get $value) (i32.const 0)))))
    i32.const 0)

  (func (export "esp32s3_ble_version_byte_valid") (param $byte i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $byte) (i32.const 0x20))
      (i32.le_u (local.get $byte) (i32.const 0x7e))))

  (func (export "esp32s3_ble_hci_opcode_valid") (param $opcode i32) (result i32)
    (if (i32.eq (local.get $opcode) (i32.const 0x0c03)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 0x2005)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 0x2006)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 0x2008)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 0x200a)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 0x200c)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "esp32s3_ble_hci_packet_kind_valid") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 1))))
    i32.const 0)

  (func $esp32s3_wifi_raw_80211_len_valid (export "esp32s3_wifi_raw_80211_len_valid") (param $len i32) (result i32)
    (i32.and
      (i32.ge_u (local.get $len) (i32.const 24))
      (i32.le_u (local.get $len) (i32.const 2352))))

  (func (export "esp32s3_wifi_promisc_sig_payload_len") (param $sig_len i32) (result i32)
    (local $len i32)
    (local.set $len (i32.and (local.get $sig_len) (i32.const 0x0fff)))
    (if (i32.gt_u (local.get $len) (i32.const 4))
      (then (local.set $len (i32.sub (local.get $len) (i32.const 4)))))
    (if (call $esp32s3_wifi_raw_80211_len_valid (local.get $len))
      (then (return (local.get $len))))
    i32.const -1)
