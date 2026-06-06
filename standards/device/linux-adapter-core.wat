(module
  (import "edgerun-core" "memory" (memory 1))
;; Linux adapter semantics captured from sysfs, USB/PCI/netif, nl80211 Wi-Fi,
  ;; evdev input, and Bluetooth GATT/HCI/Linux L2CAP adapter crates.
  ;;
  ;; Common enum codes:
  ;; - link: unknown=0 up=1 down=2 dormant=3 lowerlayerdown=4 notpresent=5 testing=6
  ;; - net kind: unknown=0 ethernet=1 loopback=2 wireless=3 bridge=4 vlan=5 tunnel=6 virtual=7
  ;; - input kind: other=0 touch=1 pen=2 pointer=3 gamepad=4 switch=5 keyboard=6

  (func $m208bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $m208min (param $a i32) (param $b i32) (result i32)
    local.get $a
    local.get $b
    i32.lt_s
    if (result i32)
      local.get $a
    else
      local.get $b
    end)

  (export "sysfs_hex_prefix_len" (func $sysfs_hex_prefix_len))
  (func $sysfs_hex_prefix_len
    (param $first i32)
    (param $second i32)
    (result i32)
    local.get $first
    i32.const 48
    i32.eq
    local.get $second
    i32.const 120
    i32.eq
    i32.and
    if (result i32)
      i32.const 2
    else
      i32.const 0
    end)

  (export "sysfs_bool_flag" (func $sysfs_bool_flag))
  (func $sysfs_bool_flag (param $token_code i32) (result i32)
    ;; true tokens=1, false tokens=2, unknown=0.
    local.get $token_code
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $token_code
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "sysfs_display_refresh_millihz" (func $sysfs_display_refresh_millihz))
  (func $sysfs_display_refresh_millihz
    (param $has_refresh i32)
    (param $hz_times_1000 i32)
    (result i32)
    local.get $has_refresh
    call $m208bool
    if (result i32)
      local.get $hz_times_1000
    else
      i32.const 60000
    end)

  (export "sysfs_pci_address_shape" (func $sysfs_pci_address_shape))
  (func $sysfs_pci_address_shape
    (param $len i32)
    (param $c4 i32)
    (param $c7 i32)
    (param $c10 i32)
    (result i32)
    local.get $len
    i32.const 12
    i32.eq
    local.get $c4
    i32.const 58
    i32.eq
    i32.and
    local.get $c7
    i32.const 58
    i32.eq
    i32.and
    local.get $c10
    i32.const 46
    i32.eq
    i32.and)

  (export "sysfs_parent_valid" (func $sysfs_parent_valid))
  (func $sysfs_parent_valid
    (param $has_parent i32)
    (param $parent_known i32)
    (result i32)
    local.get $has_parent
    call $m208bool
    local.get $parent_known
    call $m208bool
    i32.and)

  (export "usb_speed_kind" (func $usb_speed_kind))
  (func $usb_speed_kind (param $speed_mbps i32) (result i32)
    ;; unknown=0 low=1 full=2 high=3 super=4 super-plus=5.
    local.get $speed_mbps
    i32.const 15
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $speed_mbps
      i32.const 12
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $speed_mbps
        i32.const 480
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $speed_mbps
          i32.const 5000
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $speed_mbps
            i32.const 10000
            i32.eq
            local.get $speed_mbps
            i32.const 20000
            i32.eq
            i32.or
            if (result i32)
              i32.const 5
            else
              i32.const 0
            end
          end
        end
      end
    end)

  (export "usb_device_dir_valid" (func $usb_device_dir_valid))
  (func $usb_device_dir_valid
    (param $is_dir i32)
    (param $name_has_dash i32)
    (param $has_id_vendor i32)
    (param $has_busnum i32)
    (result i32)
    local.get $is_dir
    call $m208bool
    local.get $name_has_dash
    call $m208bool
    i32.and
    local.get $has_id_vendor
    call $m208bool
    local.get $has_busnum
    call $m208bool
    i32.or
    i32.and)

  (export "usb_interface_dir_valid" (func $usb_interface_dir_valid))
  (func $usb_interface_dir_valid
    (param $is_dir i32)
    (param $name_has_colon i32)
    (param $has_interface_class i32)
    (result i32)
    local.get $is_dir
    call $m208bool
    local.get $name_has_colon
    call $m208bool
    i32.and
    local.get $has_interface_class
    call $m208bool
    i32.and)

  (export "usb_parent_depth_action" (func $usb_parent_depth_action))
  (func $usb_parent_depth_action
    (param $has_dash i32)
    (param $port_dot_count i32)
    (result i32)
    ;; none=0 parent=1.
    local.get $has_dash
    call $m208bool
    local.get $port_dot_count
    i32.const 0
    i32.gt_s
    i32.and)

  (export "netif_link_state" (func $netif_link_state))
  (func $netif_link_state (param $token_code i32) (result i32)
    ;; caller maps exact strings to: up=1 down=2 dormant=3 lower=4 notpresent=5 testing=6.
    local.get $token_code
    i32.const 1
    i32.ge_s
    local.get $token_code
    i32.const 6
    i32.le_s
    i32.and
    if (result i32)
      local.get $token_code
    else
      i32.const 0
    end)

  (export "netif_kind_from_sysfs" (func $netif_kind_from_sysfs))
  (func $netif_kind_from_sysfs
    (param $has_wireless_dir i32)
    (param $has_bridge_dir i32)
    (param $has_tun_flags i32)
    (param $arphrd_type i32)
    (param $name_prefix_kind i32)
    (param $has_device_link i32)
    (result i32)
    ;; prefix kind: none=0 br=1 vlan-or-dot=2.
    local.get $has_wireless_dir
    call $m208bool
    if (result i32)
      i32.const 3
    else
      local.get $has_bridge_dir
      call $m208bool
      if (result i32)
        i32.const 4
      else
        local.get $has_tun_flags
        call $m208bool
        if (result i32)
          i32.const 6
        else
          local.get $arphrd_type
          i32.const 772
          i32.eq
          if (result i32)
            i32.const 2
          else
            local.get $arphrd_type
            i32.const 801
            i32.ge_s
            local.get $arphrd_type
            i32.const 803
            i32.le_s
            i32.and
            if (result i32)
              i32.const 3
            else
              local.get $arphrd_type
              i32.const 768
              i32.eq
              local.get $arphrd_type
              i32.const 769
              i32.eq
              i32.or
              if (result i32)
                i32.const 6
              else
                local.get $arphrd_type
                i32.const 1
                i32.eq
                if (result i32)
                  local.get $name_prefix_kind
                  i32.const 1
                  i32.eq
                  if (result i32)
                    i32.const 4
                  else
                    local.get $name_prefix_kind
                    i32.const 2
                    i32.eq
                    if (result i32)
                      i32.const 5
                    else
                      local.get $has_device_link
                      call $m208bool
                      if (result i32)
                        i32.const 1
                      else
                        i32.const 7
                      end
                    end
                  end
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (export "wifi_rfkill_state" (func $wifi_rfkill_state))
  (func $wifi_rfkill_state
    (param $matches_wlan_phy i32)
    (param $soft_blocked i32)
    (param $hard_blocked i32)
    (result i32)
    ;; unknown=0 enabled=1 blocked=2.
    local.get $matches_wlan_phy
    call $m208bool
    i32.eqz
    if (result i32)
      i32.const 0
    else
      local.get $soft_blocked
      call $m208bool
      local.get $hard_blocked
      call $m208bool
      i32.or
      if (result i32)
        i32.const 2
      else
        i32.const 1
      end
    end)

  (export "wifi_power_fallback" (func $wifi_power_fallback))
  (func $wifi_power_fallback (param $admin_state i32) (result i32)
    ;; admin up=1 -> enabled, down=2 -> disabled, else unknown.
    local.get $admin_state
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $admin_state
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 3
      else
        i32.const 0
      end
    end)

  (export "wifi_mode_to_nl80211" (func $wifi_mode_to_nl80211))
  (func $wifi_mode_to_nl80211 (param $mode i32) (result i32)
    ;; unknown=0 client=1 ap=2 adhoc=3 monitor=4 -> nl80211 numbers.
    local.get $mode
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $mode
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 3
      else
        local.get $mode
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 1
        else
          local.get $mode
          i32.const 4
          i32.eq
          if (result i32)
            i32.const 6
          else
            i32.const 0
          end
        end
      end
    end)

  (export "wifi_mode_from_nl80211" (func $wifi_mode_from_nl80211))
  (func $wifi_mode_from_nl80211 (param $iftype i32) (result i32)
    local.get $iftype
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 3
    else
      local.get $iftype
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 1
      else
        local.get $iftype
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 2
        else
          local.get $iftype
          i32.const 6
          i32.eq
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "wifi_nla_aligned_len" (func $wifi_nla_aligned_len))
  (func $wifi_nla_aligned_len (param $payload_len i32) (result i32)
    local.get $payload_len
    i32.const 4
    i32.add
    i32.const 3
    i32.add
    i32.const -4
    i32.and)

  (export "wifi_nested_attr_type" (func $wifi_nested_attr_type))
  (func $wifi_nested_attr_type (param $attr_type i32) (result i32)
    local.get $attr_type
    i32.const 0x8000
    i32.or)

  (export "wifi_scan_ssid_attr_count" (func $wifi_scan_ssid_attr_count))
  (func $wifi_scan_ssid_attr_count (param $provided_count i32) (result i32)
    local.get $provided_count
    i32.const 0
    i32.gt_s
    if (result i32)
      local.get $provided_count
    else
      i32.const 1
    end)

  (export "wifi_scan_poll_decision" (func $wifi_scan_poll_decision))
  (func $wifi_scan_poll_decision
    (param $attempts i32)
    (param $dump_ok i32)
    (result i32)
    ;; return=1 retry=2 fail=3.
    local.get $dump_ok
    call $m208bool
    if (result i32)
      i32.const 1
    else
      local.get $attempts
      i32.const 20
      i32.lt_s
      if (result i32)
        i32.const 2
      else
        i32.const 3
      end
    end)

  (export "wifi_ie_security_seen" (func $wifi_ie_security_seen))
  (func $wifi_ie_security_seen
    (param $ie_type i32)
    (param $vendor_wpa_oui i32)
    (result i32)
    local.get $ie_type
    i32.const 48
    i32.eq
    local.get $ie_type
    i32.const 221
    i32.eq
    local.get $vendor_wpa_oui
    call $m208bool
    i32.and
    i32.or)

  (export "wifi_bss_observation_valid" (func $wifi_bss_observation_valid))
  (func $wifi_bss_observation_valid (param $ssid_len i32) (result i32)
    local.get $ssid_len
    i32.const 0
    i32.gt_s)

  (export "wifi_signal_dbm" (func $wifi_signal_dbm))
  (func $wifi_signal_dbm (param $signal_mbm i32) (result i32)
    local.get $signal_mbm
    i32.const 100
    i32.div_s)

  (export "wifi_rsn_ie_len" (func $wifi_rsn_ie_len))
  (func $wifi_rsn_ie_len (result i32)
    i32.const 22)

  (export "wifi_connect_mode" (func $wifi_connect_mode))
  (func $wifi_connect_mode (param $has_pmk i32) (result i32)
    ;; kernel-handshake=1 userspace-control-port=2.
    local.get $has_pmk
    call $m208bool
    if (result i32)
      i32.const 1
    else
      i32.const 2
    end)

  (export "wifi_eapol_parse_result" (func $wifi_eapol_parse_result))
  (func $wifi_eapol_parse_result
    (param $frame_len i32)
    (param $descriptor_type i32)
    (param $key_data_len i32)
    (result i32)
    ;; short=-1 not-rsn=-2 otherwise copied key-data length, truncated to 0 if absent.
    local.get $frame_len
    i32.const 81
    i32.lt_s
    if (result i32)
      i32.const -1
    else
      local.get $descriptor_type
      i32.const 2
      i32.ne
      if (result i32)
        i32.const -2
      else
        local.get $frame_len
        i32.const 99
        local.get $key_data_len
        i32.add
        i32.ge_s
        if (result i32)
          local.get $key_data_len
        else
          i32.const 0
        end
      end
    end)

  (export "wifi_eapol_message_type" (func $wifi_eapol_message_type))
  (func $wifi_eapol_message_type (param $key_info i32) (result i32)
    ;; message1=1 message2=2 message3=3 message4=4.
    (local $ack i32)
    (local $mic i32)
    (local $secure i32)
    local.get $key_info
    i32.const 128
    i32.and
    call $m208bool
    local.set $ack
    local.get $key_info
    i32.const 256
    i32.and
    call $m208bool
    local.set $mic
    local.get $key_info
    i32.const 512
    i32.and
    call $m208bool
    local.set $secure
    local.get $ack
    local.get $mic
    i32.eqz
    i32.and
    local.get $secure
    i32.eqz
    i32.and
    if (result i32)
      i32.const 1
    else
      local.get $ack
      i32.eqz
      local.get $mic
      i32.and
      local.get $secure
      i32.eqz
      i32.and
      if (result i32)
        i32.const 2
      else
        local.get $ack
        local.get $mic
        i32.and
        local.get $secure
        i32.and
        if (result i32)
          i32.const 3
        else
          local.get $ack
          i32.eqz
          local.get $mic
          i32.and
          local.get $secure
          i32.and
          if (result i32)
            i32.const 4
          else
            i32.const 1
          end
        end
      end
    end)

  (export "wifi_eapol_is_pairwise" (func $wifi_eapol_is_pairwise))
  (func $wifi_eapol_is_pairwise (param $key_info i32) (result i32)
    local.get $key_info
    i32.const 8
    i32.and
    call $m208bool)

  (export "wifi_ptk_order_bit" (func $wifi_ptk_order_bit))
  (func $wifi_ptk_order_bit
    (param $auth_lt_supplicant i32)
    (param $anonce_lt_snonce i32)
    (result i32)
    ;; bit0 address order, bit1 nonce order.
    local.get $auth_lt_supplicant
    call $m208bool
    local.get $anonce_lt_snonce
    call $m208bool
    i32.const 1
    i32.shl
    i32.or)

  (export "wifi_eapol_mic_offset" (func $wifi_eapol_mic_offset))
  (func $wifi_eapol_mic_offset (result i32)
    i32.const 81)

  (export "evdev_product_field_count" (func $evdev_product_field_count))
  (func $evdev_product_field_count (param $slash_parts i32) (result i32)
    local.get $slash_parts
    i32.const 4
    call $m208min)

  (export "evdev_bitmap_word_index" (func $evdev_bitmap_word_index))
  (func $evdev_bitmap_word_index (param $bit i32) (result i32)
    local.get $bit
    i32.const 64
    i32.div_u)

  (export "evdev_bitmap_bit_mask_low" (func $evdev_bitmap_bit_mask_low))
  (func $evdev_bitmap_bit_mask_low (param $bit i32) (result i32)
    i32.const 1
    local.get $bit
    i32.const 31
    i32.and
    i32.shl)

  (export "evdev_classify" (func $evdev_classify))
  (func $evdev_classify
    (param $has_touch i32)
    (param $has_pen i32)
    (param $has_pointer i32)
    (param $has_gamepad i32)
    (param $has_lid_switch i32)
    (param $has_keyboard i32)
    (result i32)
    local.get $has_touch
    call $m208bool
    if (result i32)
      i32.const 1
    else
      local.get $has_pen
      call $m208bool
      if (result i32)
        i32.const 2
      else
        local.get $has_pointer
        call $m208bool
        if (result i32)
          i32.const 3
        else
          local.get $has_gamepad
          call $m208bool
          if (result i32)
            i32.const 4
          else
            local.get $has_lid_switch
            call $m208bool
            if (result i32)
              i32.const 5
            else
              local.get $has_keyboard
              call $m208bool
              if (result i32)
                i32.const 6
              else
                i32.const 0
              end
            end
          end
        end
      end
    end)

  (export "evdev_event_kind" (func $evdev_event_kind))
  (func $evdev_event_kind (param $ty i32) (result i32)
    ;; syn=1 key=2 rel=3 abs=4 switch=5 misc=6 other=0.
    local.get $ty
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $ty
      i32.const 1
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $ty
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $ty
          i32.const 3
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $ty
            i32.const 5
            i32.eq
            if (result i32)
              i32.const 5
            else
              local.get $ty
              i32.const 4
              i32.eq
              if (result i32)
                i32.const 6
              else
                i32.const 0
              end
            end
          end
        end
      end
    end)

  (export "evdev_read_loop_decision" (func $evdev_read_loop_decision))
  (func $evdev_read_loop_decision
    (param $poll_result i32)
    (param $poll_error_bits i32)
    (param $read_result i32)
    (result i32)
    ;; continue=0 timeout=1 fd-error=2 eof=3 got-data=4 would-block=5.
    local.get $poll_result
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $poll_error_bits
      i32.const 0
      i32.ne
      if (result i32)
        i32.const 2
      else
        local.get $read_result
        i32.const 0
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $read_result
          i32.const -11
          i32.eq
          if (result i32)
            i32.const 5
          else
            i32.const 4
          end
        end
      end
    end)

  (export "gatt_address_kind_from_u8" (func $gatt_address_kind_from_u8))
  (func $gatt_address_kind_from_u8 (param $value i32) (result i32)
    ;; unknown=0 public=1 random=2 anonymous=3.
    local.get $value
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $value
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "gatt_address_kind_to_u8" (func $gatt_address_kind_to_u8))
  (func $gatt_address_kind_to_u8 (param $kind i32) (result i32)
    local.get $kind
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $kind
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $kind
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "gatt_service_kind" (func $gatt_service_kind))
  (func $gatt_service_kind (param $uuid16 i32) (result i32)
    ;; device-information=1 battery=2 other=0.
    local.get $uuid16
    i32.const 0x180a
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $uuid16
      i32.const 0x180f
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "gatt_characteristic_access" (func $gatt_characteristic_access))
  (func $gatt_characteristic_access
    (param $has_read i32)
    (param $has_write i32)
    (param $has_write_no_response i32)
    (param $has_notify i32)
    (param $has_indicate i32)
    (result i32)
    ;; bit0 readable, bit1 writable, bit2 notifiable.
    local.get $has_read
    call $m208bool
    local.get $has_write
    call $m208bool
    local.get $has_write_no_response
    call $m208bool
    i32.or
    i32.const 1
    i32.shl
    i32.or
    local.get $has_notify
    call $m208bool
    local.get $has_indicate
    call $m208bool
    i32.or
    i32.const 2
    i32.shl
    i32.or)

  (export "gatt_connection_state_connected" (func $gatt_connection_state_connected))
  (func $gatt_connection_state_connected (param $state i32) (result i32)
    ;; disconnected=0 connecting=1 connected=2 encrypting=3 encrypted=4 disconnecting=5.
    local.get $state
    i32.const 2
    i32.ge_s
    local.get $state
    i32.const 4
    i32.le_s
    i32.and)

  (export "gatt_att_error_class" (func $gatt_att_error_class))
  (func $gatt_att_error_class (param $code i32) (result i32)
    ;; invalid-offset=1 invalid-length=2 not-permitted=3 read=4 write=5 auth=6 timeout=9 not-connected=10 att=99.
    local.get $code
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $code
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $code
        i32.const 3
        i32.ge_s
        local.get $code
        i32.const 6
        i32.le_s
        i32.and
        if (result i32)
          local.get $code
        else
          local.get $code
          i32.const 9
          i32.eq
          local.get $code
          i32.const 10
          i32.eq
          i32.or
          if (result i32)
            local.get $code
          else
            i32.const 99
          end
        end
      end
    end)

  (export "gatt_error_category" (func $gatt_error_category))
  (func $gatt_error_category (param $error_kind i32) (result i32)
    ;; att=1 connection=2 authentication=3 other=0.
    local.get $error_kind
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $error_kind
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $error_kind
        i32.const 3
        i32.eq
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "hci_default_fast_param" (func $hci_default_fast_param))
  (func $hci_default_fast_param (param $field i32) (result i32)
    ;; scan_interval/window/peer_type/min/max/latency/supervision.
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0x0060
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        i32.const 0x0030
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 0x01
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            i32.const 0x0018
          else
            local.get $field
            i32.const 4
            i32.eq
            if (result i32)
              i32.const 0x0028
            else
              local.get $field
              i32.const 5
              i32.eq
              if (result i32)
                i32.const 0
              else
                local.get $field
                i32.const 6
                i32.eq
                if (result i32)
                  i32.const 0x01c0
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (export "l2cap_sockaddr_mode" (func $l2cap_sockaddr_mode))
  (func $l2cap_sockaddr_mode (param $att i32) (result i32)
    ;; ATT uses fixed CID 4 and psm 0. Generic device uses CID 0 and caller PSM.
    local.get $att
    call $m208bool
    if (result i32)
      i32.const 4
    else
      i32.const 0
    end)
)