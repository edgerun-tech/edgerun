(module
  ;; EdgeRun HTTP client portable semantics.
  ;; Codes:
  ;; version: 1=h1 2=h2 3=h3 4=best 5=h2_or_h1
  ;; route: 1=h1 2=h2 3=h3 4=race_h3_tcp 5=h2_then_h1
  ;; pool action: 1=reuse 2=drop 3=create 4=evict_oldest
  ;; result: 0=pending 1=ok 2=err 3=retry 4=redirect 5=close

  (func (export "proto_abi_version") (result i32) (i32.const 2))
  (func (export "proto_standard_id") (result i32) (i32.const 300135))

  (func (export "http_client_default_connect_timeout_secs") (result i32) (i32.const 10))
  (func (export "http_client_default_read_timeout_secs") (result i32) (i32.const 30))
  (func (export "http_client_default_redirect_limit") (result i32) (i32.const 10))
  (func (export "http1_pool_default_max_per_host") (result i32) (i32.const 6))
  (func (export "http1_pool_default_idle_timeout_secs") (result i32) (i32.const 30))
  (func (export "http1_pool_default_dns_timeout_secs") (result i32) (i32.const 5))
  (func (export "http2_max_body_size") (result i32) (i32.const 104857600))
  (func (export "http2_idle_ping_secs") (result i32) (i32.const 30))
  (func (export "http2_ping_timeout_secs") (result i32) (i32.const 10))
  (func (export "tls_session_cache_default_per_server") (result i32) (i32.const 4))

  (func (export "http_client_route")
    (param $version i32) (param $is_https i32) (param $h2_disabled i32) (param $h3_disabled i32)
    (result i32)
    (if (i32.eq (local.get $version) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $version) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $version) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $version) (i32.const 5))
      (then
        (if (local.get $h2_disabled) (then (return (i32.const 1))))
        (return (i32.const 5))))
    (if (local.get $is_https)
      (then
        (if (local.get $h3_disabled)
          (then
            (if (local.get $h2_disabled) (then (return (i32.const 1))))
            (return (i32.const 5)))
          (else (return (i32.const 4)))))
      (else
        (if (local.get $h2_disabled) (then (return (i32.const 1))))
        (return (i32.const 5))))
    (i32.const 0))

  (func (export "http_client_best_winner")
    (param $h3_ok i32) (param $tcp_ok i32) (param $tcp_first i32) (result i32)
    (if (i32.and (local.get $tcp_ok) (local.get $tcp_first))
      (then (return (i32.const 2))))
    (if (local.get $h3_ok) (then (return (i32.const 3))))
    (if (local.get $tcp_ok) (then (return (i32.const 2))))
    (i32.const 2))

  (func (export "http_redirect_action")
    (param $follow i32) (param $remaining i32) (param $status i32) (param $has_location i32) (param $method i32)
    (result i32)
    (if (i32.and
          (i32.and (local.get $follow) (i32.gt_u (local.get $remaining) (i32.const 0)))
          (i32.and
            (i32.and (i32.ge_u (local.get $status) (i32.const 300)) (i32.lt_u (local.get $status) (i32.const 400)))
            (local.get $has_location)))
      (then
        ;; 303 rewrites non-GET/non-HEAD to GET; other redirects keep method.
        (if (i32.and (i32.eq (local.get $status) (i32.const 303))
                     (i32.and (i32.ne (local.get $method) (i32.const 1)) (i32.ne (local.get $method) (i32.const 8))))
          (then (return (i32.const 6))))
        (return (i32.const 4))))
    (i32.const 1))

  (func (export "http1_pool_action")
    (param $pooled_count i32) (param $idle_expired i32) (param $response_allows_reuse i32) (param $current_count i32) (param $max_per_host i32)
    (result i32)
    (if (local.get $idle_expired) (then (return (i32.const 2))))
    (if (i32.gt_u (local.get $pooled_count) (i32.const 0)) (then (return (i32.const 1))))
    (if (i32.and (local.get $response_allows_reuse) (i32.ge_u (local.get $current_count) (local.get $max_per_host)))
      (then (return (i32.const 4))))
    (i32.const 3))

  (func (export "http1_response_allows_reuse")
    (param $connection_close i32) (param $body_fully_read i32) (result i32)
    (i32.and (i32.eqz (local.get $connection_close)) (local.get $body_fully_read)))

  (func (export "http1_default_port")
    (param $is_https i32) (param $explicit_port i32) (result i32)
    (if (i32.gt_u (local.get $explicit_port) (i32.const 0))
      (then (return (local.get $explicit_port))))
    (if (local.get $is_https) (then (return (i32.const 443))))
    (i32.const 80))

  (func (export "http_request_builder_header_mask")
    (param $has_host i32) (param $has_connection i32) (param $has_user_agent i32) (param $has_body i32)
    (result i32)
    ;; bit0 add Host, bit1 add keep-alive, bit2 add user-agent, bit3 set content-length.
    (i32.or
      (i32.or
        (select (i32.const 0) (i32.const 1) (local.get $has_host))
        (select (i32.const 0) (i32.const 2) (local.get $has_connection)))
      (i32.or
        (select (i32.const 0) (i32.const 4) (local.get $has_user_agent))
        (select (i32.const 8) (i32.const 0) (local.get $has_body)))))

  (func (export "http_decompress_gate")
    (param $auto_decompress i32) (param $encoding i32) (param $is_head i32) (result i32)
    ;; encoding: 0=none/unknown 1=gzip 2=zlib/deflate 3=br-reserved
    (if (i32.or (i32.eqz (local.get $auto_decompress)) (local.get $is_head))
      (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $encoding) (i32.const 1)) (i32.eq (local.get $encoding) (i32.const 2)))
      (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http2_next_client_stream_id")
    (param $current i32) (result i32)
    (i32.add (local.get $current) (i32.const 2)))

  (func (export "http2_request_end_stream")
    (param $body_present i32) (param $body_len i32) (result i32)
    (i32.or (i32.eqz (local.get $body_present)) (i32.eqz (local.get $body_len))))

  (func (export "http2_data_frame_count")
    (param $body_len i32) (param $max_frame_size i32) (result i32)
    (if (i32.eqz (local.get $body_len)) (then (return (i32.const 0))))
    (i32.div_u
      (i32.add (local.get $body_len) (i32.sub (local.get $max_frame_size) (i32.const 1)))
      (local.get $max_frame_size)))

  (func (export "http2_keepalive_action")
    (param $idle_elapsed i32) (param $ping_pending i32) (param $ping_timed_out i32) (param $incoming_frame i32)
    (result i32)
    ;; 0=none 1=send_ping 2=clear_ping 5=close
    (if (i32.and (local.get $ping_pending) (local.get $ping_timed_out)) (then (return (i32.const 5))))
    (if (i32.and (local.get $ping_pending) (local.get $incoming_frame)) (then (return (i32.const 2))))
    (if (i32.and (i32.eqz (local.get $ping_pending)) (local.get $idle_elapsed)) (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http3_initial_bidi_stream")
    (param $is_server i32) (result i32)
    (if (local.get $is_server) (then (return (i32.const 1))))
    (i32.const 0))

  (func (export "http3_initial_uni_stream")
    (param $is_server i32) (result i32)
    (if (local.get $is_server) (then (return (i32.const 3))))
    (i32.const 2))

  (func (export "http3_preface_stream_count") (result i32) (i32.const 3))

  (func (export "http3_uni_stream_role")
    (param $stream_type i32) (result i32)
    ;; 0=control 2=qpack-encoder 3=qpack-decoder 1=push 9=unknown
    (if (i32.eq (local.get $stream_type) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $stream_type) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $stream_type) (i32.const 3)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $stream_type) (i32.const 1)) (then (return (i32.const 1))))
    (i32.const 9))

  (func (export "http3_control_frame_allowed")
    (param $frame_type i32) (result i32)
    ;; SETTINGS=4, GOAWAY=7, MAX_PUSH_ID=13 are legal control stream frames here.
    (i32.or
      (i32.or (i32.eq (local.get $frame_type) (i32.const 4)) (i32.eq (local.get $frame_type) (i32.const 7)))
      (i32.eq (local.get $frame_type) (i32.const 13))))

  (func (export "http3_goaway_accept_new_stream")
    (param $going_away_id i64) (param $new_stream_id i64) (result i32)
    (i64.le_u (local.get $new_stream_id) (local.get $going_away_id)))

  (func (export "middleware_chain_index")
    (param $middleware_count i32) (param $call_depth i32) (result i32)
    ;; Chain applies first registered middleware outermost.
    (if (i32.ge_u (local.get $call_depth) (local.get $middleware_count))
      (then (return (i32.const -1))))
    (local.get $call_depth))

  (func (export "tls_session_ticket_action")
    (param $ticket_empty i32) (param $is_duplicate i32) (param $valid_count i32) (param $max_per_server i32)
    (result i32)
    ;; 0=skip 1=store 4=evict_oldest_then_store
    (if (i32.or (local.get $ticket_empty) (local.get $is_duplicate)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $valid_count) (local.get $max_per_server)) (then (return (i32.const 4))))
    (i32.const 1))

  (func (export "tls_obfuscated_ticket_age")
    (param $elapsed_ms i32) (param $age_add i32) (result i32)
    (i32.add (local.get $elapsed_ms) (local.get $age_add)))
)
