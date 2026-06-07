;; Pipeline Stage Registry — elem entries for stage dispatch table
  ;; Must be included at the END of the MANIFEST so all process_* functions are in scope.
  ;; Most stages are auto-generated in pipeline-stages.wat by gen-stages.js from registry.json.

  ;; Slots 0:  passthrough (defined in pipeline-core.wat)
  ;; Slots 1-2: hex encode/decode (from encoding-text.wat)
  (elem (i32.const 1) $process_hex_encode)
  (elem (i32.const 2) $process_hex_decode)
  ;; Slots 3-4: base64 standard encode/decode (from base64-encode-stage.wat / base64-decode-stage.wat)
  (elem (i32.const 3) $process_base64_encode)
  (elem (i32.const 4) $process_base64_decode)
  ;; Slot 5:  transport (from net/socket-core.wat)
  (elem (i32.const 5) $process_transport)
  ;; Slots 6-9: mux/demux (from mux-core.wat)
  (elem (i32.const 6) $process_mux_static)
  (elem (i32.const 7) $process_demux_static)
  (elem (i32.const 8) $process_mux_dynamic)
  (elem (i32.const 9) $process_demux_dynamic)
  ;; Slots 10-12: WS frame/encode/decode
  (elem (i32.const 10) $process_ws_frame)
  (elem (i32.const 11) $process_ws_encode)
  (elem (i32.const 12) $process_ws_decode)
  ;; Slot 13: exec (from pipeline/wasm-exec-stage.wat)
  (elem (i32.const 13) $process_wasm_detect)
  (elem (i32.const 14) $process_wasm_load)
  ;; Slots 14: (removed — old dashboard deleted)
  ;; Slot 15: frame pacer (from frame-pacer.wat)
  (elem (i32.const 15) $process_frame_pacer)
  ;; Slots 16-20: hash stages
  (elem (i32.const 16) $process_sha256)
  (elem (i32.const 17) $process_hmac_sha256)
  (elem (i32.const 18) $process_sha1)
  (elem (i32.const 19) $process_sha512)
  (elem (i32.const 20) $process_sha384)
  ;; Slots 21-23: AES encrypt/decrypt
  (elem (i32.const 21) $process_aes128_gcm_encrypt)
  (elem (i32.const 22) $process_aes128_gcm_decrypt)
  (elem (i32.const 23) $process_aes128_ctr_xor)
  ;; Slots 24-26: checksum/hash
  (elem (i32.const 24) $process_djb2_hash)
  (elem (i32.const 25) $process_inet_checksum)
  (elem (i32.const 26) $process_crc32_bzip)

  ;; Slots 27-32: Compression stages (deflate/gzip/zlib encode/decode)
  (elem (i32.const 27) $process_deflate_decode)
  (elem (i32.const 28) $process_deflate_encode)
  (elem (i32.const 29) $process_gzip_decode)
  (elem (i32.const 30) $process_gzip_encode)
  (elem (i32.const 31) $process_zlib_decode)
  (elem (i32.const 32) $process_zlib_encode)
  ;; Slots 33: JSON parse
  (elem (i32.const 33) $process_json_parse)
  ;; Slots 34-35: Percent encode/decode
  (elem (i32.const 34) $process_percent_decode)
  (elem (i32.const 35) $process_percent_encode)
  ;; Slot 36: UTF-8 repair
  (elem (i32.const 36) $process_utf8_repair)
  ;; Slots 37-38: ASCII case conversion
  (elem (i32.const 37) $process_to_lower)
  (elem (i32.const 38) $process_to_upper)
  ;; Slot 39: Hex decode (compat)
  (elem (i32.const 39) $process_hex_decode_compat)
  ;; Slot 40: CP1252 decode
  (elem (i32.const 40) $process_cp1252_decode)
  ;; Slot 41: PEM compact base64
  (elem (i32.const 41) $process_pem_compact)
  ;; Slots 42-43: AES-128 block encrypt/decrypt
  (elem (i32.const 42) $process_aes128_encrypt)
  (elem (i32.const 43) $process_aes128_decrypt)
  ;; Slots 44-45: HTTP request line / status line parse
  (elem (i32.const 44) $process_http_request_line_parse)
  (elem (i32.const 45) $process_http_status_line_parse)
  ;; Slots 46-47: Edgerun compiler/interpreter stages
  (elem (i32.const 46) $process_edgerun_parse)
  (elem (i32.const 47) $process_edgerun_exec)
  ;; Slots 48-50: Queue/Buffer/CDC stages
  (elem (i32.const 48) $process_queue)
  (elem (i32.const 49) $process_buffer)
  (elem (i32.const 50) $process_cdc)
  ;; Slots 51-52: UUID format/parse
  (elem (i32.const 51) $process_uuid_format)
  (elem (i32.const 52) $process_uuid_parse)
  ;; Slot 53: Title case
  (elem (i32.const 53) $process_to_title_case)
  ;; Slot 54: CESU-8 to UTF-8
  (elem (i32.const 54) $process_cesu8_to_utf8)
  ;; Slots 55-56: Escape/unescape text
  (elem (i32.const 55) $process_escape_text)
  (elem (i32.const 56) $process_unescape_text)
  ;; Slots 57-58: HTTP date parse/format
  (elem (i32.const 57) $process_http_date_parse)
  (elem (i32.const 58) $process_http_date_format)
  ;; Slots 59-60: DNS header decode/encode
  (elem (i32.const 59) $process_dns_header_decode)
  (elem (i32.const 60) $process_dns_header_encode)
  ;; Slot 61: TLS record decode
  (elem (i32.const 61) $process_tls_record_decode)
  ;; Slot 62: HPACK huffman decode
  (elem (i32.const 62) $process_hpack_huffman_decode)
  ;; Slots 63-64: HTTP next header / chunk scan
  (elem (i32.const 63) $process_http_next_header)
  (elem (i32.const 64) $process_http_chunk_scan)
  ;; Slots 65-66: DNS name decompress / TLS ClientHello scan
  (elem (i32.const 65) $process_dns_name_decompress)
  (elem (i32.const 66) $process_tls_clienthello_scan)
  ;; Slots 67-68: Generic TLV decode/encode
  (elem (i32.const 67) $process_generic_tlv_decode)
  (elem (i32.const 68) $process_generic_tlv_encode)
  ;; Slots 69-70: Frame header decode/encode
  (elem (i32.const 69) $process_frame_header_decode)
  (elem (i32.const 70) $process_frame_header_encode)
  ;; Slots 71-72: DNS question next / RR next
  (elem (i32.const 71) $process_dns_question_next)
  (elem (i32.const 72) $process_dns_rr_next)
  ;; Slots 73-74: Endian read/write
  (elem (i32.const 73) $process_endian_read)
  (elem (i32.const 74) $process_endian_write)
  ;; Slots 75-76: RSA PKCS1 verify/emit
  (elem (i32.const 75) $process_rsa_pkcs1_verify)
  (elem (i32.const 76) $process_rsa_pkcs1_emit)
  ;; Slot 77: OAuth URL encode
  (elem (i32.const 77) $process_oauth_url_encode)
  ;; Slot 78: HTTP classify body framing
  (elem (i32.const 78) $process_http_classify_body)
  ;; Slot 79: URL scan
  (elem (i32.const 79) $process_url_scan)
  ;; Slot 80: X.509 certificate scan
  (elem (i32.const 80) $process_x509_cert_scan)
  ;; Slots 81-82: HTTP/2 + HTTP/3 frame header decode
  (elem (i32.const 81) $process_http2_frame_header_decode)
  (elem (i32.const 82) $process_http3_frame_header_decode)
  ;; Slots 83-84: TLS cert list scan + SNI host
  (elem (i32.const 83) $process_tls_cert_list_scan)
  (elem (i32.const 84) $process_tls_sni_host)
  ;; Slot 85: RFC 3339 timestamp parse
  (elem (i32.const 85) $process_rfc3339_parse)
  ;; Slot 86: HTTP lowercase header name
  (elem (i32.const 86) $process_http_lowercase_header)
  ;; Slot 87: TLS ALPN next
  (elem (i32.const 87) $process_tls_alpn_next)
  ;; Slot 88: DER SEQUENCE decode
  (elem (i32.const 88) $process_der_sequence_decode)
  ;; Slot 89: HTTP/1 validate header block
  (elem (i32.const 89) $process_http_validate_headers)
  ;; Slot 90: HTTP parse content-length
  (elem (i32.const 90) $process_http_parse_content_length)
  ;; Slots 91-93: DER integer/octet-string/time decode
  (elem (i32.const 91) $process_der_integer_decode)
  (elem (i32.const 92) $process_der_octet_string_decode)
  (elem (i32.const 93) $process_der_time_decode)
  ;; Slot 94: TLS certificate entry next
  (elem (i32.const 94) $process_tls_cert_entry_next)
  ;; Slot 95: OCI reference scan
  (elem (i32.const 95) $process_oci_reference_scan)
  ;; Slot 96: SSH authorized key scan
  (elem (i32.const 96) $process_ssh_auth_key_scan)
  ;; Slot 97: DER BIT STRING decode
  (elem (i32.const 97) $process_der_bit_string_decode)
  ;; Slot 98: UTF-8 scan
  (elem (i32.const 98) $process_utf8_scan)
  ;; Slot 99: HPACK header block scan
  (elem (i32.const 99) $process_hpack_header_block_scan)
  ;; Slot 100: WebSocket parse header
  (elem (i32.const 100) $process_ws_parse_header)
  ;; Slot 101: WebSocket write frame header
  (elem (i32.const 101) $process_ws_write_frame_header)
  ;; Slot 102: TLS DNS name normalize
  (elem (i32.const 102) $process_tls_dns_name_normalize)
  ;; Slot 103: DER OID root decode
  (elem (i32.const 103) $process_der_oid_root_decode)
  ;; Slot 104: DER OID next arc
  (elem (i32.const 104) $process_der_oid_next_arc)
  ;; Slot 105: JSON emit string
  (elem (i32.const 105) $process_json_emit_string)
  ;; Slot 106: PEM find boundaries
  (elem (i32.const 106) $process_pem_find_boundaries)
  ;; Slot 107: URI scan path query
  (elem (i32.const 107) $process_uri_scan_path_query)
  ;; Slot 108: form-urlencoded next pair
  (elem (i32.const 108) $process_form_urlencoded_next_pair)
  ;; Slot 109: Base64url encode
  (elem (i32.const 109) $process_base64url_encode)
  ;; Slot 110: Base64url decode
  (elem (i32.const 110) $process_base64url_decode)
  ;; Slot 111: MAC address scan
  (elem (i32.const 111) $process_mac_scan)
  ;; Slot 112: UTF-8 scan SIMD
  (elem (i32.const 112) $process_utf8_scan_simd)
  ;; Slot 113: TOML scan scalar
  (elem (i32.const 113) $process_toml_scan_scalar)
  ;; Slot 114: TOML scan key-value
  (elem (i32.const 114) $process_toml_scan_key_value)
  ;; Slot 115: YAML scan line
  (elem (i32.const 115) $process_yaml_scan_line)
  ;; Slot 116: OAuth JSON value parse
  (elem (i32.const 116) $process_oauth_json_value)
  ;; Slot 117: OAuth parse HTTP response
  (elem (i32.const 117) $process_oauth_parse_http_response)
  ;; Slot 118: SDK seed shape32
  (elem (i32.const 118) $process_sdk_seed_shape32)
  ;; Slot 119: CRC-32 (unrolled)
  (elem (i32.const 119) $process_crc32_unrolled)
  ;; Slot 120: Adler-32 checksum
  (elem (i32.const 120) $process_adler32_vec)
  ;; Slot 121: Base32hex encode
  (elem (i32.const 121) $process_base32hex_encode)
  ;; Slot 122: Base32hex decode
  (elem (i32.const 122) $process_base32hex_decode)
  ;; Slot 123: Varint decode
  (elem (i32.const 123) $process_varint_decode)
  ;; Slot 124: Varint encode
  (elem (i32.const 124) $process_varint_encode)
  ;; Slot 125: TLS record header encode
  (elem (i32.const 125) $process_tls_record_header_encode)
  ;; Slot 126: TLS handshake header decode
  (elem (i32.const 126) $process_tls_handshake_header_decode)
  ;; Slot 127: HTTP find CRLF
  (elem (i32.const 127) $process_http_find_crlf)
  ;; Slot 128: JSON unescape string
  (elem (i32.const 128) $process_json_unescape_string)
  ;; Slot 129: HTTP find double CRLF
  (elem (i32.const 129) $process_http_find_double_crlf)
  ;; Slot 130: TLS extension next
  (elem (i32.const 130) $process_tls_extension_next)
  ;; Slot 131: QPACK prefix integer decode
  (elem (i32.const 131) $process_qpack_prefix_int_decode)
  ;; Slot 132: QUIC varint decode
  (elem (i32.const 132) $process_quic_varint_decode)
  ;; Slot 133: QUIC varint encode
  (elem (i32.const 133) $process_quic_varint_encode)
  ;; Slot 134: YAML scan document
  (elem (i32.const 134) $process_yaml_scan_document)
  ;; Slot 135: Key:Value colon scan
  (elem (i32.const 135) $process_kv_colon_scan)
  ;; Slot 136: Unquote span
  (elem (i32.const 136) $process_unquote_span)
  ;; Slot 137: Bracket list next
  (elem (i32.const 137) $process_bracket_list_next)
  ;; Slot 138: Host:Port scan
  (elem (i32.const 138) $process_host_port_scan)
  ;; Slot 139: X25519 scalar multiply
  (elem (i32.const 139) $process_x25519_scalar_mult)
  ;; Slot 140: AES-256 encrypt
  (elem (i32.const 140) $process_aes256_encrypt)
  ;; Slots 141-143: UI stages
  (elem (i32.const 141) $process_ui_layout)
  (elem (i32.const 142) $process_ui_paint)
  (elem (i32.const 143) $process_ui_event)
  ;; Slot 144: WASM call
  (elem (i32.const 144) $process_wasm_call)
  (elem (i32.const 145) $process_edgerun_compile)
