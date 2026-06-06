(module
  (import "edgerun-core" "memory" (memory 1))
  (import "edgerun-core" "pack" (func $pack (param i32 i32) (result i64)))

(func (export "proto_standard_id") (result i32)
    (i32.const 300104))

  ;; Packet type ids mirror quic::packet::PacketType:
  ;; 0 Initial, 1 0-RTT, 2 Handshake, 3 Retry, 4 1-RTT short header.
  (func $quic_packet_type_from_first (export "quic_packet_type_from_first") (param $first i32) (result i32)
    (local.set $first (i32.and (local.get $first) (i32.const 255)))
    (if (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then
        (return
          (i32.and
            (i32.shr_u (local.get $first) (i32.const 4))
            (i32.const 3)))))
    (i32.const 4))

  ;; Form flags: bit0 fixed bit, bit1 long header, bit2 short header,
  ;; bit3 short-header key phase, bit4 retry long header.
  (func (export "quic_packet_form_flags") (param $first i32) (result i32)
    (local $flags i32)
    (local.set $first (i32.and (local.get $first) (i32.const 255)))
    (if (i32.ne (i32.and (local.get $first) (i32.const 64)) (i32.const 0))
      (then (local.set $flags (i32.or (local.get $flags) (i32.const 1)))))
    (if (i32.ne (i32.and (local.get $first) (i32.const 128)) (i32.const 0))
      (then
        (local.set $flags (i32.or (local.get $flags) (i32.const 2)))
        (if (i32.eq (call $quic_packet_type_from_first (local.get $first)) (i32.const 3))
          (then (local.set $flags (i32.or (local.get $flags) (i32.const 16))))))
      (else
        (local.set $flags (i32.or (local.get $flags) (i32.const 4)))
        (if (i32.ne (i32.and (local.get $first) (i32.const 4)) (i32.const 0))
          (then (local.set $flags (i32.or (local.get $flags) (i32.const 8)))))))
    (local.get $flags))

  (func (export "quic_packet_number_length") (param $first i32) (result i32)
    (i32.add (i32.and (local.get $first) (i32.const 3)) (i32.const 1)))

  (func (export "quic_packet_number_length_for_value") (param $packet_number i64) (result i32)
    (if (i64.le_u (local.get $packet_number) (i64.const 255))
      (then (return (i32.const 1))))
    (if (i64.le_u (local.get $packet_number) (i64.const 65535))
      (then (return (i32.const 2))))
    (if (i64.le_u (local.get $packet_number) (i64.const 16777215))
      (then (return (i32.const 3))))
    (i32.const 4))

  ;; QUIC varint decode status: 0 ok, 1 empty/out-of-range offset, 5 truncated.
  ;; Returns pack(status, bytes_read) and stores the decoded u64 at out_ptr on ok.
  (func (export "quic_varint_decode_at")
    (param $in_ptr i32) (param $in_len i32) (param $offset i32) (param $out_ptr i32)
    (result i64)
    (local $first i32)
    (local $need i32)
    (local $i i32)
    (local $value i64)
    (if (i32.ge_u (local.get $offset) (local.get $in_len))
      (then (return (call $pack (i32.const 1) (i32.const 0)))))
    (local.set $first (i32.load8_u (i32.add (local.get $in_ptr) (local.get $offset))))
    (local.set $need
      (i32.shl
        (i32.const 1)
        (i32.shr_u (local.get $first) (i32.const 6))))
    (if
      (i32.or
        (i32.gt_u (local.get $need) (local.get $in_len))
        (i32.gt_u (local.get $offset) (i32.sub (local.get $in_len) (local.get $need))))
      (then (return (call $pack (i32.const 5) (i32.const 0)))))
    (local.set $value (i64.extend_i32_u (i32.and (local.get $first) (i32.const 63))))
    (local.set $i (i32.const 1))
    (loop $again
      (if (i32.lt_u (local.get $i) (local.get $need))
        (then
          (local.set $value
            (i64.or
              (i64.shl (local.get $value) (i64.const 8))
              (i64.extend_i32_u
                (i32.load8_u
                  (i32.add
                    (i32.add (local.get $in_ptr) (local.get $offset))
                    (local.get $i))))))
          (local.set $i (i32.add (local.get $i) (i32.const 1)))
          (br $again))))
    (i64.store (local.get $out_ptr) (local.get $value))
    (call $pack (i32.const 0) (local.get $need)))

  ;; Transport parameter classes for fields retained by TransportParameters:
  ;; 0 max_idle_timeout, 1 max_packet_size, 2 ack_delay_exponent,
  ;; 3 max_ack_delay, 4 active_connection_id_limit, 8 reserved greasing,
  ;; 9 known but not retained here, 10 unknown.
  (func (export "quic_transport_param_classify") (param $id i64) (result i32)
    (if (i64.eq (local.get $id) (i64.const 1))
      (then (return (i32.const 0))))
    (if (i64.eq (local.get $id) (i64.const 3))
      (then (return (i32.const 1))))
    (if (i64.eq (local.get $id) (i64.const 10))
      (then (return (i32.const 2))))
    (if (i64.eq (local.get $id) (i64.const 11))
      (then (return (i32.const 3))))
    (if (i64.eq (local.get $id) (i64.const 14))
      (then (return (i32.const 4))))
    (if (i64.eq (i64.rem_u (local.get $id) (i64.const 31)) (i64.const 27))
      (then (return (i32.const 8))))
    (if
      (i32.or
        (i64.eq (local.get $id) (i64.const 0))
        (i32.or
          (i64.eq (local.get $id) (i64.const 2))
          (i32.or
            (i64.eq (local.get $id) (i64.const 4))
            (i32.or
              (i64.eq (local.get $id) (i64.const 5))
              (i32.or
                (i64.eq (local.get $id) (i64.const 6))
                (i32.or
                  (i64.eq (local.get $id) (i64.const 7))
                  (i32.or
                    (i64.eq (local.get $id) (i64.const 8))
                    (i64.eq (local.get $id) (i64.const 9)))))))))
      (then (return (i32.const 9))))
    (i32.const 10))

  ;; Frame classes mirror quic::frame::QuicFrameType order:
  ;; 0 padding, 1 ping, 2 ack, 3 ack_ecn, 4 reset_stream, 5 stop_sending,
  ;; 6 crypto, 7 new_token, 8 stream, 9 max_data, 10 max_stream_data,
  ;; 11 max_streams_bidi, 12 max_streams_uni, 13 data_blocked,
  ;; 14 stream_data_blocked, 15 streams_blocked_bidi, 16 streams_blocked_uni,
  ;; 17 new_connection_id, 18 retire_connection_id, 19 path_challenge,
  ;; 20 path_response, 21 connection_close, 22 app_connection_close,
  ;; 23 handshake_done, 255 unknown.
  (func (export "quic_frame_type_classify") (param $frame_type i32) (result i32)
    (local.set $frame_type (i32.and (local.get $frame_type) (i32.const 255)))
    (if (i32.le_u (local.get $frame_type) (i32.const 7))
      (then (return (local.get $frame_type))))
    (if
      (i32.and
        (i32.ge_u (local.get $frame_type) (i32.const 8))
        (i32.le_u (local.get $frame_type) (i32.const 15)))
      (then (return (i32.const 8))))
    (if
      (i32.and
        (i32.ge_u (local.get $frame_type) (i32.const 16))
        (i32.le_u (local.get $frame_type) (i32.const 30)))
      (then (return (i32.sub (local.get $frame_type) (i32.const 7)))))
    (i32.const 255))

  ;; Packet number spaces and CryptoPhase share the same core levels:
  ;; 0 Initial, 1 Handshake, 2 ApplicationData/Application, 3 retry/no crypto.
  (func $quic_crypto_level_for_packet_type (export "quic_crypto_level_for_packet_type") (param $packet_type i32) (result i32)
    (if (i32.eq (local.get $packet_type) (i32.const 0))
      (then (return (i32.const 0))))
    (if (i32.eq (local.get $packet_type) (i32.const 2))
      (then (return (i32.const 1))))
    (if
      (i32.or
        (i32.eq (local.get $packet_type) (i32.const 1))
        (i32.eq (local.get $packet_type) (i32.const 4)))
      (then (return (i32.const 2))))
    (i32.const 3))

  (func (export "quic_packet_number_space_for_packet_type") (param $packet_type i32) (result i32)
    (call $quic_crypto_level_for_packet_type (local.get $packet_type)))

  ;; TLS handshake message classes used by the client QUIC-TLS driver:
  ;; 0 ServerHello, 1 EncryptedExtensions, 2 Certificate,
  ;; 3 CertificateVerify, 4 Finished, 9 other.
  (func (export "quic_tls_message_classify") (param $msg_type i32) (result i32)
    (local.set $msg_type (i32.and (local.get $msg_type) (i32.const 255)))
    (if (i32.eq (local.get $msg_type) (i32.const 2))
      (then (return (i32.const 0))))
    (if (i32.eq (local.get $msg_type) (i32.const 8))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $msg_type) (i32.const 11))
      (then (return (i32.const 2))))
    (if (i32.eq (local.get $msg_type) (i32.const 15))
      (then (return (i32.const 3))))
    (if (i32.eq (local.get $msg_type) (i32.const 20))
      (then (return (i32.const 4))))
    (i32.const 9))

  ;; Strict client-side state progression distilled from handshake.rs:
  ;; 0 new, 1 initial/client_hello sent, 2 server_hello processed,
  ;; 3 encrypted_extensions processed, 4 certificate processed,
  ;; 5 certificate_verify processed, 6 server_finished processed/client_finished ready,
  ;; 7 application keys ready, 100 invalid transition. Event ids are:
  ;; 1 client_initial_sent, 2 ServerHello, 8 EncryptedExtensions,
  ;; 11 Certificate, 15 CertificateVerify, 20 Finished, 21 app_keys_derived.
  (func (export "quic_handshake_transition") (param $state i32) (param $event i32) (result i32)
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 0))
        (i32.eq (local.get $event) (i32.const 1)))
      (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 1))
        (i32.eq (local.get $event) (i32.const 2)))
      (then (return (i32.const 2))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 2))
        (i32.eq (local.get $event) (i32.const 8)))
      (then (return (i32.const 3))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 3))
        (i32.eq (local.get $event) (i32.const 11)))
      (then (return (i32.const 4))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 4))
        (i32.eq (local.get $event) (i32.const 15)))
      (then (return (i32.const 5))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 5))
        (i32.eq (local.get $event) (i32.const 20)))
      (then (return (i32.const 6))))
    (if
      (i32.and
        (i32.eq (local.get $state) (i32.const 6))
        (i32.eq (local.get $event) (i32.const 21)))
      (then (return (i32.const 7))))
    (if (i32.eq (local.get $event) (i32.const 0))
      (then (return (local.get $state))))
    (i32.const 100))

  (func (export "quic_transport_default_u64") (param $field i32) (result i64)
    (if (i32.eq (local.get $field) (i32.const 0))
      (then (return (i64.const 65535)))) ;; max_data
    (if (i32.eq (local.get $field) (i32.const 1))
      (then (return (i64.const 65535)))) ;; max_stream_data
    (if (i32.eq (local.get $field) (i32.const 2))
      (then (return (i64.const 1200)))) ;; mtu
    (if (i32.eq (local.get $field) (i32.const 3))
      (then (return (i64.const 100000)))) ;; default rtt micros
    (if (i32.eq (local.get $field) (i32.const 4))
      (then (return (i64.const 50000)))) ;; default rttvar micros
    (i64.const 0))
)
