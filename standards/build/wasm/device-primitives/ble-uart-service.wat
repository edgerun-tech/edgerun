(module
  (memory (export "memory") 1)

  (global $ble_uart_abi i32 (i32.const 1))

  ;; Nordic UART Service UUIDs
  (global $nus_service_uuid_hi i64 (i64.const 0x6E400001B5A3F393))
  (global $nus_service_uuid_lo i64 (i64.const 0xE0A9E50E24DCCA9E))
  (global $nus_tx_uuid_hi i64 (i64.const 0x6E400002B5A3F393))
  (global $nus_tx_uuid_lo i64 (i64.const 0xE0A9E50E24DCCA9E))
  (global $nus_rx_uuid_hi i64 (i64.const 0x6E400003B5A3F393))
  (global $nus_rx_uuid_lo i64 (i64.const 0xE0A9E50E24DCCA9E))

  (func (export "ble_uart_abi_version") (result i32)
    global.get $ble_uart_abi
  )

  ;; Store NUS service UUID at given offset in memory
  (func (export "ble_uart_get_service_uuid")
    (param $offset i32)
    (i64.store offset=0 (local.get $offset) (global.get $nus_service_uuid_hi))
    (i64.store offset=8 (local.get $offset) (global.get $nus_service_uuid_lo))
  )

  ;; Store TX characteristic UUID at given offset in memory
  (func (export "ble_uart_get_tx_uuid")
    (param $offset i32)
    (i64.store offset=0 (local.get $offset) (global.get $nus_tx_uuid_hi))
    (i64.store offset=8 (local.get $offset) (global.get $nus_tx_uuid_lo))
  )

  ;; Store RX characteristic UUID at given offset in memory
  (func (export "ble_uart_get_rx_uuid")
    (param $offset i32)
    (i64.store offset=0 (local.get $offset) (global.get $nus_rx_uuid_hi))
    (i64.store offset=8 (local.get $offset) (global.get $nus_rx_uuid_lo))
  )

  ;; Validate BLE write payload size against MTU
  (func (export "ble_uart_validate_payload")
    (param $data_len i32)
    (param $mtu i32)
    (result i32)
    (if (i32.eqz (local.get $data_len))
      (then (return (i32.const 1)))
    )
    (if (i32.gt_u (local.get $data_len) (local.get $mtu))
      (then (return (i32.const 2)))
    )
    (i32.const 0)
  )

  ;; Validate security level for BLE characteristic access
  (func (export "ble_uart_validate_security")
    (param $security_level i32)
    (param $require_encryption i32)
    (result i32)
    (if (i32.and (local.get $require_encryption) (i32.eqz (local.get $security_level)))
      (then (return (i32.const 1)))
    )
    (i32.const 0)
  )
)
