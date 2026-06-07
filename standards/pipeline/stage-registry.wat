;; Pipeline Stage Registry — elem entries for stage dispatch table
  ;; Must be included at the END of the MANIFEST so all process_* functions are in scope.

  ;; Slots 0:  passthrough (defined in pipeline-core.wat)
  ;; Slots 1-2: hex encode/decode (from encoding-text.wat)
  (elem (i32.const 1) $process_hex_encode)
  (elem (i32.const 2) $process_hex_decode)
  ;; Slots 3-4: base64 standard encode/decode (from base64-encode-stage.wat / base64-decode-stage.wat)
  (elem (i32.const 3) $process_base64_encode)
  (elem (i32.const 4) $process_base64_decode)
  ;; Slot 5:  transport (not implemented)
  ;; Slots 6-9: mux/demux (from mux-core.wat)
  (elem (i32.const 6) $process_mux_static)
  (elem (i32.const 7) $process_demux_static)
  (elem (i32.const 8) $process_mux_dynamic)
  (elem (i32.const 9) $process_demux_dynamic)
  ;; Slots 10-12: WS frame/encode/decode (not implemented)
  ;; Slot 13: exec (deferred — needs stubs)
  ;; Slot 14: dashboard
  (elem (i32.const 14) $process_dashboard)
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

  ;; Slots 46-47: Edgerun compiler/interpreter stages
  (elem (i32.const 46) $process_edgerun_parse)
  (elem (i32.const 47) $process_edgerun_exec)

  ;; Slots 48-50: Queue/Buffer/CDC stages
  (elem (i32.const 48) $process_queue)
  (elem (i32.const 49) $process_buffer)
  (elem (i32.const 50) $process_cdc)
