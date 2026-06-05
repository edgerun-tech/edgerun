(module
  ;; Device type model semantics captured from edgerun-devices.
  ;;
  ;; Generic result codes use 0 for ok and nonzero for the first rejection.

  (func $bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (export "device_biometric_assurance" (func $device_biometric_assurance))
  (func $device_biometric_assurance
    (param $user_present i32)
    (param $verified i32)
    (param $hardware_protected i32)
    (result i32)
    ;; none=0 presence=1 biometric=2 hardware-protected=3.
    local.get $verified
    call $bool
    local.get $hardware_protected
    call $bool
    i32.and
    if (result i32)
      i32.const 3
    else
      local.get $verified
      call $bool
      if (result i32)
        i32.const 2
      else
        local.get $user_present
        call $bool
        if (result i32)
          i32.const 1
        else
          i32.const 0
        end
      end
    end)

  (export "device_biometric_satisfies" (func $device_biometric_satisfies))
  (func $device_biometric_satisfies
    (param $strength i32)
    (param $minimum i32)
    (result i32)
    local.get $strength
    local.get $minimum
    i32.ge_s)

  (export "device_display_update_result" (func $device_display_update_result))
  (func $device_display_update_result
    (param $has_width i32)
    (param $width i32)
    (param $has_height i32)
    (param $height i32)
    (result i32)
    ;; width-zero=1 height-zero=2.
    local.get $has_width
    call $bool
    local.get $width
    i32.const 0
    i32.eq
    i32.and
    if (result i32)
      i32.const 1
    else
      local.get $has_height
      call $bool
      local.get $height
      i32.const 0
      i32.eq
      i32.and
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "device_audio_capture_result" (func $device_audio_capture_result))
  (func $device_audio_capture_result
    (param $duration_ms i32)
    (param $sample_rate_hz i32)
    (param $channels i32)
    (result i32)
    local.get $duration_ms
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $sample_rate_hz
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $channels
        i32.const 0
        i32.eq
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "device_audio_playback_result" (func $device_audio_playback_result))
  (func $device_audio_playback_result
    (param $duration_ms i32)
    (param $sample_rate_hz i32)
    (param $channels i32)
    (param $audio_bytes_len i32)
    (param $has_gain i32)
    (param $gain_percent i32)
    (param $has_target_level i32)
    (param $target_level_percent i32)
    (result i32)
    ;; duration=1 rate=2 channels=3 empty=4 gain=5 level=6.
    local.get $duration_ms
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $sample_rate_hz
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $channels
        i32.const 0
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $audio_bytes_len
          i32.const 0
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $has_gain
            call $bool
            local.get $gain_percent
            i32.const 400
            i32.gt_s
            i32.and
            if (result i32)
              i32.const 5
            else
              local.get $has_target_level
              call $bool
              local.get $target_level_percent
              i32.const 100
              i32.gt_s
              i32.and
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

  (export "device_wifi_ap_config_result" (func $device_wifi_ap_config_result))
  (func $device_wifi_ap_config_result
    (param $trimmed_ssid_len i32)
    (param $raw_ssid_len i32)
    (param $secure i32)
    (result i32)
    ;; empty=1 too-long=2 secure-unsupported=3.
    local.get $trimmed_ssid_len
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $raw_ssid_len
      i32.const 32
      i32.gt_s
      if (result i32)
        i32.const 2
      else
        local.get $secure
        call $bool
        if (result i32)
          i32.const 3
        else
          i32.const 0
        end
      end
    end)

  (export "device_input_read_result" (func $device_input_read_result))
  (func $device_input_read_result (param $max_events i32) (result i32)
    local.get $max_events
    i32.const 0
    i32.gt_s)

  (export "device_npu_workload_result" (func $device_npu_workload_result))
  (func $device_npu_workload_result (param $input_bytes_len i32) (result i32)
    local.get $input_bytes_len
    i32.const 0
    i32.gt_s)

  (export "device_gpu_vendor" (func $device_gpu_vendor))
  (func $device_gpu_vendor (param $vendor_id i32) (result i32)
    ;; unknown=0 amd=1 intel=2 nvidia=3 qualcomm=4 apple=5 arm=6 matrox=7 aspeed=8 virtio=9 microsoft=10.
    local.get $vendor_id
    i32.const 0x1002
    i32.eq
    local.get $vendor_id
    i32.const 0x1022
    i32.eq
    i32.or
    if (result i32)
      i32.const 1
    else
      local.get $vendor_id
      i32.const 0x8086
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $vendor_id
        i32.const 0x10de
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $vendor_id
          i32.const 0x17cb
          i32.eq
          local.get $vendor_id
          i32.const 0x5143
          i32.eq
          i32.or
          if (result i32)
            i32.const 4
          else
            local.get $vendor_id
            i32.const 0x106b
            i32.eq
            if (result i32)
              i32.const 5
            else
              local.get $vendor_id
              i32.const 0x13b5
              i32.eq
              if (result i32)
                i32.const 6
              else
                local.get $vendor_id
                i32.const 0x102b
                i32.eq
                if (result i32)
                  i32.const 7
                else
                  local.get $vendor_id
                  i32.const 0x1a03
                  i32.eq
                  if (result i32)
                    i32.const 8
                  else
                    local.get $vendor_id
                    i32.const 0x1af4
                    i32.eq
                    if (result i32)
                      i32.const 9
                    else
                      local.get $vendor_id
                      i32.const 0x1414
                      i32.eq
                      if (result i32)
                        i32.const 10
                      else
                        i32.const 0
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end
    end)

  (export "device_nfc_technology_code" (func $device_nfc_technology_code))
  (func $device_nfc_technology_code (param $token_code i32) (result i32)
    ;; caller tokenizes aliases to nfc-a=1 b=2 f=3 v=4 iso-dep=5 mifare=6.
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

  (export "device_camera_enroll_result" (func $device_camera_enroll_result))
  (func $device_camera_enroll_result
    (param $trimmed_label_len i32)
    (param $samples_required i32)
    (result i32)
    local.get $trimmed_label_len
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $samples_required
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "device_liveness_challenge_result" (func $device_liveness_challenge_result))
  (func $device_liveness_challenge_result
    (param $timeout_ms i32)
    (param $require_rgb i32)
    (param $require_infrared i32)
    (result i32)
    local.get $timeout_ms
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $require_rgb
      call $bool
      local.get $require_infrared
      call $bool
      i32.or
      i32.eqz
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "device_default_liveness_field" (func $device_default_liveness_field))
  (func $device_default_liveness_field (param $field i32) (result i32)
    ;; kind=0 timeout=1 rgb=2 infrared=3.
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        i32.const 1500
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 1
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            i32.const 0
          else
            i32.const 0
          end
        end
      end
    end)

  (export "device_fingerprint_enroll_result" (func $device_fingerprint_enroll_result))
  (func $device_fingerprint_enroll_result
    (param $trimmed_label_len i32)
    (param $samples_required i32)
    (result i32)
    local.get $trimmed_label_len
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $samples_required
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "device_quectel_model_code" (func $device_quectel_model_code))
  (func $device_quectel_model_code (param $model i32) (result i32)
    ;; CN=1 AU=2 EU=3 EL=4.
    local.get $model
    i32.const 1
    i32.ge_s
    local.get $model
    i32.const 4
    i32.le_s
    i32.and
    if (result i32)
      local.get $model
    else
      i32.const 0
    end)

  (export "device_quectel_default_config" (func $device_quectel_default_config))
  (func $device_quectel_default_config (param $field i32) (result i32)
    ;; dta-disabled=0 default-apn-internet=1 pin-none=2 network-auto=4 state-unknown=0.
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        i32.const 1
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 2
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            i32.const 4
          else
            i32.const 0
          end
        end
      end
    end)

  (export "device_quectel_pin_valid" (func $device_quectel_pin_valid))
  (func $device_quectel_pin_valid
    (param $has_pin i32)
    (param $pin_len i32)
    (param $all_ascii_digits i32)
    (result i32)
    local.get $has_pin
    call $bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $pin_len
      i32.const 4
      i32.eq
      local.get $all_ascii_digits
      call $bool
      i32.and
    end)

  (export "device_quectel_runtime_api_result" (func $device_quectel_runtime_api_result))
  (func $device_quectel_runtime_api_result (result i32)
    ;; direct APIs always reject because AT execution is runtime-owned.
    i32.const 1)

  (export "device_quectel_at_command_kind" (func $device_quectel_at_command_kind))
  (func $device_quectel_at_command_kind (param $command_code i32) (result i32)
    ;; AT=1 CREG=2 COPS=3 CSQ=4 ATI=5 CIMI=6 CFUN1=7 CFUN4=8.
    local.get $command_code
    i32.const 1
    i32.ge_s
    local.get $command_code
    i32.const 8
    i32.le_s
    i32.and
    if (result i32)
      local.get $command_code
    else
      i32.const 0
    end)

  (export "device_quectel_state_after_at" (func $device_quectel_state_after_at))
  (func $device_quectel_state_after_at (param $command_code i32) (result i32)
    ;; unknown=0 off=1 on=2 registered=5, otherwise unchanged=99.
    local.get $command_code
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $command_code
      i32.const 2
      i32.eq
      local.get $command_code
      i32.const 7
      i32.eq
      i32.or
      if (result i32)
        i32.const 5
      else
        local.get $command_code
        i32.const 8
        i32.eq
        if (result i32)
          i32.const 1
        else
          i32.const 99
        end
      end
    end)

  (export "device_quectel_power_spec_mv" (func $device_quectel_power_spec_mv))
  (func $device_quectel_power_spec_mv
    (param $supply i32)
    (param $field i32)
    (result i32)
    ;; supply: bb=1 rf=2 ext=3; field min=0 typ=1 max=2 peak=3 avg=4.
    local.get $supply
    i32.const 3
    i32.eq
    if (result i32)
      local.get $field
      i32.const 0
      i32.eq
      if (result i32)
        i32.const 1710
      else
        local.get $field
        i32.const 1
        i32.eq
        if (result i32)
          i32.const 1800
        else
          local.get $field
          i32.const 2
          i32.eq
          if (result i32)
            i32.const 1890
          else
            local.get $field
            i32.const 3
            i32.eq
            if (result i32)
              i32.const 100
            else
              i32.const 50
            end
          end
        end
      end
    else
      local.get $field
      i32.const 3
      i32.eq
      if (result i32)
        local.get $supply
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 1800
        else
          i32.const 800
        end
      else
        local.get $field
        i32.const 4
        i32.eq
        if (result i32)
          local.get $supply
          i32.const 2
          i32.eq
          if (result i32)
            i32.const 500
          else
            i32.const 300
          end
        else
          local.get $field
          i32.const 0
          i32.eq
          if (result i32)
            i32.const 3300
          else
            local.get $field
            i32.const 1
            i32.eq
            if (result i32)
              i32.const 3700
            else
              i32.const 4300
            end
          end
        end
      end
    end)

  (export "device_quectel_uart_default" (func $device_quectel_uart_default))
  (func $device_quectel_uart_default (param $field i32) (result i32)
    ;; port=0 baud=1 data=2 stop=3 parity=4 flow=5.
    local.get $field
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $field
      i32.const 1
      i32.eq
      if (result i32)
        i32.const 115200
      else
        local.get $field
        i32.const 2
        i32.eq
        if (result i32)
          i32.const 8
        else
          local.get $field
          i32.const 3
          i32.eq
          if (result i32)
            i32.const 1
          else
            i32.const 0
          end
        end
      end
    end)

  (export "device_quectel_adc_divider_mv" (func $device_quectel_adc_divider_mv))
  (func $device_quectel_adc_divider_mv (param $ohms i32) (result i32)
    i32.const 1800
    local.get $ohms
    i32.mul
    i32.const 1800
    local.get $ohms
    i32.add
    i32.div_u)

  (export "device_quectel_pin_function" (func $device_quectel_pin_function))
  (func $device_quectel_pin_function (param $pin i32) (result i32)
    ;; power=1 reset=2 sim=3 uart=4 gpio=5 adc=6 antenna=7 rf=8 unknown=0.
    local.get $pin
    i32.const 1
    i32.ge_s
    local.get $pin
    i32.const 3
    i32.le_s
    i32.and
    if (result i32)
      i32.const 1
    else
      local.get $pin
      i32.const 4
      i32.eq
      local.get $pin
      i32.const 5
      i32.eq
      i32.or
      if (result i32)
        i32.const 2
      else
        local.get $pin
        i32.const 8
        i32.ge_s
        local.get $pin
        i32.const 12
        i32.le_s
        i32.and
        if (result i32)
          i32.const 3
        else
          local.get $pin
          i32.const 13
          i32.ge_s
          local.get $pin
          i32.const 18
          i32.le_s
          i32.and
          if (result i32)
            i32.const 4
          else
            local.get $pin
            i32.const 19
            i32.ge_s
            local.get $pin
            i32.const 20
            i32.le_s
            i32.and
            if (result i32)
              i32.const 6
            else
              local.get $pin
              i32.const 23
              i32.ge_s
              local.get $pin
              i32.const 27
              i32.le_s
              i32.and
              if (result i32)
                i32.const 5
              else
                local.get $pin
                i32.const 28
                i32.ge_s
                if (result i32)
                  i32.const 7
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)
)
