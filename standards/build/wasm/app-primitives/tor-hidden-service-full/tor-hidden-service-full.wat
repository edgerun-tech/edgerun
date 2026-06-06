(module
  ;; Tor onion-service v3 and EdgeRun identity-fabric surface.
  ;; This module owns hidden-service state, onion address shape checks,
  ;; descriptor path construction, intro/rendezvous relay payloads, and
  ;; preallocated app memory records. Cryptographic checks are kept as WAT
  ;; dependencies in the protocol metadata, not as host shortcuts.
  (memory (export "memory") 4)

  (global $STANDARD_ID i32 (i32.const 300207))
  (global $ABI_VERSION i32 (i32.const 2))
  (global $OK i32 (i32.const 0))
  (global $ERR_INVALID i32 (i32.const -1))
  (global $ERR_BOUNDS i32 (i32.const -2))
  (global $ERR_UNSUPPORTED i32 (i32.const -3))

  (global $ONION_RAW_LEN i32 (i32.const 35))
  (global $ONION_ADDR_LEN i32 (i32.const 62))
  (global $ONION_BASE32_LEN i32 (i32.const 56))
  (global $HS_VERSION_V3 i32 (i32.const 3))
  (global $HS_DESC_LIFETIME_MIN i32 (i32.const 180))
  (global $HS_REPUBLISH_MIN i32 (i32.const 60))
  (global $HS_REPUBLISH_MAX i32 (i32.const 120))
  (global $HS_DESC_MAX_BYTES i32 (i32.const 50000))
  (global $HS_INTRO_MIN i32 (i32.const 3))
  (global $HS_INTRO_MAX i32 (i32.const 20))
  (global $HS_AUTH_KEY_LEN i32 (i32.const 32))
  (global $HS_ENC_KEY_LEN i32 (i32.const 32))
  (global $HS_COOKIE_LEN i32 (i32.const 20))
  (global $HS_SUBCRED_LEN i32 (i32.const 32))
  (global $HS_NTOR_HANDSHAKE_TYPE i32 (i32.const 2))
  (global $HS_NTOR_HANDSHAKE_LEN i32 (i32.const 84))

  (global $RELAY_ESTABLISH_INTRO i32 (i32.const 32))
  (global $RELAY_ESTABLISH_RENDEZVOUS i32 (i32.const 33))
  (global $RELAY_INTRODUCE1 i32 (i32.const 34))
  (global $RELAY_INTRODUCE2 i32 (i32.const 35))
  (global $RELAY_RENDEZVOUS1 i32 (i32.const 36))
  (global $RELAY_RENDEZVOUS2 i32 (i32.const 37))
  (global $RELAY_INTRO_ESTABLISHED i32 (i32.const 38))
  (global $RELAY_RENDEZVOUS_ESTABLISHED i32 (i32.const 39))
  (global $RELAY_INTRODUCE_ACK i32 (i32.const 40))

  (global $STATE_BASE i32 (i32.const 4096))
  (global $INTRO_BASE i32 (i32.const 8192))
  (global $INTRO_SIZE i32 (i32.const 128))
  (global $INTRO_MAX_RECORDS i32 (i32.const 20))
  (global $APP_MEMORY_BASE i32 (i32.const 32768))
  (global $APP_MEMORY_BYTES i32 (i32.const 131072))

  (data (i32.const 1024) "/tor/hs/3/")
  (data (i32.const 1040) "/tor/hs/3/publish")
  (data (i32.const 1072) ".onion")

  ;; State record:
  ;; 0 initialized, 4 role caps, 8 intro_count, 12 revision_counter,
  ;; 16 first_tp, 20 second_tp, 24 next_republish_minute,
  ;; 28 app_memory_base, 32 app_memory_len.

  (func $is_base32_onion (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
      (i32.and (i32.ge_u (local.get $c) (i32.const 50)) (i32.le_u (local.get $c) (i32.const 55)))))

  (func $put_u16be (param $p i32) (param $v i32)
    (i32.store8 (local.get $p) (i32.shr_u (local.get $v) (i32.const 8)))
    (i32.store8 (i32.add (local.get $p) (i32.const 1)) (local.get $v)))

  (func $put_u32le (param $p i32) (param $v i32)
    (i32.store (local.get $p) (local.get $v)))

  (func $intro_ptr (param $index i32) (result i32)
    (if (i32.ge_u (local.get $index) (global.get $INTRO_MAX_RECORDS)) (then (return (i32.const 0))))
    (i32.add (global.get $INTRO_BASE) (i32.mul (local.get $index) (global.get $INTRO_SIZE))))

  (func (export "proto_standard_id") (result i32) (global.get $STANDARD_ID))
  (func (export "proto_abi_version") (result i32) (global.get $ABI_VERSION))
  (func (export "simd_capabilities") (result i32) (i32.const 1))
  (func (export "tor_hs_onion_addr_len") (result i32) (global.get $ONION_ADDR_LEN))
  (func (export "tor_hs_desc_lifetime_minutes") (result i32) (global.get $HS_DESC_LIFETIME_MIN))
  (func (export "tor_hs_desc_max_bytes") (result i32) (global.get $HS_DESC_MAX_BYTES))
  (func (export "tor_hs_intro_min") (result i32) (global.get $HS_INTRO_MIN))
  (func (export "tor_hs_intro_max") (result i32) (global.get $HS_INTRO_MAX))
  (func (export "tor_hs_app_memory_base") (result i32) (global.get $APP_MEMORY_BASE))
  (func (export "tor_hs_app_memory_bytes") (result i32) (global.get $APP_MEMORY_BYTES))

  (func (export "tor_hs_full_init") (result i32)
    (memory.fill (global.get $STATE_BASE) (i32.const 0) (i32.const 256))
    (memory.fill (global.get $INTRO_BASE) (i32.const 0) (i32.mul (global.get $INTRO_MAX_RECORDS) (global.get $INTRO_SIZE)))
    (i32.store (global.get $STATE_BASE) (i32.const 1))
    (i32.store offset=4 (global.get $STATE_BASE) (i32.const 0x000000e2)) ;; guard | middle | hs-service | hs-intro | hs-rend
    (i32.store offset=28 (global.get $STATE_BASE) (global.get $APP_MEMORY_BASE))
    (i32.store offset=32 (global.get $STATE_BASE) (global.get $APP_MEMORY_BYTES))
    (global.get $OK))

  (func (export "tor_hs_role_caps") (result i32)
    (i32.load offset=4 (global.get $STATE_BASE)))

  (func (export "tor_hs_validate_onion_address") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (global.get $ONION_ADDR_LEN)) (then (return (global.get $ERR_INVALID))))
    (block $done
      (loop $chars
        (br_if $done (i32.ge_u (local.get $i) (global.get $ONION_BASE32_LEN)))
        (if (i32.eqz (call $is_base32_onion (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (global.get $ERR_INVALID))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $chars)))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 56))) (i32.const 46)) (then (return (global.get $ERR_INVALID))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 57))) (i32.const 111)) (then (return (global.get $ERR_INVALID))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 58))) (i32.const 110)) (then (return (global.get $ERR_INVALID))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 59))) (i32.const 105)) (then (return (global.get $ERR_INVALID))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 60))) (i32.const 111)) (then (return (global.get $ERR_INVALID))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 61))) (i32.const 110)) (then (return (global.get $ERR_INVALID))))
    (global.get $OK))

  (func $tor_hs_time_period (export "tor_hs_time_period") (param $valid_after_minute i32) (param $period_minutes i32) (param $offset_minutes i32) (result i32)
    (if (i32.eqz (local.get $period_minutes)) (then (return (global.get $ERR_INVALID))))
    (i32.div_u (i32.add (local.get $valid_after_minute) (local.get $offset_minutes)) (local.get $period_minutes)))

  (func (export "tor_hs_descriptor_slot") (param $now_minute i32) (param $srv_minute i32) (param $period_minutes i32) (param $slot i32) (result i32)
    (local $tp i32)
    (if (i32.gt_u (local.get $slot) (i32.const 1)) (then (return (global.get $ERR_INVALID))))
    (local.set $tp (call $tor_hs_time_period (local.get $now_minute) (local.get $period_minutes) (i32.const 0)))
    (if (i32.lt_u (local.get $now_minute) (local.get $srv_minute))
      (then (return (select (local.get $tp) (i32.sub (local.get $tp) (i32.const 1)) (local.get $slot)))))
    (select (i32.add (local.get $tp) (i32.const 1)) (local.get $tp) (local.get $slot)))

  (func (export "tor_hs_should_republish") (param $now_minute i32) (result i32)
    (i32.ge_u (local.get $now_minute) (i32.load offset=24 (global.get $STATE_BASE))))

  (func (export "tor_hs_schedule_republish") (param $now_minute i32) (param $jitter_minutes i32) (result i32)
    (local $delay i32)
    (local.set $delay (i32.add (global.get $HS_REPUBLISH_MIN) (i32.rem_u (local.get $jitter_minutes) (i32.add (i32.sub (global.get $HS_REPUBLISH_MAX) (global.get $HS_REPUBLISH_MIN)) (i32.const 1)))))
    (i32.store offset=24 (global.get $STATE_BASE) (i32.add (local.get $now_minute) (local.get $delay)))
    (i32.load offset=24 (global.get $STATE_BASE)))

  (func (export "tor_hs_build_fetch_path") (param $out i32) (param $blind_b64 i32) (param $blind_len i32) (result i32)
    (if (i32.gt_u (local.get $blind_len) (i32.const 128)) (then (return (global.get $ERR_BOUNDS))))
    (memory.copy (local.get $out) (i32.const 1024) (i32.const 10))
    (memory.copy (i32.add (local.get $out) (i32.const 10)) (local.get $blind_b64) (local.get $blind_len))
    (i32.add (i32.const 10) (local.get $blind_len)))

  (func (export "tor_hs_build_publish_path") (param $out i32) (result i32)
    (memory.copy (local.get $out) (i32.const 1040) (i32.const 17))
    (i32.const 17))

  (func (export "tor_hs_register_intro_point")
    (param $circ_id i32) (param $auth_key32 i32) (param $enc_key32 i32) (param $link_spec i32) (param $link_len i32) (result i32)
    (local $count i32) (local $p i32) (local $copy_len i32)
    (local.set $count (i32.load offset=8 (global.get $STATE_BASE)))
    (if (i32.ge_u (local.get $count) (global.get $INTRO_MAX_RECORDS)) (then (return (global.get $ERR_BOUNDS))))
    (local.set $p (call $intro_ptr (local.get $count)))
    (i32.store (local.get $p) (local.get $circ_id))
    (memory.copy (i32.add (local.get $p) (i32.const 4)) (local.get $auth_key32) (i32.const 32))
    (memory.copy (i32.add (local.get $p) (i32.const 36)) (local.get $enc_key32) (i32.const 32))
    (local.set $copy_len (select (i32.const 60) (local.get $link_len) (i32.gt_u (local.get $link_len) (i32.const 60))))
    (i32.store offset=68 (local.get $p) (local.get $copy_len))
    (if (local.get $copy_len)
      (then (memory.copy (i32.add (local.get $p) (i32.const 72)) (local.get $link_spec) (local.get $copy_len))))
    (i32.store offset=8 (global.get $STATE_BASE) (i32.add (local.get $count) (i32.const 1)))
    (local.get $count))

  (func (export "tor_hs_intro_count") (result i32)
    (i32.load offset=8 (global.get $STATE_BASE)))

  (func (export "tor_hs_intro_record_ptr") (param $index i32) (result i32)
    (call $intro_ptr (local.get $index)))

  (func (export "tor_hs_build_descriptor_record")
    (param $out i32) (param $revision i32) (param $lifetime_minutes i32) (param $intro_count i32) (result i32)
    (if (i32.gt_u (local.get $intro_count) (global.get $HS_INTRO_MAX)) (then (return (global.get $ERR_INVALID))))
    (call $put_u32le (local.get $out) (global.get $HS_VERSION_V3))
    (call $put_u32le (i32.add (local.get $out) (i32.const 4)) (local.get $revision))
    (call $put_u32le (i32.add (local.get $out) (i32.const 8)) (local.get $lifetime_minutes))
    (call $put_u32le (i32.add (local.get $out) (i32.const 12)) (local.get $intro_count))
    (i32.const 16))

  (func (export "tor_hs_build_intro_established") (param $out i32) (result i32)
    (i32.store8 (local.get $out) (global.get $RELAY_INTRO_ESTABLISHED))
    (i32.store8 (i32.add (local.get $out) (i32.const 1)) (i32.const 0))
    (i32.const 2))

  (func (export "tor_hs_build_rendezvous_established") (param $out i32) (result i32)
    (i32.store8 (local.get $out) (global.get $RELAY_RENDEZVOUS_ESTABLISHED))
    (i32.store8 (i32.add (local.get $out) (i32.const 1)) (i32.const 0))
    (i32.const 2))

  (func (export "tor_hs_build_introduce_ack") (param $out i32) (param $status i32) (result i32)
    (i32.store8 (local.get $out) (global.get $RELAY_INTRODUCE_ACK))
    (i32.store8 (i32.add (local.get $out) (i32.const 1)) (local.get $status))
    (i32.const 2))

  (func (export "tor_hs_build_establish_rendezvous") (param $out i32) (param $cookie20 i32) (result i32)
    (memory.copy (local.get $out) (local.get $cookie20) (global.get $HS_COOKIE_LEN))
    (global.get $HS_COOKIE_LEN))

  (func (export "tor_hs_parse_establish_rendezvous") (param $body i32) (param $body_len i32) (param $out_cookie20 i32) (result i32)
    (if (i32.lt_u (local.get $body_len) (global.get $HS_COOKIE_LEN)) (then (return (global.get $ERR_INVALID))))
    (memory.copy (local.get $out_cookie20) (local.get $body) (global.get $HS_COOKIE_LEN))
    (global.get $OK))

  (func (export "tor_hs_build_rendezvous1") (param $out i32) (param $cookie20 i32) (param $handshake i32) (param $handshake_len i32) (result i32)
    (if (i32.gt_u (local.get $handshake_len) (i32.const 477)) (then (return (global.get $ERR_BOUNDS))))
    (memory.copy (local.get $out) (local.get $cookie20) (global.get $HS_COOKIE_LEN))
    (memory.copy (i32.add (local.get $out) (global.get $HS_COOKIE_LEN)) (local.get $handshake) (local.get $handshake_len))
    (i32.add (global.get $HS_COOKIE_LEN) (local.get $handshake_len)))

  (func (export "tor_hs_parse_rendezvous1") (param $body i32) (param $body_len i32) (param $out_cookie20 i32) (param $out_handshake i32) (result i32)
    (if (i32.lt_u (local.get $body_len) (global.get $HS_COOKIE_LEN)) (then (return (global.get $ERR_INVALID))))
    (memory.copy (local.get $out_cookie20) (local.get $body) (global.get $HS_COOKIE_LEN))
    (memory.copy (local.get $out_handshake) (i32.add (local.get $body) (global.get $HS_COOKIE_LEN)) (i32.sub (local.get $body_len) (global.get $HS_COOKIE_LEN)))
    (i32.sub (local.get $body_len) (global.get $HS_COOKIE_LEN)))

  (func (export "tor_hs_parse_intro_status") (param $body i32) (param $body_len i32) (result i32)
    (if (i32.lt_u (local.get $body_len) (i32.const 1)) (then (return (global.get $ERR_INVALID))))
    (i32.load8_u (local.get $body)))
)

