(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(func (export "proto_standard_id") (result i32)
    i32.const 300102)

  ;; Status values: 0 ok, 1 input_short, 2 output_short, 3 invalid,
  ;; 4 invalid_cookie, 5 invalid_option_length.
  ;; Packed i64 return: low u32 status, high u32 value or next_offset.

  (func $m74read_u16_be (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 8
    i32.shl
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.or)

  (func $m74read_u32_be (param $ptr i32) (result i32)
    local.get $ptr
    i32.load8_u
    i32.const 24
    i32.shl
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 16
    i32.shl
    i32.or
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 8
    i32.shl
    i32.or
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.or)

  (func (export "dhcp_magic_cookie") (result i32)
    i32.const 0x63825363)

  (func (export "dhcp_ipv4_pack")
    (param $a i32) (param $b i32) (param $c i32) (param $d i32)
    (result i32)
    local.get $a
    i32.const 255
    i32.and
    i32.const 24
    i32.shl
    local.get $b
    i32.const 255
    i32.and
    i32.const 16
    i32.shl
    i32.or
    local.get $c
    i32.const 255
    i32.and
    i32.const 8
    i32.shl
    i32.or
    local.get $d
    i32.const 255
    i32.and
    i32.or)

  ;; op mapping: 1 BOOTREQUEST, 2 BOOTREPLY, 0 invalid.
  (func (export "dhcp_op_classify") (param $op i32) (result i32)
    local.get $op
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $op
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (func (export "dhcp_op_status") (param $op i32) (result i32)
    local.get $op
    call $dhcp_op_classify_internal
    i32.eqz
    if (result i32)
      i32.const 3
    else
      i32.const 0
    end)

  (func $dhcp_op_classify_internal (param $op i32) (result i32)
    local.get $op
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $op
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  ;; htype/hlen mapping: 1 Ethernet(1,6), 2 other non-zero hardware,
  ;; 0 invalid/empty. DHCP chaddr storage caps hlen at 16.
  (func $dhcp_htype_classify (export "dhcp_htype_classify")
    (param $htype i32) (param $hlen i32)
    (result i32)
    local.get $htype
    i32.eqz
    local.get $hlen
    i32.eqz
    i32.or
    local.get $hlen
    i32.const 16
    i32.gt_u
    i32.or
    if (result i32)
      i32.const 0
    else
      local.get $htype
      i32.const 1
      i32.eq
      local.get $hlen
      i32.const 6
      i32.eq
      i32.and
      if (result i32)
        i32.const 1
      else
        i32.const 2
      end
    end)

  ;; Message type mapping: 1 discover, 2 offer, 3 request, 4 decline,
  ;; 5 ack, 6 nak, 7 release, 8 inform, 0 invalid.
  (func (export "dhcp_message_type_classify") (param $value i32) (result i32)
    local.get $value
    i32.const 1
    i32.ge_u
    local.get $value
    i32.const 8
    i32.le_u
    i32.and
    if (result i32)
      local.get $value
    else
      i32.const 0
    end)

  (func (export "dhcp_message_type_status") (param $value i32) (result i32)
    local.get $value
    i32.const 1
    i32.ge_u
    local.get $value
    i32.const 8
    i32.le_u
    i32.and
    if (result i32)
      i32.const 0
    else
      i32.const 3
    end)

  ;; Option classes:
  ;; 0 pad, 1 end, 2 single IPv4, 3 IPv4 list, 4 u32 seconds,
  ;; 5 DHCP message type, 6 byte list, 7 text/opaque, 8 PXE arch,
  ;; 9 PXE UNDI, 10 PXE machine id, 11 vendor encapsulated, 12 client id,
  ;; 255 unrecognized raw option.
  (func $dhcp_option_classify (export "dhcp_option_classify") (param $code i32) (result i32)
    local.get $code
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $code
      i32.const 255
      i32.eq
      if (result i32)
        i32.const 1
      else
        local.get $code
        i32.const 1
        i32.eq
        local.get $code
        i32.const 50
        i32.eq
        i32.or
        local.get $code
        i32.const 54
        i32.eq
        i32.or
        if (result i32)
          i32.const 2
        else
          local.get $code
          i32.const 3
          i32.eq
          local.get $code
          i32.const 6
          i32.eq
          i32.or
          if (result i32)
            i32.const 3
          else
            local.get $code
            i32.const 51
            i32.eq
            local.get $code
            i32.const 58
            i32.eq
            i32.or
            local.get $code
            i32.const 59
            i32.eq
            i32.or
            if (result i32)
              i32.const 4
            else
              local.get $code
              i32.const 53
              i32.eq
              if (result i32)
                i32.const 5
              else
                local.get $code
                i32.const 55
                i32.eq
                if (result i32)
                  i32.const 6
                else
                  local.get $code
                  i32.const 12
                  i32.eq
                  local.get $code
                  i32.const 66
                  i32.eq
                  i32.or
                  local.get $code
                  i32.const 67
                  i32.eq
                  i32.or
                  if (result i32)
                    i32.const 7
                  else
                    local.get $code
                    i32.const 93
                    i32.eq
                    if (result i32)
                      i32.const 8
                    else
                      local.get $code
                      i32.const 94
                      i32.eq
                      if (result i32)
                        i32.const 9
                      else
                        local.get $code
                        i32.const 97
                        i32.eq
                        if (result i32)
                          i32.const 10
                        else
                          local.get $code
                          i32.const 43
                          i32.eq
                          if (result i32)
                            i32.const 11
                          else
                            local.get $code
                            i32.const 61
                            i32.eq
                            if (result i32)
                              i32.const 12
                            else
                              i32.const 255
                            end
                          end
                        end
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

  (func $dhcp_option_length_status (export "dhcp_option_length_status")
    (param $code i32) (param $value_len i32)
    (result i32)
    local.get $code
    i32.const 0
    i32.eq
    local.get $code
    i32.const 255
    i32.eq
    i32.or
    if (result i32)
      local.get $value_len
      i32.eqz
      if (result i32) i32.const 0 else i32.const 5 end
    else
      local.get $code
      i32.const 1
      i32.eq
      local.get $code
      i32.const 50
      i32.eq
      i32.or
      local.get $code
      i32.const 51
      i32.eq
      i32.or
      local.get $code
      i32.const 54
      i32.eq
      i32.or
      local.get $code
      i32.const 58
      i32.eq
      i32.or
      local.get $code
      i32.const 59
      i32.eq
      i32.or
      if (result i32)
        local.get $value_len
        i32.const 4
        i32.eq
        if (result i32) i32.const 0 else i32.const 5 end
      else
        local.get $code
        i32.const 3
        i32.eq
        local.get $code
        i32.const 6
        i32.eq
        i32.or
        if (result i32)
          local.get $value_len
          i32.const 4
          i32.ge_u
          local.get $value_len
          i32.const 3
          i32.and
          i32.eqz
          i32.and
          if (result i32) i32.const 0 else i32.const 5 end
        else
          local.get $code
          i32.const 53
          i32.eq
          if (result i32)
            local.get $value_len
            i32.const 1
            i32.eq
            if (result i32) i32.const 0 else i32.const 5 end
          else
            local.get $code
            i32.const 93
            i32.eq
            if (result i32)
              local.get $value_len
              i32.const 2
              i32.ge_u
              if (result i32) i32.const 0 else i32.const 5 end
            else
              local.get $code
              i32.const 94
              i32.eq
              if (result i32)
                local.get $value_len
                i32.const 3
                i32.ge_u
                if (result i32) i32.const 0 else i32.const 5 end
              else
                local.get $code
                i32.const 97
                i32.eq
                if (result i32)
                  local.get $value_len
                  i32.const 17
                  i32.eq
                  if (result i32) i32.const 0 else i32.const 5 end
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (func (export "dhcp_cookie_status")
    (param $ptr i32) (param $len i32)
    (result i32)
    local.get $len
    i32.const 240
    i32.lt_u
    if (result i32)
      i32.const 1
    else
      local.get $ptr
      i32.const 236
      i32.add
      call $m74read_u32_be
      i32.const 0x63825363
      i32.eq
      if (result i32) i32.const 0 else i32.const 4 end
    end)

  ;; field: 0 ciaddr, 1 yiaddr, 2 siaddr, 3 giaddr.
  (func (export "dhcp_ipv4_field")
    (param $ptr i32) (param $len i32) (param $field i32)
    (result i64)
    (local $offset i32)
    local.get $len
    i32.const 28
    i32.lt_u
    if
      i32.const 1
      i32.const 0
      call $pack
      return
    end
    local.get $field
    i32.const 3
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    i32.const 12
    local.get $field
    i32.const 4
    i32.mul
    i32.add
    local.set $offset
    i32.const 0
    local.get $ptr
    local.get $offset
    i32.add
    call $m74read_u32_be
    call $pack)

  ;; Decode fixed DHCPv4/BOOTP header plus magic cookie. Writes little-endian u32:
  ;; op_class, htype_class, htype, hlen, hops, xid, secs, flags, broadcast,
  ;; ciaddr, yiaddr, siaddr, giaddr, cookie.
  (func (export "dhcp_header_decode")
    (param $ptr i32) (param $len i32) (param $out i32)
    (result i32)
    (local $op i32)
    (local $htype i32)
    (local $hlen i32)
    (local $flags i32)
    local.get $out
    i32.eqz
    if
      i32.const 2
      return
    end
    local.get $len
    i32.const 240
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.tee $op
    call $dhcp_op_classify_internal
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $ptr
    i32.const 236
    i32.add
    call $m74read_u32_be
    i32.const 0x63825363
    i32.ne
    if
      i32.const 4
      return
    end
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $htype
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    local.set $hlen
    local.get $ptr
    i32.const 10
    i32.add
    call $m74read_u16_be
    local.set $flags

    local.get $out
    local.get $op
    call $dhcp_op_classify_internal
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $htype
    local.get $hlen
    call $dhcp_htype_classify
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $htype
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $hlen
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $ptr
    i32.const 4
    i32.add
    call $m74read_u32_be
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $ptr
    i32.const 8
    i32.add
    call $m74read_u16_be
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $flags
    i32.store
    local.get $out
    i32.const 32
    i32.add
    local.get $flags
    i32.const 0x8000
    i32.and
    i32.const 0
    i32.ne
    i32.store
    local.get $out
    i32.const 36
    i32.add
    local.get $ptr
    i32.const 12
    i32.add
    call $m74read_u32_be
    i32.store
    local.get $out
    i32.const 40
    i32.add
    local.get $ptr
    i32.const 16
    i32.add
    call $m74read_u32_be
    i32.store
    local.get $out
    i32.const 44
    i32.add
    local.get $ptr
    i32.const 20
    i32.add
    call $m74read_u32_be
    i32.store
    local.get $out
    i32.const 48
    i32.add
    local.get $ptr
    i32.const 24
    i32.add
    call $m74read_u32_be
    i32.store
    local.get $out
    i32.const 52
    i32.add
    i32.const 0x63825363
    i32.store
    i32.const 0)

  ;; Walk one DHCP option at offset relative to ptr. For PAD and END there is
  ;; no length byte. Record fields are u32: code,class,value_off,value_len,total_len,length_status.
  (func (export "dhcp_option_next")
    (param $ptr i32) (param $len i32) (param $offset i32) (param $out i32)
    (result i64)
    (local $code i32)
    (local $value_len i32)
    (local $length_status i32)
    (local $next i32)
    local.get $out
    i32.eqz
    if
      i32.const 2
      local.get $offset
      call $pack
      return
    end
    local.get $offset
    local.get $len
    i32.ge_u
    if
      i32.const 1
      local.get $len
      call $pack
      return
    end
    local.get $ptr
    local.get $offset
    i32.add
    i32.load8_u
    local.set $code
    local.get $code
    i32.const 0
    i32.eq
    local.get $code
    i32.const 255
    i32.eq
    i32.or
    if
      i32.const 0
      local.set $value_len
      local.get $offset
      i32.const 1
      i32.add
      local.set $next
      i32.const 0
      local.set $length_status
    else
      local.get $len
      local.get $offset
      i32.sub
      i32.const 2
      i32.lt_u
      if
        i32.const 1
        local.get $len
        call $pack
        return
      end
      local.get $ptr
      local.get $offset
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      local.set $value_len
      local.get $offset
      i32.const 2
      i32.add
      local.get $value_len
      i32.add
      local.tee $next
      local.get $len
      i32.gt_u
      if
        i32.const 1
        local.get $len
        call $pack
        return
      end
      local.get $code
      local.get $value_len
      call $dhcp_option_length_status
      local.set $length_status
    end

    local.get $out
    local.get $code
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $code
    call $dhcp_option_classify
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $offset
    local.get $code
    i32.const 0
    i32.eq
    local.get $code
    i32.const 255
    i32.eq
    i32.or
    if (result i32) i32.const 1 else i32.const 2 end
    i32.add
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $value_len
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $next
    local.get $offset
    i32.sub
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $length_status
    i32.store

    local.get $length_status
    i32.eqz
    if (result i64)
      i32.const 0
      local.get $next
      call $pack
    else
      i32.const 5
      local.get $next
      call $pack
    end)
)
