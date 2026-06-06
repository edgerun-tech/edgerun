;; Residual protocol-suite semantics plundered from edgerun-protocols.

  (func (export "protocol_suite_abi_version") (result i32) i32.const 1)

  (func (export "proxy_http_request_kind") (param $utf8_ok i32) (param $has_first_line i32) (param $has_method i32) (param $has_target i32) (param $method_is_connect i32) (result i32)
    ;; 1 CONNECT, 2 forward, negative values are parser failures.
    (if (i32.eqz (local.get $utf8_ok)) (then (return (i32.const -2))))
    (if (i32.eqz (local.get $has_first_line)) (then (return (i32.const -1))))
    (if (i32.eqz (i32.and (local.get $has_method) (local.get $has_target))) (then (return (i32.const -3))))
    (if (local.get $method_is_connect) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "proxy_socks5_greeting_result") (param $len i32) (param $version i32) (param $method_count i32) (param $has_no_auth i32) (result i32)
    ;; 0 select no-auth, 1 unsupported version, 2 bad request/truncated methods, 3 no acceptable auth.
    (if (i32.or (i32.lt_u (local.get $len) (i32.const 3)) (i32.ne (local.get $version) (i32.const 5))) (then (return (i32.const 1))))
    (if (i32.lt_u (local.get $len) (i32.add (i32.const 2) (local.get $method_count))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $has_no_auth)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "proxy_socks5_request_result") (param $len i32) (param $version i32) (param $cmd i32) (param $reserved i32) (param $atyp i32) (param $domain_len i32) (result i32)
    ;; 0 ok, 1 unsupported version, 2 unsupported command, 3 bad request, 4 unsupported address type.
    (if (i32.or (i32.lt_u (local.get $len) (i32.const 5)) (i32.ne (local.get $version) (i32.const 5))) (then (return (i32.const 1))))
    (if (i32.ne (local.get $cmd) (i32.const 1)) (then (return (i32.const 2))))
    (if (i32.ne (local.get $reserved) (i32.const 0)) (then (return (i32.const 3))))
    (if (i32.and (i32.eq (local.get $atyp) (i32.const 1)) (i32.ge_u (local.get $len) (i32.const 10))) (then (return (i32.const 0))))
    (if (i32.eq (local.get $atyp) (i32.const 3))
      (then
        (if (i32.lt_u (local.get $len) (i32.add (i32.const 7) (local.get $domain_len))) (then (return (i32.const 3))))
        (return (i32.const 0))))
    i32.const 4)

  (func (export "pkce_verifier_result") (param $len i32) (result i32)
    ;; RFC7636 verifier length: 43..128 chars.
    (if (i32.or (i32.lt_u (local.get $len) (i32.const 43)) (i32.gt_u (local.get $len) (i32.const 128))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "bootstrap_genesis_result") (param $keygen_ok i32) (param $sign_ok i32) (param $store_ok i32) (param $seq i64) (param $event_type i32) (param $stream_matches_node i32) (result i32)
    ;; 0 ok, 1 key generation/store failure, 2 sign failure, 3 not seq-0 node genesis, 4 stream mismatch.
    (if (i32.eqz (local.get $keygen_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $sign_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $store_ok)) (then (return (i32.const 1))))
    (if (i32.or (i64.ne (local.get $seq) (i64.const 0)) (i32.ne (local.get $event_type) (i32.const 1))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $stream_matches_node)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "ethernet_ipv4_packet_result") (param $len i32) (param $ethertype i32) (result i32)
    ;; 0 ok, 1 too short, 2 non-IPv4.
    (if (i32.lt_u (local.get $len) (i32.const 34)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $ethertype) (i32.const 2048)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "ipv4_is_private") (param $a i32) (param $b i32) (result i32)
    (if (i32.eq (local.get $a) (i32.const 10)) (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $a) (i32.const 172)) (i32.and (i32.ge_u (local.get $b) (i32.const 16)) (i32.lt_u (local.get $b) (i32.const 32)))) (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $a) (i32.const 192)) (i32.eq (local.get $b) (i32.const 168))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "ipv4_route_uses_gateway") (param $dst_a i32) (param $local_a i32) (result i32)
    ;; The crate routed directly only when first octets matched.
    (if (i32.eq (local.get $dst_a) (local.get $local_a)) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "udp_packet_len_result") (param $payload_len i32) (result i32)
    ;; 0 ok for Ethernet MTU packet buffer, 1 too large/overflow.
    (if (i32.gt_u (i32.add (i32.const 42) (local.get $payload_len)) (i32.const 1514)) (then (return (i32.const 1))))
    (if (i32.gt_u (i32.add (i32.const 8) (local.get $payload_len)) (i32.const 65535)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "dbus_consume_type_result") (param $first_byte i32) (param $sig_len i32) (param $compound_closed i32) (result i32)
    ;; 0 ok, 1 empty, 2 malformed compound, 3 bad type.
    (if (i32.eqz (local.get $sig_len)) (then (return (i32.const 1))))
    (if (i32.or (i32.eq (local.get $first_byte) (i32.const 97)) (i32.eq (local.get $first_byte) (i32.const 40)))
      (then
        (if (i32.eqz (local.get $compound_closed)) (then (return (i32.const 2))))
        (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $first_byte) (i32.const 118))
        (i32.or (i32.eq (local.get $first_byte) (i32.const 121))
          (i32.or (i32.eq (local.get $first_byte) (i32.const 98))
            (i32.or (i32.eq (local.get $first_byte) (i32.const 113))
              (i32.or (i32.eq (local.get $first_byte) (i32.const 105))
                (i32.or (i32.eq (local.get $first_byte) (i32.const 117))
                  (i32.or (i32.eq (local.get $first_byte) (i32.const 116))
                    (i32.or (i32.eq (local.get $first_byte) (i32.const 115))
                      (i32.or (i32.eq (local.get $first_byte) (i32.const 111)) (i32.eq (local.get $first_byte) (i32.const 103)))))))))))
      (then (return (i32.const 0))))
    i32.const 3)

  (func (export "dbus_decode_message_result") (param $len i32) (param $byte_order i32) (param $message_type i32) (param $body_len i32) (param $header_fields_len i32) (result i32)
    ;; 0 ok, 1 short, 2 bad byte order, 3 bad message type, 4 truncated body.
    (local $aligned_header i32)
    (if (i32.lt_u (local.get $len) (i32.const 16)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $byte_order) (i32.const 108)) (then (return (i32.const 2))))
    (if (i32.or (i32.lt_u (local.get $message_type) (i32.const 1)) (i32.gt_u (local.get $message_type) (i32.const 4))) (then (return (i32.const 3))))
    (local.set $aligned_header (i32.and (i32.add (i32.add (i32.const 16) (local.get $header_fields_len)) (i32.const 7)) (i32.const -8)))
    (if (i32.gt_u (i32.add (local.get $aligned_header) (local.get $body_len)) (local.get $len)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "goodix_packet_result") (param $len i32) (param $header_crc_ok i32) (param $len_with_crc i32) (param $packet_crc_ok i32) (result i32)
    ;; 0 ok, 1 short packet, 2 bad header crc, 3 short payload, 4 bad payload len, 5 bad packet crc.
    (if (i32.lt_u (local.get $len) (i32.const 12)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $header_crc_ok)) (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $len) (i32.add (i32.const 8) (local.get $len_with_crc))) (then (return (i32.const 3))))
    (if (i32.lt_u (local.get $len_with_crc) (i32.const 4)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $packet_crc_ok)) (then (return (i32.const 5))))
    i32.const 0)

  (func (export "goodix_ack_result") (param $cmd0 i32) (param $payload_len i32) (result i32)
    (if (i32.or (i32.ne (local.get $cmd0) (i32.const 170)) (i32.lt_u (local.get $payload_len) (i32.const 2))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "goodix_template_result") (param $len i32) (param $marker i32) (param $payload_size i32) (result i32)
    ;; 0 ok, 1 short, 2 bad marker, 3 bad payload size.
    (if (i32.lt_u (local.get $len) (i32.const 71)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $marker) (i32.const 67)) (then (return (i32.const 2))))
    (if (i32.or (i32.gt_u (local.get $payload_size) (i32.const 56)) (i32.lt_u (local.get $len) (i32.add (i32.const 69) (local.get $payload_size)))) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "goodix_result_success") (param $result i32) (result i32)
    (if (i32.lt_u (local.get $result) (i32.const 128)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "goodix_finger_mode_code") (param $status i32) (result i32)
    ;; 1 success, 2 wait-finger-up timeout, 0 other.
    (if (i32.eq (local.get $status) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $status) (i32.const 199)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "goodix_finger_list_result") (param $payload_len i32) (param $status i32) (param $count i32) (result i32)
    ;; 0 ok, 1 short payload, 2 failed status, 3 short count, 4 count too large.
    (if (i32.eqz (local.get $payload_len)) (then (return (i32.const 1))))
    (if (i32.ge_u (local.get $status) (i32.const 128)) (then (return (i32.const 2))))
    (if (i32.lt_u (local.get $payload_len) (i32.const 2)) (then (return (i32.const 3))))
    (if (i32.gt_u (local.get $count) (i32.const 20)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "wifi_ap_config_result") (param $ssid_len i32) (result i32)
    ;; 0 ok, 1 empty ssid, 2 too long.
    (if (i32.eqz (local.get $ssid_len)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $ssid_len) (i32.const 32)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "wifi_open_ap_action")
    (param $frame_len i32) (param $frame_type i32) (param $subtype i32) (param $probe_matches i32)
    (param $station_valid i32) (param $station_slot_available i32) (param $addr1_matches_bssid i32)
    (param $to_ds i32) (param $from_ds i32) (param $snap_ok i32) (result i32)
    ;; 1 probe response, 2 auth response, 3 assoc response, 4 decapsulated data,
    ;; negative codes: -1 malformed, -2 unsupported, -3 no station slot.
    (if (i32.lt_u (local.get $frame_len) (i32.const 2)) (then (return (i32.const -1))))
    (if (i32.and (i32.eq (local.get $frame_type) (i32.const 0)) (i32.eq (local.get $subtype) (i32.const 4)))
      (then
        (if (i32.lt_u (local.get $frame_len) (i32.const 24)) (then (return (i32.const -1))))
        (if (i32.eqz (local.get $probe_matches)) (then (return (i32.const -2))))
        (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $frame_type) (i32.const 0)) (i32.eq (local.get $subtype) (i32.const 11)))
      (then
        (if (i32.eqz (local.get $station_valid)) (then (return (i32.const -1))))
        (if (i32.eqz (local.get $station_slot_available)) (then (return (i32.const -3))))
        (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $frame_type) (i32.const 0)) (i32.eq (local.get $subtype) (i32.const 0)))
      (then
        (if (i32.eqz (local.get $station_valid)) (then (return (i32.const -1))))
        (if (i32.eqz (local.get $station_slot_available)) (then (return (i32.const -3))))
        (return (i32.const 3))))
    (if (i32.and (i32.eq (local.get $frame_type) (i32.const 2)) (i32.eq (local.get $subtype) (i32.const 0)))
      (then
        (if (i32.eqz (local.get $addr1_matches_bssid)) (then (return (i32.const -2))))
        (if (i32.or (i32.eqz (local.get $to_ds)) (local.get $from_ds)) (then (return (i32.const -2))))
        (if (i32.or (i32.lt_u (local.get $frame_len) (i32.const 32)) (i32.eqz (local.get $snap_ok))) (then (return (i32.const -2))))
        (return (i32.const 4))))
    i32.const -2)

  (func (export "wifi_next_seq") (param $seq i32) (result i32)
    (i32.and (i32.add (local.get $seq) (i32.const 1)) (i32.const 4095)))