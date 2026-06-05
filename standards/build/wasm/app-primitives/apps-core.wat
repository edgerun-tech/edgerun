(module
  ;; OAuth app service semantics plundered from edgerun-oauth.
  ;; Result codes are local to this executable standard:
  ;; 0 ok/false, 1 true or primary error, higher values are ordered failures.

  (func (export "oauth_default_scope_count") (result i32)
    i32.const 3)

  (func (export "oauth_default_timeout_secs") (result i64)
    i64.const 300)

  (func (export "oauth_default_path_code") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 1) ;; /oauth2/device/code
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 2) ;; /oauth2/token
          (else
            (if (result i32) (i32.eq (local.get $which) (i32.const 3))
              (then i32.const 3) ;; /oauth2/authorize
              (else i32.const 0)))))))

  (func (export "oauth_credentials_expired") (param $has_expiry i32) (param $now i64) (param $grace i64) (param $expiry i64) (result i32)
    (if (result i32) (i32.eqz (local.get $has_expiry))
      (then i32.const 1)
      (else
        (i64.ge_u
          (i64.add (local.get $now) (local.get $grace))
          (local.get $expiry)))))

  (func (export "oauth_credentials_valid") (param $has_access i32) (param $has_expiry i32) (param $now i64) (param $grace i64) (param $expiry i64) (result i32)
    (i32.and
      (local.get $has_access)
      (i32.eqz
        (call $oauth_credentials_expired_impl
          (local.get $has_expiry)
          (local.get $now)
          (local.get $grace)
          (local.get $expiry)))))

  (func $oauth_credentials_expired_impl (param $has_expiry i32) (param $now i64) (param $grace i64) (param $expiry i64) (result i32)
    (if (result i32) (i32.eqz (local.get $has_expiry))
      (then i32.const 1)
      (else
        (i64.ge_u
          (i64.add (local.get $now) (local.get $grace))
          (local.get $expiry)))))

  (func (export "oauth_token_request_field_count")
    (param $client_secret i32) (param $device_code i32) (param $code i32)
    (param $redirect_uri i32) (param $code_verifier i32) (param $refresh_token i32)
    (param $scope i32) (result i32)
    (i32.add
      (i32.const 2)
      (i32.add
        (i32.add (local.get $client_secret) (local.get $device_code))
        (i32.add
          (i32.add (local.get $code) (local.get $redirect_uri))
          (i32.add
            (i32.add (local.get $code_verifier) (local.get $refresh_token))
            (local.get $scope))))))

  (func (export "oauth_token_response_field_count")
    (param $access i32) (param $token_type i32) (param $expires i32) (param $refresh i32)
    (param $id_token i32) (param $scope i32) (param $error i32) (param $error_desc i32)
    (result i32)
    (i32.add
      (i32.add
        (i32.add (local.get $access) (local.get $token_type))
        (i32.add (local.get $expires) (local.get $refresh)))
      (i32.add
        (i32.add (local.get $id_token) (local.get $scope))
        (i32.add (local.get $error) (local.get $error_desc)))))

  (func (export "oauth_device_response_result")
    (param $has_device_code i32) (param $has_user_code i32)
    (param $has_verification_uri i32) (param $has_complete_uri i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $has_device_code))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $has_user_code))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $has_verification_uri))
              (then i32.const 3)
              (else
                (if (result i32) (i32.eqz (local.get $has_complete_uri))
                  (then i32.const 4)
                  (else i32.const 0)))))))))

  (func (export "oauth_device_response_default") (param $which i32) (result i64)
    (if (result i64) (i32.eq (local.get $which) (i32.const 1))
      (then i64.const 600) ;; expires_in
      (else
        (if (result i64) (i32.eq (local.get $which) (i32.const 2))
          (then i64.const 5) ;; interval
          (else i64.const 0)))))

  (func (export "oauth_poll_error_action") (param $error_code i32) (param $interval_secs i64) (result i64)
    (if (result i64) (i32.eq (local.get $error_code) (i32.const 1))
      (then local.get $interval_secs) ;; authorization_pending: keep polling
      (else
        (if (result i64) (i32.eq (local.get $error_code) (i32.const 2))
          (then (i64.add (local.get $interval_secs) (i64.const 2))) ;; slow_down
          (else i64.const -1)))))

  (func (export "oauth_auth_url_field_count") (result i32)
    i32.const 7)

  (func (export "oauth_json_parse_object_result")
    (param $starts_object i32) (param $all_keys_strings i32) (param $has_trailing i32) (param $unterminated i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $starts_object))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $all_keys_strings))
          (then i32.const 2)
          (else
            (if (result i32) (local.get $unterminated)
              (then i32.const 3)
              (else
                (if (result i32) (local.get $has_trailing)
                  (then i32.const 4)
                  (else i32.const 0)))))))))

  (func (export "oauth_json_escape_code") (param $byte i32) (result i32)
    (if (result i32) (i32.eq (local.get $byte) (i32.const 34))
      (then i32.const 1) ;; quote
      (else
        (if (result i32) (i32.eq (local.get $byte) (i32.const 92))
          (then i32.const 2) ;; backslash
          (else
            (if (result i32) (i32.eq (local.get $byte) (i32.const 10))
              (then i32.const 3) ;; newline
              (else
                (if (result i32) (i32.lt_u (local.get $byte) (i32.const 32))
                  (then i32.const 4) ;; unicode control escape
                  (else i32.const 0)))))))))

  (func (export "oauth_jwt_parse_result") (param $part_count i32) (param $header_ok i32) (param $payload_ok i32) (result i32)
    (if (result i32) (i32.ne (local.get $part_count) (i32.const 3))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $header_ok))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $payload_ok))
              (then i32.const 3)
              (else i32.const 0)))))))

  (func (export "oauth_jwt_payload_expired") (param $now i64) (param $grace i64) (param $exp i64) (result i32)
    (i64.ge_u (i64.add (local.get $now) (local.get $grace)) (local.get $exp)))

  (func (export "oauth_jwt_aud_valid") (param $aud_empty i32) (param $contains_expected i32) (result i32)
    (i32.or (local.get $aud_empty) (local.get $contains_expected)))

  (func (export "oauth_jwt_nonce_valid") (param $has_expected i32) (param $has_nonce i32) (param $matches i32) (result i32)
    (if (result i32) (i32.eqz (local.get $has_expected))
      (then i32.const 1)
      (else (i32.and (local.get $has_nonce) (local.get $matches)))))

  (func (export "oauth_jwt_at_hash_valid") (param $has_at_hash i32) (param $matches i32) (result i32)
    (if (result i32) (i32.eqz (local.get $has_at_hash))
      (then i32.const 1)
      (else local.get $matches)))

  (func (export "oauth_jwt_verify_dispatch") (param $alg i32) (param $verifier i32) (param $sig_matches i32) (result i32)
    (if (result i32) (i32.eqz (local.get $alg))
      (then i32.const 4) ;; unsupported
      (else
        (if (result i32) (i32.ne (local.get $alg) (local.get $verifier))
          (then i32.const 1) ;; verifier kind mismatch
          (else
            (if (result i32) (i32.eqz (local.get $sig_matches))
              (then i32.const 2)
              (else i32.const 0)))))))

  (func (export "oauth_jwk_verifier_result")
    (param $kty i32) (param $curve_ok i32) (param $has_x i32) (param $has_y i32)
    (param $has_n i32) (param $has_e i32) (param $has_k i32)
    (result i32)
    (if (result i32) (i32.eq (local.get $kty) (i32.const 1)) ;; EC
      (then
        (if (result i32) (i32.eqz (local.get $curve_ok))
          (then i32.const 2)
          (else
            (if (result i32) (i32.and (local.get $has_x) (local.get $has_y))
              (then i32.const 0)
              (else i32.const 3)))))
      (else
        (if (result i32) (i32.eq (local.get $kty) (i32.const 2)) ;; RSA
          (then
            (if (result i32) (local.get $has_n)
              (then i32.const 0)
              (else i32.const 4)))
          (else
            (if (result i32) (i32.eq (local.get $kty) (i32.const 3)) ;; oct
              (then
                (if (result i32) (local.get $has_k)
                  (then i32.const 0)
                  (else i32.const 5)))
              (else i32.const 1)))))))

  (func (export "oauth_constant_time_eq") (param $len_a i32) (param $len_b i32) (param $diff i32) (result i32)
    (i32.and (i32.eq (local.get $len_a) (local.get $len_b)) (i32.eqz (local.get $diff))))

  (func (export "oauth_secret_namespace_code") (param $collection_default i32) (param $key_default i32) (result i32)
    (if (result i32) (i32.and (local.get $collection_default) (local.get $key_default))
      (then i32.const 1) ;; /org/freedesktop/secrets/collections/default + default
      (else i32.const 0)))

  ;; Exchange API semantics plundered from edgerun-exchange-api.

  (func (export "exchange_id_valid") (param $prefix_ok i32) (param $suffix_len i32) (param $suffix_hex i32) (result i32)
    (i32.and
      (local.get $prefix_ok)
      (i32.and
        (i32.eq (local.get $suffix_len) (i32.const 32))
        (local.get $suffix_hex))))

  (func (export "exchange_mode_code") (param $mode i32) (result i32)
    (if (result i32) (i32.eq (local.get $mode) (i32.const 2))
      (then i32.const 2) ;; floating
      (else i32.const 1))) ;; instant/default

  (func (export "exchange_amount_side_code") (param $side i32) (result i32)
    (if (result i32) (i32.eq (local.get $side) (i32.const 2))
      (then i32.const 2) ;; pay
      (else i32.const 1))) ;; settlement/default

  (func (export "exchange_quote_input_result")
    (param $has_settlement i32) (param $has_settlement_symbol i32) (param $has_settlement_network i32)
    (param $has_pay i32) (param $has_pay_symbol i32) (param $has_pay_network i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $has_settlement))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $has_settlement_symbol))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $has_settlement_network))
              (then i32.const 3)
              (else
                (if (result i32) (i32.eqz (local.get $has_pay))
                  (then i32.const 4)
                  (else
                    (if (result i32) (i32.eqz (local.get $has_pay_symbol))
                      (then i32.const 5)
                      (else
                        (if (result i32) (i32.eqz (local.get $has_pay_network))
                          (then i32.const 6)
                          (else i32.const 0)))))))))))))

  (func (export "exchange_quote_amounts_present") (param $amount_side i32) (param $has_amount i32) (result i32)
    (if (result i32) (i32.eqz (local.get $has_amount))
      (then i32.const 0)
      (else
        (if (result i32) (i32.eq (local.get $amount_side) (i32.const 2))
          (then i32.const 2) ;; pay_amount gets amount
          (else i32.const 1))))) ;; settlement_amount gets amount

  (func (export "exchange_payment_request_input_result")
    (param $has_settlement i32) (param $has_amount i32) (param $has_expires i32) (param $expires_future i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $has_settlement))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $has_amount))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $has_expires))
              (then i32.const 3)
              (else
                (if (result i32) (i32.eqz (local.get $expires_future))
                  (then i32.const 4)
                  (else i32.const 0)))))))))

  (func (export "exchange_payment_quote_result") (param $request_exists i32) (param $request_expired i32) (param $pay_available i32) (result i32)
    (if (result i32) (i32.eqz (local.get $request_exists))
      (then i32.const 1)
      (else
        (if (result i32) (local.get $request_expired)
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $pay_available))
              (then i32.const 3)
              (else i32.const 0)))))))

  (func (export "exchange_order_input_result") (param $has_quote_id i32) (param $quote_id_valid i32) (param $has_destination i32) (result i32)
    (if (result i32) (i32.eqz (local.get $has_quote_id))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $quote_id_valid))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $has_destination))
              (then i32.const 3)
              (else i32.const 0)))))))

  (func (export "exchange_order_status_str_code") (param $status i32) (result i32)
    (if (result i32) (i32.and (i32.ge_s (local.get $status) (i32.const 1)) (i32.le_s (local.get $status) (i32.const 19)))
      (then local.get $status)
      (else i32.const 0)))

  (func (export "exchange_record_provider_status_result")
    (param $provider_matches i32) (param $current_known i32) (param $next_known i32)
    (param $same_status i32) (param $transition_allowed i32) (param $terminal i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $provider_matches))
      (then i32.const 1) ;; manual review: provider_status_mismatch
      (else
        (if (result i32) (local.get $same_status)
          (then i32.const 0)
          (else
            (if (result i32) (i32.eqz (local.get $current_known))
              (then i32.const 2)
              (else
                (if (result i32) (i32.eqz (local.get $next_known))
                  (then i32.const 3)
                  (else
                    (if (result i32) (i32.eqz (local.get $transition_allowed))
                      (then i32.const 4)
                      (else
                        (if (result i32) (local.get $terminal)
                          (then i32.const 6) ;; changed plus terminal event
                          (else i32.const 5)))))))))))))

  (func (export "exchange_route_code") (param $method i32) (param $path i32) (param $path_len i32) (result i32)
    (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 2)) (i32.eq (local.get $path) (i32.const 1)))
      (then i32.const 1) ;; POST /v1/quote
      (else
        (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 2)) (i32.eq (local.get $path) (i32.const 2)))
          (then i32.const 2) ;; POST /v1/payment-request
          (else
            (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 2)) (i32.eq (local.get $path) (i32.const 3)))
              (then i32.const 3) ;; POST /v1/payment-request/:id/quote
              (else
                (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 1)) (i32.eq (local.get $path) (i32.const 4)))
                  (then i32.const 4) ;; GET /v1/payment-request/:id
                  (else
                    (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 2)) (i32.eq (local.get $path) (i32.const 5)))
                      (then i32.const 5) ;; POST /v1/order
                      (else
                        (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 2)) (i32.eq (local.get $path) (i32.const 6)))
                          (then i32.const 6) ;; POST /v1/order/:id/refresh
                          (else
                            (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 1)) (i32.eq (local.get $path) (i32.const 7)))
                              (then i32.const 7) ;; GET /v1/assets
                              (else
                                (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 1)) (i32.eq (local.get $path) (i32.const 8)))
                                  (then i32.const 8) ;; GET /health
                                  (else
                                    (if (result i32) (i32.and (i32.eq (local.get $method) (i32.const 1)) (i32.eq (local.get $path) (i32.const 9)))
                                      (then i32.const 9) ;; GET /v1/order/:id
                                      (else i32.const 0)))))))))))))))))))

  (func (export "exchange_assets_catalog_count") (result i32)
    i32.const 5)

  (func (export "exchange_health_status") (result i32)
    i32.const 2) ;; degraded; provider health checks not implemented

  ;; Tor bench semantics plundered from edgerun-tor-bench.

  (func (export "tor_cell_len") (result i32)
    i32.const 514)

  (func (export "tor_relay_payload_len") (result i32)
    i32.const 498)

  (func (export "tor_relay_cell_payload_len") (result i32)
    i32.const 509)

  (func (export "tor_command_code") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 5)
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 6)
          (else
            (if (result i32) (i32.eq (local.get $which) (i32.const 3))
              (then i32.const 10)
              (else
                (if (result i32) (i32.eq (local.get $which) (i32.const 4))
                  (then i32.const 11)
                  (else
                    (if (result i32) (i32.eq (local.get $which) (i32.const 5))
                      (then i32.const 3)
                      (else
                        (if (result i32) (i32.eq (local.get $which) (i32.const 6))
                          (then i32.const 7)
                          (else
                            (if (result i32) (i32.eq (local.get $which) (i32.const 7))
                              (then i32.const 129)
                              (else
                                (if (result i32) (i32.eq (local.get $which) (i32.const 8))
                                  (then i32.const 130)
                                  (else
                                    (if (result i32) (i32.eq (local.get $which) (i32.const 9))
                                      (then i32.const 131)
                                      (else i32.const 0)))))))))))))))))))

  (func (export "tor_relay_command_code") (param $which i32) (result i32)
    (if (result i32) (i32.eq (local.get $which) (i32.const 1))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eq (local.get $which) (i32.const 2))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eq (local.get $which) (i32.const 3))
              (then i32.const 14)
              (else
                (if (result i32) (i32.eq (local.get $which) (i32.const 4))
                  (then i32.const 15)
                  (else
                    (if (result i32) (i32.eq (local.get $which) (i32.const 5))
                      (then i32.const 33)
                      (else
                        (if (result i32) (i32.eq (local.get $which) (i32.const 6))
                          (then i32.const 34)
                          (else
                            (if (result i32) (i32.eq (local.get $which) (i32.const 7))
                              (then i32.const 37)
                              (else i32.const 0)))))))))))))))

  (func (export "tor_fixed_cell_len_for_body") (param $body_len i32) (result i32)
    (if (result i32) (i32.gt_u (local.get $body_len) (i32.const 509))
      (then i32.const -1)
      (else i32.const 514)))

  (func (export "tor_var_cell_len_v0") (param $payload_len i32) (result i32)
    (i32.add (i32.const 5) (local.get $payload_len)))

  (func (export "tor_var_cell_len_v3") (param $payload_len i32) (result i32)
    (i32.add (i32.const 7) (local.get $payload_len)))

  (func (export "tor_read_any_cell_body_kind") (param $cmd i32) (result i32)
    (if (result i32) (i32.or (i32.ge_u (local.get $cmd) (i32.const 128)) (i32.eq (local.get $cmd) (i32.const 7)))
      (then i32.const 1) ;; variable length: read len + body
      (else i32.const 2))) ;; fixed: read 509 payload bytes

  (func (export "tor_parse_versions_result") (param $payload_len i32) (param $best_supported i32) (result i32)
    (if (result i32) (i32.ne (i32.rem_u (local.get $payload_len) (i32.const 2)) (i32.const 0))
      (then i32.const 1)
      (else
        (if (result i32) (i32.eqz (local.get $best_supported))
          (then i32.const 2)
          (else local.get $best_supported)))))

  (func (export "tor_base64_decode_result") (param $len_mod4 i32) (param $invalid_char i32) (result i32)
    (if (result i32) (i32.eq (local.get $len_mod4) (i32.const 1))
      (then i32.const 1)
      (else
        (if (result i32) (local.get $invalid_char)
          (then i32.const 2)
          (else i32.const 0)))))

  (func (export "tor_base32_decode_result") (param $invalid_char i32) (param $decoded_len i32) (param $expect_onion i32) (result i32)
    (if (result i32) (local.get $invalid_char)
      (then i32.const 1)
      (else
        (if (result i32) (i32.and (local.get $expect_onion) (i32.ne (local.get $decoded_len) (i32.const 35)))
          (then i32.const 2)
          (else i32.const 0)))))

  (func (export "tor_consensus_line_action") (param $line i32) (result i32)
    (if (result i32) (i32.eq (local.get $line) (i32.const 1))
      (then i32.const 1) ;; r line starts relay record
      (else
        (if (result i32) (i32.eq (local.get $line) (i32.const 2))
          (then i32.const 2) ;; a line fills missing address
          (else
            (if (result i32) (i32.eq (local.get $line) (i32.const 3))
              (then i32.const 3) ;; ntor-onion-key saves onion key
              (else
                (if (result i32) (i32.eq (local.get $line) (i32.const 4))
                  (then i32.const 4) ;; s line finalizes relay
                  (else i32.const 0)))))))))

  (func (export "tor_consensus_relay_accept") (param $ident_len i32) (param $or_port i32) (param $has_addr i32) (result i32)
    (i32.and
      (i32.eq (local.get $ident_len) (i32.const 20))
      (i32.and (i32.gt_u (local.get $or_port) (i32.const 0)) (local.get $has_addr))))

  (func (export "tor_relay_cell_data_len") (param $input_len i32) (result i32)
    (if (result i32) (i32.gt_u (local.get $input_len) (i32.const 498))
      (then i32.const 498)
      (else local.get $input_len)))

  (func (export "tor_decrypt_relay_result")
    (param $cell_len_ok i32) (param $dlen i32) (param $digest_matches i32) (param $expected_cmd i32) (param $actual_cmd i32)
    (result i32)
    (if (result i32) (i32.eqz (local.get $cell_len_ok))
      (then i32.const 1)
      (else
        (if (result i32) (i32.gt_u (i32.add (local.get $dlen) (i32.const 11)) (i32.const 498))
          (then i32.const 2)
          (else
            (if (result i32) (i32.eqz (local.get $digest_matches))
              (then i32.const 3)
              (else
                (if (result i32) (i32.ne (local.get $expected_cmd) (local.get $actual_cmd))
                  (then i32.const 4)
                  (else i32.const 0)))))))))

  (func (export "tor_authenticate_body_len") (result i32)
    i32.const 356) ;; type(2) + length(2) + AUTH0003 body(352)

  (func (export "tor_percentile_index") (param $len i32) (param $pct_times_100 i32) (result i32)
    (if (result i32) (i32.eqz (local.get $len))
      (then i32.const 0)
      (else
        (i32.div_u
          (i32.add
            (i32.mul
              (local.get $pct_times_100)
              (i32.sub (local.get $len) (i32.const 1)))
            (i32.const 5000))
          (i32.const 10000)))))
)
