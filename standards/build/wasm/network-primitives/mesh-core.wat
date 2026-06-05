(module
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
)
