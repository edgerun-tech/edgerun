(module
  (import "edgerun" "pack" (func $pack (param i32 i32) (result i64)))
  (import "edgerun" "lo" (func $lo (param i64) (result i32)))
  (import "edgerun" "hi" (func $hi (param i64) (result i32)))
  (import "edgerun" "is_digit" (func $is_digit (param i32) (result i32)))
  (memory (export "memory") 1)
;; EdgeRun mesh semantics plundered from edgerun-mesh.
  ;; Shared result codes: 0 ok/false, 1 true or primary failure.

  (func (export "mesh_node_id_len") (result i32)
    i32.const 64)

  (func (export "mesh_signature_len") (result i32)
    i32.const 64)

  (func (export "mesh_header_len") (result i32)
    i32.const 130)

  (func (export "mesh_min_wire_len") (result i32)
    i32.const 194)

  (func (export "mesh_max_payload_len") (result i32)
    i32.const 65536)

  (func (export "mesh_default_ttl") (result i32)
    i32.const 16)

  (func (export "mesh_frame_type_code") (param $byte i32) (result i32)
    (if (result i32) (i32.le_u (local.get $byte) (i32.const 7))
      (then local.get $byte)
      (else i32.const -1)))

  (func (export "mesh_frame_type_admissible") (param $byte i32) (result i32)
    (i32.le_u (local.get $byte) (i32.const 7)))

  (func (export "mesh_wire_len") (param $payload_len i32) (result i32)
    (i32.add (i32.const 194) (local.get $payload_len)))

  (func (export "mesh_payload_len_from_wire") (param $wire_len i32) (result i32)
    (if (result i32) (i32.lt_u (local.get $wire_len) (i32.const 194))
      (then i32.const -1)
      (else (i32.sub (local.get $wire_len) (i32.const 194)))))

  (func (export "mesh_from_wire_result") (param $wire_len i32) (param $frame_type i32) (result i32)
    (local $payload_len i32)
    (if (i32.lt_u (local.get $wire_len) (i32.const 194))
      (then (return (i32.const 1)))) ;; too short
    (local.set $payload_len (i32.sub (local.get $wire_len) (i32.const 194)))
    (if (i32.gt_u (local.get $payload_len) (i32.const 65536))
      (then (return (i32.const 2)))) ;; payload too large
    (if (i32.gt_u (local.get $frame_type) (i32.const 255))
      (then (return (i32.const 3))))
    i32.const 0)

  (func (export "mesh_inspect_result")
    (param $wire_len i32) (param $frame_type i32) (param $ttl i32)
    (result i32)
    (local $payload_len i32)
    (if (i32.lt_u (local.get $wire_len) (i32.const 194))
      (then (return (i32.const 1)))) ;; TooShort
    (local.set $payload_len (i32.sub (local.get $wire_len) (i32.const 194)))
    (if (i32.gt_u (local.get $payload_len) (i32.const 65536))
      (then (return (i32.const 2)))) ;; PayloadTooLarge
    (if (i32.gt_u (local.get $frame_type) (i32.const 7))
      (then (return (i32.const 3)))) ;; InvalidFrameType
    (if (i32.eqz (local.get $ttl))
      (then (return (i32.const 4)))) ;; ExpiredTtl
    i32.const 0)

  (func (export "mesh_policy_result")
    (param $inspect_result i32) (param $is_local i32) (param $is_broadcast i32)
    (param $allow_broadcast i32) (param $allow_forward i32)
    (param $payload_len i32) (param $max_payload_len i32)
    (result i32)
    (if (i32.ne (local.get $inspect_result) (i32.const 0))
      (then (return (local.get $inspect_result))))
    (if (i32.gt_u (local.get $payload_len) (local.get $max_payload_len))
      (then (return (i32.const 2))))
    (if (i32.or
          (local.get $is_local)
          (i32.or
            (i32.and (local.get $is_broadcast) (local.get $allow_broadcast))
            (local.get $allow_forward)))
      (then (return (i32.const 0))))
    i32.const 5) ;; NotForThisNode

  (func (export "mesh_public_policy_allows") (param $is_local i32) (param $is_broadcast i32) (result i32)
    (drop (local.get $is_local))
    (drop (local.get $is_broadcast))
    i32.const 1)

  (func (export "mesh_local_only_policy_allows") (param $is_local i32) (param $is_broadcast i32) (result i32)
    (i32.or (local.get $is_local) (local.get $is_broadcast)))

  (func (export "mesh_signed_preimage_len") (param $payload_len i32) (result i32)
    (i32.add (i32.const 130) (local.get $payload_len)))

  (func (export "mesh_signature_domain_code") (result i32)
    i32.const 1) ;; "edgerun:v0:sig:mesh-frame" || 0 || sha256(header||payload)

  (func (export "mesh_route_update_action") (param $existing i32) (param $existing_cost i32) (param $new_cost i32) (result i32)
    (if (result i32) (i32.eqz (local.get $existing))
      (then i32.const 1) ;; insert
      (else
        (if (result i32) (i32.lt_u (local.get $new_cost) (local.get $existing_cost))
          (then i32.const 2) ;; replace
          (else i32.const 0))))) ;; keep existing

  (func (export "mesh_route_lookup_prefers_new") (param $existing_cost i32) (param $candidate_cost i32) (result i32)
    (i32.lt_u (local.get $candidate_cost) (local.get $existing_cost)))

  (func (export "mesh_route_remove_via_keeps") (param $has_next_hop i32) (param $next_hop_matches i32) (result i32)
    (i32.eqz (i32.and (local.get $has_next_hop) (local.get $next_hop_matches))))

  (func (export "mesh_peer_dead") (param $missed_heartbeats i32) (result i32)
    (i32.ge_u (local.get $missed_heartbeats) (i32.const 3)))

  (func (export "mesh_heartbeat_after_tick") (param $missed_heartbeats i32) (result i32)
    (if (result i32) (i32.eq (local.get $missed_heartbeats) (i32.const 255))
      (then i32.const 255)
      (else (i32.add (local.get $missed_heartbeats) (i32.const 1)))))

  (func (export "mesh_heartbeat_went_dead") (param $missed_before_tick i32) (result i32)
    (i32.eq (call $mesh_heartbeat_after_tick_impl (local.get $missed_before_tick)) (i32.const 3)))

  (func $mesh_heartbeat_after_tick_impl (param $missed_heartbeats i32) (result i32)
    (if (result i32) (i32.eq (local.get $missed_heartbeats) (i32.const 255))
      (then i32.const 255)
      (else (i32.add (local.get $missed_heartbeats) (i32.const 1)))))

  (func (export "mesh_process_discovery_route_action")
    (param $dest_is_local i32) (param $dest_is_sender i32)
    (param $advertised_cost i32) (param $existing i32) (param $existing_cost i32)
    (result i32)
    (local $new_cost i32)
    (if (i32.or (local.get $dest_is_local) (local.get $dest_is_sender))
      (then (return (i32.const 0)))) ;; skip echo/self
    (local.set $new_cost
      (if (result i32) (i32.eq (local.get $advertised_cost) (i32.const 255))
        (then i32.const 255)
        (else (i32.add (local.get $advertised_cost) (i32.const 1)))))
    (if (i32.eqz (local.get $new_cost))
      (then (return (i32.const 0))))
    (call $mesh_route_update_action_impl (local.get $existing) (local.get $existing_cost) (local.get $new_cost)))

  (func $mesh_route_update_action_impl (param $existing i32) (param $existing_cost i32) (param $new_cost i32) (result i32)
    (if (result i32) (i32.eqz (local.get $existing))
      (then i32.const 1)
      (else
        (if (result i32) (i32.lt_u (local.get $new_cost) (local.get $existing_cost))
          (then i32.const 2)
          (else i32.const 0)))))

  (func (export "mesh_direct_peer_route_action") (param $existing i32) (param $existing_cost i32) (result i32)
    (call $mesh_route_update_action_impl (local.get $existing) (local.get $existing_cost) (i32.const 1)))

  (func (export "mesh_next_hop_kind") (param $dest_is_local i32) (param $has_route i32) (param $route_has_next_hop i32) (result i32)
    (if (i32.or (local.get $dest_is_local) (i32.eqz (local.get $has_route)))
      (then (return (i32.const 0)))) ;; none
    (if (result i32) (local.get $route_has_next_hop)
      (then i32.const 2) ;; route.next_hop
      (else i32.const 1))) ;; destination itself

  (func (export "mesh_should_forward_result") (param $dest_is_local i32) (param $ttl i32) (param $has_route i32) (result i32)
    (if (i32.or (local.get $dest_is_local) (i32.eqz (local.get $ttl)))
      (then (return (i32.const -1))))
    (if (i32.eqz (local.get $has_route))
      (then (return (i32.const -1))))
    (i32.sub (local.get $ttl) (i32.const 1)))

  (func (export "mesh_discovery_max_routes") (result i32)
    i32.const 50)

  (func (export "mesh_discovery_encoded_len") (param $route_count i32) (result i32)
    (i32.add
      (i32.const 5)
      (i32.mul
        (if (result i32) (i32.gt_u (local.get $route_count) (i32.const 50))
          (then i32.const 50)
          (else local.get $route_count))
        (i32.const 65))))

  (func (export "mesh_discovery_decode_result") (param $wire_len i32) (param $route_count i32) (result i32)
    (local $expected i32)
    (if (i32.lt_u (local.get $wire_len) (i32.const 5))
      (then (return (i32.const 1))))
    (local.set $expected (i32.add (i32.const 5) (i32.mul (local.get $route_count) (i32.const 65))))
    (if (result i32) (i32.lt_u (local.get $wire_len) (local.get $expected))
      (then i32.const 2)
      (else i32.const 0)))

  (func (export "mesh_next_sequence") (param $current i32) (result i32)
    (i32.add (local.get $current) (i32.const 1)))

  (func (export "mesh_session_constant") (param $which i32) (result i64)
    (if (result i64) (i32.eq (local.get $which) (i32.const 1))
      (then i64.const 65) ;; ECDH public key size
      (else
        (if (result i64) (i32.eq (local.get $which) (i32.const 2))
          (then i64.const 129) ;; handshake size
          (else
            (if (result i64) (i32.eq (local.get $which) (i32.const 3))
              (then i64.const 12) ;; nonce size
              (else
                (if (result i64) (i32.eq (local.get $which) (i32.const 4))
                  (then i64.const 4) ;; nonce prefix
                  (else
                    (if (result i64) (i32.eq (local.get $which) (i32.const 5))
                      (then i64.const 1000000) ;; max frames before rekey
                      (else
                        (if (result i64) (i32.eq (local.get $which) (i32.const 6))
                          (then i64.const 300) ;; max age seconds
                          (else i64.const 0)))))))))))))

  (func (export "mesh_handshake_decode_result") (param $wire_len i32) (param $pubkey_valid i32) (result i32)
    (if (i32.ne (local.get $wire_len) (i32.const 129))
      (then (return (i32.const 1))))
    (if (result i32) (local.get $pubkey_valid)
      (then i32.const 0)
      (else i32.const 2)))

  (func (export "mesh_encrypt_wire_len") (param $plaintext_len i32) (result i32)
    (i32.add (i32.const 28) (local.get $plaintext_len))) ;; nonce(12) + AES-GCM tag(16)

  (func (export "mesh_decrypt_result") (param $ciphertext_len i32) (param $counter_seen i32) (param $counter i64) (param $highest i64) (param $auth_ok i32) (result i32)
    (if (i32.lt_u (local.get $ciphertext_len) (i32.const 12))
      (then (return (i32.const 5)))) ;; CiphertextTooShort
    (if (i32.and (local.get $counter_seen) (i64.le_u (local.get $counter) (local.get $highest)))
      (then (return (i32.const 6)))) ;; ReplayDetected
    (if (result i32) (local.get $auth_ok)
      (then i32.const 0)
      (else i32.const 4))) ;; DecryptionFailed

  (func (export "mesh_needs_rekey") (param $frame_count i64) (param $created_at i64) (param $now i64) (result i32)
    (i32.or
      (i64.ge_u (local.get $frame_count) (i64.const 1000000))
      (i64.ge_u (i64.sub (local.get $now) (local.get $created_at)) (i64.const 300))))

  (func (export "mesh_session_manager_encrypt_result") (param $has_session i32) (param $needs_rekey i32) (result i32)
    (if (i32.eqz (local.get $has_session))
      (then (return (i32.const 1)))) ;; NoActiveSession
    (if (result i32) (local.get $needs_rekey)
      (then i32.const 2) ;; SessionExpired
      (else i32.const 0)))

  (func (export "mesh_replay_clock_action") (param $is_logical i32) (result i32)
    (if (result i32) (local.get $is_logical)
      (then i32.const 0)
      (else i32.const 7))) ;; ReplayOnlyInput

  (func (export "mesh_link_constant") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 0x88b5) ;; EtherType
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 47080) ;; public UDP port
          (else
            (if (result i32) (i32.eq (local.get $which) (i32.const 3))
              (then i32.const 47079) ;; dev broadcast port
              (else i32.const 0)))))))

  (func (export "mesh_send_transport_choice")
    (param $has_tunnel i32) (param $is_broadcast i32) (param $has_mac i32)
    (param $has_udp_peer i32) (param $has_udp i32) (param $has_multicast i32)
    (result i32)
    (if (local.get $has_tunnel) (then (return (i32.const 1)))) ;; tunnel
    (if (local.get $is_broadcast) (then (return (i32.const 2)))) ;; multicast/udp/raw broadcast
    (if (local.get $has_mac) (then (return (i32.const 3)))) ;; raw Ethernet unicast
    (if (i32.and (local.get $has_udp) (local.get $has_udp_peer)) (then (return (i32.const 4)))) ;; learned UDP peer
    (if (local.get $has_udp) (then (return (i32.const 5)))) ;; UDP broadcast fallback
    (if (local.get $has_multicast) (then (return (i32.const 6)))) ;; multicast fallback
    i32.const 0)

  (func (export "mesh_inbound_action")
    (param $signature_ok i32) (param $is_for_us i32) (param $frame_type i32)
    (param $should_forward_ttl i32)
    (result i32)
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 0)))) ;; drop
    (if (result i32) (local.get $is_for_us)
      (then
        (if (result i32) (i32.eq (local.get $frame_type) (i32.const 1))
          (then i32.const 1) ;; process discovery
          (else
            (if (result i32) (i32.le_u (local.get $frame_type) (i32.const 7))
              (then i32.const 2) ;; queue for dispatcher
              (else i32.const 0)))))
      (else
        (if (result i32) (i32.ge_s (local.get $should_forward_ttl) (i32.const 0))
          (then i32.const 3) ;; forward
          (else i32.const 0)))))

  (func (export "mesh_admitted_frame_result") (param $policy_ok i32) (param $is_local i32) (param $is_broadcast i32) (param $has_route i32) (result i32)
    (if (i32.eqz (local.get $policy_ok)) (then (return (i32.const 0))))
    (i32.or
      (i32.or (local.get $is_local) (local.get $is_broadcast))
      (local.get $has_route)))

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

;; Node UI surface and authority-flow semantics plundered from crates/node.

  (func (export "node_surfaces_abi_version") (result i32) i32.const 1)
  (func (export "frontend_work_projection_schema_version") (result i32) i32.const 1)
  (func (export "frontend_input_buffer_capacity") (result i32) i32.const 4096)
  (func (export "frontend_color_scheme") (result i32) i32.const 0)

  (func (export "frontend_frame_active") (param $time_ms i64) (result i32)
    ;; active alternates every 800ms.
    (if (i64.eqz (i64.rem_u (i64.div_u (local.get $time_ms) (i64.const 800)) (i64.const 2))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_action_dirty") (param $action_kind i32) (result i32)
    ;; 1 hovered, 2 focused, 3 activated, 4 scroll changed, 5 open changed.
    (if (i32.and (i32.ge_u (local.get $action_kind) (i32.const 1)) (i32.le_u (local.get $action_kind) (i32.const 5))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_pointer_click_dirty") (param $down_dirty i32) (param $up_dirty i32) (result i32)
    (if (i32.or (local.get $down_dirty) (local.get $up_dirty)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "frontend_input_len") (param $requested_len i32) (result i32)
    (if (i32.gt_u (local.get $requested_len) (i32.const 4096)) (then (return (i32.const 4096))))
    local.get $requested_len)

  (func (export "frontend_hit_code") (param $has_hit i32) (param $kind_code i32) (param $id i32) (result i32)
    ;; u32::MAX for no hit, otherwise kind in high byte and 24-bit id.
    (if (i32.eqz (local.get $has_hit)) (then (return (i32.const -1))))
    (i32.or (i32.shl (i32.and (local.get $kind_code) (i32.const 255)) (i32.const 24)) (i32.and (local.get $id) (i32.const 0x00ffffff))))

  (func (export "web_key_code") (param $code i32) (result i32)
    ;; Returns stable UiKey codes: 1 backspace, 2 tab, 3 enter, 4 escape,
    ;; 5 pageup, 6 pagedown, 7 end, 8 home, 9..12 arrows, 13 delete, otherwise 1000+raw.
    (if (i32.eq (local.get $code) (i32.const 8)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $code) (i32.const 9)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $code) (i32.const 13)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $code) (i32.const 27)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $code) (i32.const 33)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $code) (i32.const 34)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $code) (i32.const 35)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $code) (i32.const 36)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $code) (i32.const 37)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $code) (i32.const 38)) (then (return (i32.const 10))))
    (if (i32.eq (local.get $code) (i32.const 39)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $code) (i32.const 40)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $code) (i32.const 46)) (then (return (i32.const 13))))
    (i32.add (i32.const 1000) (local.get $code)))

  (func (export "native_arg_result") (param $arg_kind i32) (param $has_value i32) (param $value_valid i32) (result i32)
    ;; arg_kind: 1 --frames, 2 --dump-scene, 3 --scheme, 4 help, 5 unknown.
    ;; 0 ok, 1 missing value, 2 invalid value, 3 help exit, 4 unknown.
    (if (i32.eq (local.get $arg_kind) (i32.const 4)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $arg_kind) (i32.const 5)) (then (return (i32.const 4))))
    (if (i32.or (i32.eq (local.get $arg_kind) (i32.const 1)) (i32.eq (local.get $arg_kind) (i32.const 3)))
      (then
        (if (i32.eqz (local.get $has_value)) (then (return (i32.const 1))))
        (if (i32.eqz (local.get $value_valid)) (then (return (i32.const 2))))))
    i32.const 0)

  (func (export "scheme_code") (param $scheme i32) (result i32)
    ;; dark/light/terminal are 1..3.
    (if (i32.and (i32.ge_u (local.get $scheme) (i32.const 1)) (i32.le_u (local.get $scheme) (i32.const 3))) (then (return (local.get $scheme))))
    i32.const 0)

  (func (export "ws_masked_client_frame_header_len") (param $payload_len i64) (result i32)
    ;; Client binary frame always includes 2 base bytes plus mask, with optional extended length.
    (if (i64.lt_u (local.get $payload_len) (i64.const 126)) (then (return (i32.const 6))))
    (if (i64.le_u (local.get $payload_len) (i64.const 65535)) (then (return (i32.const 8))))
    i32.const 14)

  (func (export "ws_server_binary_result") (param $opcode i32) (param $masked i32) (result i32)
    ;; 0 ok, 1 non-binary, 2 server frame masked.
    (if (i32.ne (i32.and (local.get $opcode) (i32.const 15)) (i32.const 2)) (then (return (i32.const 1))))
    (if (local.get $masked) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "ws_handshake_result") (param $closed_early i32) (param $response_len i32) (param $has_101 i32) (param $has_protocol i32) (result i32)
    ;; 0 ok, 1 closed early, 2 too large, 3 not switching protocols, 4 missing edgerun-work-v1.
    (if (local.get $closed_early) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $response_len) (i32.const 4096)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $has_101)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $has_protocol)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "authority_command_decision") (param $signature_ok i32) (param $duplicate i32) (result i32)
    ;; 1 committed, 2 rejected.
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 2))))
    (if (local.get $duplicate) (then (return (i32.const 1))))
    i32.const 1)

  (func (export "authority_reason_code") (param $signature_ok i32) (param $duplicate i32) (result i32)
    ;; 0 empty, 1 rejected, 2 duplicate_command.
    (if (i32.eqz (local.get $signature_ok)) (then (return (i32.const 1))))
    (if (local.get $duplicate) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_stream_len_after_event") (param $current_len i32) (result i32)
    (i32.add (local.get $current_len) (i32.const 1)))

  (func (export "sdk_runtime_projection_result") (param $graph_decodes i32) (param $records_event i32) (param $payload_hash_matches i32) (param $projection_matches_package i32) (result i32)
    ;; 0 ok, 1 graph decode failed, 2 event missing/wrong kind, 3 payload hash mismatch, 4 projection mismatch.
    (if (i32.eqz (local.get $graph_decodes)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $records_event)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $payload_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $projection_matches_package)) (then (return (i32.const 4))))
    i32.const 0)

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

;; Work/admission/settlement semantics plundered from edgerun-work.

  (func (export "work_wire_abi_version") (result i32) i32.const 1)
  (func (export "work_default_heartbeat_secs") (result i64) i64.const 10)
  (func (export "work_max_frame_len") (result i32) i32.const 1048576)
  (func (export "work_max_relay_transit_bundle_hops") (result i32) i32.const 64)

  (func (export "work_node_role_valid") (param $role i32) (result i32)
    ;; relay, storage, compute, admission, message, capability, notary, verifier.
    (i32.and (i32.ge_u (local.get $role) (i32.const 1)) (i32.le_u (local.get $role) (i32.const 8))))

  (func (export "work_type_department") (param $work_type i32) (result i32)
    ;; admission=1 relay=2 message=3 storage=4 retrieval=5 compute=6 capability=7 notary=8 verification=9.
    (if (i32.eq (local.get $work_type) (i32.const 1)) (then (return (i32.const 3))))
    (if (i32.or (i32.eq (local.get $work_type) (i32.const 2)) (i32.eq (local.get $work_type) (i32.const 4))) (then (return (i32.const 4))))
    (if (i32.eq (local.get $work_type) (i32.const 3)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $work_type) (i32.const 5)) (then (return (i32.const 6))))
    (if (i32.and (i32.ge_u (local.get $work_type) (i32.const 6)) (i32.le_u (local.get $work_type) (i32.const 10))) (then (return (i32.const 6))))
    (if (i32.and (i32.ge_u (local.get $work_type) (i32.const 11)) (i32.le_u (local.get $work_type) (i32.const 14))) (then (return (i32.const 7))))
    (if (i32.or (i32.eq (local.get $work_type) (i32.const 15)) (i32.eq (local.get $work_type) (i32.const 16))) (then (return (i32.const 8))))
    (if (i32.eq (local.get $work_type) (i32.const 17)) (then (return (i32.const 9))))
    i32.const 0)

  (func (export "work_packet_tag_valid") (param $tag i32) (result i32)
    ;; NodeAvailable, Heartbeat, RelayAssignment, NetworkMessage, Request, Admission, Receipt, Ack.
    (i32.le_u (local.get $tag) (i32.const 7)))

  (func (export "work_identity_result") (param $role i32) (param $node_id_matches_public_key i32) (result i32)
    ;; 0 ok, 1 bad role, 2 node id mismatch.
    (if (i32.eqz (call $role_valid (local.get $role))) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $node_id_matches_public_key)) (then (return (i32.const 2))))
    i32.const 0)

  (func $role_valid (param $role i32) (result i32)
    (i32.and (i32.ge_u (local.get $role) (i32.const 1)) (i32.le_u (local.get $role) (i32.const 8))))

  (func (export "work_admission_result")
    (param $request_sig_ok i32) (param $admission_sig_ok i32) (param $request_hash_matches i32)
    (param $user_matches i32) (param $admission_node_role i32) (param $has_relay_path i32)
    (param $admitted_budget i64) (param $request_max_cost i64) (param $admission_valid_until i64) (param $request_valid_until i64)
    (result i32)
    ;; 0 ok, 1 invalid request, 2 invalid admission, 3 hash/user mismatch, 4 wrong admission node,
    ;; 5 no route, 6 budget exceeds request, 7 admission outlives request.
    (if (i32.eqz (local.get $request_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_sig_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $request_hash_matches) (local.get $user_matches))) (then (return (i32.const 3))))
    (if (i32.ne (local.get $admission_node_role) (i32.const 4)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $has_relay_path)) (then (return (i32.const 5))))
    (if (i64.gt_u (local.get $admitted_budget) (local.get $request_max_cost)) (then (return (i32.const 6))))
    (if (i64.gt_u (local.get $admission_valid_until) (local.get $request_valid_until)) (then (return (i32.const 7))))
    i32.const 0)

  (func (export "work_reserve_admission_result")
    (param $admission_ok i32) (param $duplicate i32) (param $known_user i32) (param $balance i64) (param $budget i64)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 duplicate, 3 unknown user, 4 insufficient balance.
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 1))))
    (if (local.get $duplicate) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $known_user)) (then (return (i32.const 3))))
    (if (i64.lt_u (local.get $balance) (local.get $budget)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_receipt_common_result")
    (param $admission_sig_ok i32) (param $receipt_sig_ok i32) (param $admission_hash_matches i32)
    (param $request_hash_matches i32) (param $duplicate_receipt i32) (param $budget_reserved i32)
    (param $already_spent i64) (param $claim i64) (param $reserved_budget i64)
    (result i32)
    ;; 0 ok, 1 invalid admission, 2 invalid receipt, 3 admission mismatch,
    ;; 4 duplicate receipt, 5 budget not reserved, 6 claim exceeds budget.
    (if (i32.eqz (local.get $admission_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $receipt_sig_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $admission_hash_matches) (local.get $request_hash_matches))) (then (return (i32.const 3))))
    (if (local.get $duplicate_receipt) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $budget_reserved)) (then (return (i32.const 5))))
    (if (i64.gt_u (i64.add (local.get $already_spent) (local.get $claim)) (local.get $reserved_budget)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_unchecked_receipt_evidence_result") (param $worker_role i32) (param $common_result i32) (result i32)
    ;; Relay receipts require delivery/transit evidence and must not use unchecked settlement.
    (if (i32.ne (local.get $common_result) (i32.const 0)) (then (return (local.get $common_result))))
    (if (result i32) (i32.eq (local.get $worker_role) (i32.const 1)) (then i32.const 7) (else i32.const 0)))

  (func (export "work_commit_settlement_spent_after") (param $already_spent i64) (param $claim i64) (result i64)
    (i64.add (local.get $already_spent) (local.get $claim)))

  (func (export "work_prune_refund") (param $reserved_budget i64) (param $spent i64) (result i64)
    (if (i64.gt_u (local.get $spent) (local.get $reserved_budget))
      (then (return (i64.const 0))))
    (i64.sub (local.get $reserved_budget) (local.get $spent)))

  (func (export "work_batch_result")
    (param $receipt_count i32) (param $admission_ok i32) (param $duplicates_in_batch i32)
    (param $any_receipt_invalid i32) (param $any_mismatch i32) (param $total_claim i64) (param $admitted_budget i64)
    (result i32)
    ;; build_receipt_batch preflights the whole batch before ledger mutation.
    ;; 0 ok, 1 empty, 2 invalid admission, 3 invalid receipt, 4 duplicate in batch, 5 mismatch, 6 budget exceeded.
    (if (i32.eqz (local.get $receipt_count)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_ok)) (then (return (i32.const 2))))
    (if (local.get $any_receipt_invalid) (then (return (i32.const 3))))
    (if (local.get $duplicates_in_batch) (then (return (i32.const 4))))
    (if (local.get $any_mismatch) (then (return (i32.const 5))))
    (if (i64.gt_u (local.get $total_claim) (local.get $admitted_budget)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_ordered_channel_result")
    (param $route_hash_matches i32) (param $packet_hash_matches i32)
    (param $sequence i64) (param $expected_sequence i64) (param $previous_hash_matches i32)
    (result i32)
    ;; 0 ok, 1 route mismatch, 2 packet hash mismatch, 3 sequence out of order, 4 previous hash mismatch.
    (if (i32.eqz (local.get $route_hash_matches)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $packet_hash_matches)) (then (return (i32.const 2))))
    (if (i64.ne (local.get $sequence) (local.get $expected_sequence)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $previous_hash_matches)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_relay_delivery_evidence_result")
    (param $worker_role i32) (param $worker_is_relay_id i32) (param $policy_hash_matches i32)
    (param $admission_route_matches i32) (param $delivery_from_worker i32) (param $delivery_to_recipient i32)
    (param $same_packet_hash i32) (param $message_policy_allows i32) (param $recipient_proof_ok i32)
    (param $receipt_input_matches i32) (param $receipt_output_matches i32)
    (result i32)
    ;; 0 ok, 1 wrong worker, 2 wrong relay, 3 policy mismatch, 4 route mismatch,
    ;; 5 wrong recipient, 6 packet mismatch, 7 message policy rejected, 8 invalid proof,
    ;; 9 input mismatch, 10 output mismatch.
    (if (i32.ne (local.get $worker_role) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $worker_is_relay_id) (local.get $delivery_from_worker))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $policy_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $admission_route_matches)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $delivery_to_recipient)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $same_packet_hash)) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $message_policy_allows)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $recipient_proof_ok)) (then (return (i32.const 8))))
    (if (i32.eqz (local.get $receipt_input_matches)) (then (return (i32.const 9))))
    (if (i32.eqz (local.get $receipt_output_matches)) (then (return (i32.const 10))))
    i32.const 0)

  (func (export "work_relay_transit_builder_result")
    (param $relay_path_len i32) (param $max_hops i32) (result i32)
    ;; 0 ok, 1 empty path, 2 too many hops. max_hops 0 means default 64.
    (local $limit i32)
    (local.set $limit (if (result i32) (i32.eqz (local.get $max_hops)) (then i32.const 64) (else local.get $max_hops)))
    (if (i32.eqz (local.get $relay_path_len)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $relay_path_len) (local.get $limit)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_relay_transit_hop_result")
    (param $index i32) (param $max_hops i32) (param $path_len i32)
    (param $relay_matches_path i32) (param $endpoint_matches i32) (param $packet_matches i32)
    (result i32)
    ;; 0 ok, 1 too many hops, 2 relay mismatch, 3 endpoint mismatch, 4 packet mismatch.
    (if (i32.or (i32.ge_u (local.get $index) (local.get $max_hops)) (i32.ge_u (local.get $index) (local.get $path_len)))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $relay_matches_path)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $endpoint_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $packet_matches)) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_relay_transit_settlement_result")
    (param $worker_role i32) (param $worker_is_relay_id i32) (param $admission_path_matches i32)
    (param $route_commitment_matches i32) (param $recipient_matches i32) (param $policy_hash_matches i32)
    (param $final_relay_delivered i32) (param $packet_matches i32) (param $message_policy_allows i32)
    (param $recipient_proof_ok i32) (param $bundle_ok i32) (param $hop_found i32) (param $receipt_matches_hop i32)
    (result i32)
    ;; 0 ok, 1 wrong worker, 2 wrong relay, 3 route mismatch, 4 wrong recipient,
    ;; 5 policy mismatch, 6 packet mismatch, 7 policy rejected, 8 proof invalid,
    ;; 9 invalid bundle, 10 missing hop, 11 receipt mismatch.
    (if (i32.ne (local.get $worker_role) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $worker_is_relay_id)) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (local.get $admission_path_matches) (local.get $route_commitment_matches))) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $recipient_matches)) (then (return (i32.const 4))))
    (if (i32.eqz (local.get $policy_hash_matches)) (then (return (i32.const 5))))
    (if (i32.eqz (local.get $final_relay_delivered)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $packet_matches)) (then (return (i32.const 6))))
    (if (i32.eqz (local.get $message_policy_allows)) (then (return (i32.const 7))))
    (if (i32.eqz (local.get $recipient_proof_ok)) (then (return (i32.const 8))))
    (if (i32.eqz (local.get $bundle_ok)) (then (return (i32.const 9))))
    (if (i32.eqz (local.get $hop_found)) (then (return (i32.const 10))))
    (if (i32.eqz (local.get $receipt_matches_hop)) (then (return (i32.const 11))))
    i32.const 0)

  (func (export "work_custody_kind_valid") (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_custody_result")
    (param $expected_root_nonzero i32) (param $kind i32) (param $ack_root_matches i32)
    (param $ack_kind_matches i32) (param $ack_bundle_ok i32)
    (result i32)
    ;; 0 ok, 1 invalid requirement, 2 ack mismatch, 3 ack invalid.
    (if (i32.eqz (i32.and (local.get $expected_root_nonzero) (call $work_custody_kind_valid_internal (local.get $kind)))) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $ack_root_matches) (local.get $ack_kind_matches))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $ack_bundle_ok)) (then (return (i32.const 3))))
    i32.const 0)

  (func $work_custody_kind_valid_internal (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_role_claim_result")
    (param $claim_sig_ok i32) (param $abi_ok i32) (param $range_ok i32) (param $units_used i64)
    (param $admission_matches i32) (param $not_expired i32) (param $receipt_matches i32)
    (param $duplicate_claim i32) (param $range_overlap i32)
    (result i32)
    ;; 0 ok, 1 invalid claim, 2 admission mismatch, 3 expired, 4 receipt mismatch,
    ;; 5 duplicate, 6 range overlap.
    (if (i32.eqz (i32.and (i32.and (local.get $claim_sig_ok) (local.get $abi_ok)) (local.get $range_ok))) (then (return (i32.const 1))))
    (if (i64.eqz (local.get $units_used)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $admission_matches)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $not_expired)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $receipt_matches)) (then (return (i32.const 4))))
    (if (local.get $duplicate_claim) (then (return (i32.const 5))))
    (if (local.get $range_overlap) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "work_ranges_overlap") (param $a_start i64) (param $a_end i64) (param $b_start i64) (param $b_end i64) (result i32)
    (i32.and (i64.le_u (local.get $a_start) (local.get $b_end)) (i64.le_u (local.get $b_start) (local.get $a_end))))

  (func (export "work_storage_role_accepts") (param $department i32) (param $work_type i32) (result i32)
    (i32.and
      (i32.or (i32.eq (local.get $department) (i32.const 4)) (i32.eq (local.get $department) (i32.const 5)))
      (i32.or (i32.eq (local.get $work_type) (i32.const 2)) (i32.eq (local.get $work_type) (i32.const 3)))))

  (func (export "work_storage_payload_kind_valid") (param $kind i32) (result i32)
    (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 3))))

  (func (export "work_store_request_result") (param $work_type i32) (param $shard_hash_matches i32) (result i32)
    ;; 0 ok, 1 wrong work type, 2 hash mismatch.
    (if (i32.ne (local.get $work_type) (i32.const 2)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $shard_hash_matches)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_retrieve_response_result") (param $shard_hash_matches i32) (result i32)
    (if (result i32) (local.get $shard_hash_matches) (then i32.const 0) (else i32.const 1)))

  (func $work_capability_kind_to_work_type (export "work_capability_kind_to_work_type") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 11))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 12))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 13))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 14))))
    i32.const 0)

  (func (export "work_capability_operation_matches_content") (param $operation i32) (param $content_type i32) (result i32)
    ;; content: opaque=0 control=1 video=2 audio=3 input=4 render=5 object=6.
    (if (i32.or (i32.eq (local.get $operation) (i32.const 1)) (i32.eq (local.get $operation) (i32.const 2)))
      (then (return (i32.or (i32.eq (local.get $content_type) (i32.const 1)) (i32.eq (local.get $content_type) (i32.const 0))))))
    (if (i32.and (i32.ge_u (local.get $operation) (i32.const 10)) (i32.le_u (local.get $operation) (i32.const 12)))
      (then (return (i32.eq (local.get $content_type) (i32.const 6)))))
    (if (i32.or (i32.eq (local.get $operation) (i32.const 20)) (i32.eq (local.get $operation) (i32.const 21)))
      (then (return (i32.or (i32.eq (local.get $content_type) (i32.const 2)) (i32.or (i32.eq (local.get $content_type) (i32.const 3)) (i32.eq (local.get $content_type) (i32.const 0)))))))
    (if (i32.eq (local.get $operation) (i32.const 30)) (then (return (i32.eq (local.get $content_type) (i32.const 4)))))
    (if (i32.eq (local.get $operation) (i32.const 40)) (then (return (i32.eq (local.get $content_type) (i32.const 5)))))
    i32.const 0)

  (func (export "work_capability_message_result")
    (param $message_department i32) (param $envelope_abi_ok i32) (param $kind i32)
    (param $operation_content_ok i32) (param $work_type_matches i32)
    (param $source_target_match i32) (param $payload_hash_matches i32)
    (result i32)
    ;; 0 ok, 1 unsupported department, 2 invalid shape, 3 hash/mapping mismatch.
    (if (i32.ne (local.get $message_department) (i32.const 7)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (local.get $envelope_abi_ok) (i32.ne (call $work_capability_kind_to_work_type (local.get $kind)) (i32.const 0))) (local.get $operation_content_ok))) (then (return (i32.const 2))))
    (if (i32.eqz (i32.and (i32.and (local.get $work_type_matches) (local.get $source_target_match)) (local.get $payload_hash_matches))) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "work_admitted_route_result")
    (param $request_ok i32) (param $admission_ok i32) (param $request_hash_matches i32)
    (param $user_matches i32) (param $admission_not_outliving_request i32)
    (param $relay_path_nonempty i32) (param $first_relay_matches i32) (param $recipient_not_in_relay_path i32)
    (result i32)
    ;; 0 ok, 1 invalid shape/signature, 2 hash mismatch, 3 expired, 4 wrong relay.
    (if (i32.eqz (i32.and (local.get $request_ok) (local.get $admission_ok))) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $request_hash_matches) (local.get $user_matches))) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $admission_not_outliving_request)) (then (return (i32.const 3))))
    (if (i32.eqz (i32.and (i32.and (local.get $relay_path_nonempty) (local.get $first_relay_matches)) (local.get $recipient_not_in_relay_path))) (then (return (i32.const 4))))
    i32.const 0)

  (func (export "work_message_against_route_result")
    (param $abi_ok i32) (param $source_matches i32) (param $target_matches i32)
    (param $relay_matches i32) (param $relay_first_matches i32)
    (param $department_matches i32) (param $work_type_matches i32)
    (result i32)
    ;; 0 ok, 1 invalid shape, 2 wrong relay/route.
    (if (i32.eqz (local.get $abi_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (i32.and (local.get $source_matches) (local.get $target_matches)) (i32.and (local.get $relay_matches) (local.get $relay_first_matches))) (i32.and (local.get $department_matches) (local.get $work_type_matches)))) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "work_receipt_against_route_result")
    (param $receipt_sig_ok i32) (param $request_matches i32) (param $admission_matches i32)
    (param $worker_target_matches i32) (param $worker_role_matches i32) (param $relay_matches i32)
    (param $claim i64) (param $admitted_budget i64)
    (result i32)
    ;; 0 ok, 1 invalid signature, 2 hash/route mismatch, 3 budget exceeded.
    (if (i32.eqz (local.get $receipt_sig_ok)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (i32.and (local.get $request_matches) (local.get $admission_matches)) (i32.and (i32.and (local.get $worker_target_matches) (local.get $worker_role_matches)) (local.get $relay_matches)))) (then (return (i32.const 2))))
    (if (i64.gt_u (local.get $claim) (local.get $admitted_budget)) (then (return (i32.const 3))))
    i32.const 0)

)
