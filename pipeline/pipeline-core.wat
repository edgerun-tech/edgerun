;; Pipeline Core — pipeline_run + dispatch table

    ;; Standard ID removed — merged into single module

  ;; ── Stage dispatch table ──
  (table (export "stage_table") 146 funcref)

  ;; ── Stage type constants (dispatch table indices) ──
  (global (export "STAGE_PASSTHROUGH") i32 (i32.const 0))
  (global (export "STAGE_HEX_ENCODE") i32 (i32.const 1))
  (global (export "STAGE_HEX_DECODE") i32 (i32.const 2))
  (global (export "STAGE_B64_ENCODE") i32 (i32.const 3))
  (global (export "STAGE_B64_DECODE") i32 (i32.const 4))
  (global (export "STAGE_TRANSPORT") i32 (i32.const 5))
  (global (export "STAGE_MUX_STATIC") i32 (i32.const 6))
  (global (export "STAGE_DEMUX_STATIC") i32 (i32.const 7))
  (global (export "STAGE_MUX_DYNAMIC") i32 (i32.const 8))
  (global (export "STAGE_DEMUX_DYNAMIC") i32 (i32.const 9))
  (global (export "STAGE_WS_FRAME") i32 (i32.const 10))
  (global (export "STAGE_WS_ENCODE") i32 (i32.const 11))
  (global (export "STAGE_WS_DECODE") i32 (i32.const 12))
  (global (export "STAGE_WASM_DETECT") i32 (i32.const 13))
  (global (export "STAGE_WASM_LOAD") i32 (i32.const 14))
  (global (export "STAGE_FRAME_PACER") i32 (i32.const 15))
  (global (export "STAGE_SHA256") i32 (i32.const 16))
  (global (export "STAGE_HMAC_SHA256") i32 (i32.const 17))
  (global (export "STAGE_SHA1") i32 (i32.const 18))
  (global (export "STAGE_SHA512") i32 (i32.const 19))
  (global (export "STAGE_SHA384") i32 (i32.const 20))
  (global (export "STAGE_AES128_GCM_ENCRYPT") i32 (i32.const 21))
  (global (export "STAGE_AES128_GCM_DECRYPT") i32 (i32.const 22))
  (global (export "STAGE_AES128_CTR_XOR") i32 (i32.const 23))
  (global (export "STAGE_DJB2_HASH") i32 (i32.const 24))
  (global (export "STAGE_INET_CHECKSUM") i32 (i32.const 25))
  (global (export "STAGE_CRC32_BZIP") i32 (i32.const 26))
  (global (export "STAGE_DEFLATE_DECODE") i32 (i32.const 27))
  (global (export "STAGE_DEFLATE_ENCODE") i32 (i32.const 28))
  (global (export "STAGE_GZIP_DECODE") i32 (i32.const 29))
  (global (export "STAGE_GZIP_ENCODE") i32 (i32.const 30))
  (global (export "STAGE_ZLIB_DECODE") i32 (i32.const 31))
  (global (export "STAGE_ZLIB_ENCODE") i32 (i32.const 32))
  (global (export "STAGE_JSON_PARSE") i32 (i32.const 33))
  (global (export "STAGE_PERCENT_DECODE") i32 (i32.const 34))
  (global (export "STAGE_PERCENT_ENCODE") i32 (i32.const 35))
  (global (export "STAGE_UTF8_REPAIR") i32 (i32.const 36))
  (global (export "STAGE_TO_LOWER") i32 (i32.const 37))
  (global (export "STAGE_TO_UPPER") i32 (i32.const 38))
  (global (export "STAGE_HEX_DECODE_COMPAT") i32 (i32.const 39))
  (global (export "STAGE_CP1252_DECODE") i32 (i32.const 40))
  (global (export "STAGE_PEM_COMPACT") i32 (i32.const 41))
  (global (export "STAGE_AES128_ENCRYPT") i32 (i32.const 42))
  (global (export "STAGE_AES128_DECRYPT") i32 (i32.const 43))
  (global (export "STAGE_HTTP_REQUEST_LINE_PARSE") i32 (i32.const 44))
  (global (export "STAGE_HTTP_STATUS_LINE_PARSE") i32 (i32.const 45))
  (global (export "STAGE_EDGERUN_PARSE") i32 (i32.const 46))
  (global (export "STAGE_EDGERUN_EXEC") i32 (i32.const 47))
  (global (export "STAGE_QUEUE") i32 (i32.const 48))
  (global (export "STAGE_BUFFER") i32 (i32.const 49))
  (global (export "STAGE_CDC") i32 (i32.const 50))
  (global (export "STAGE_UUID_FORMAT") i32 (i32.const 51))
  (global (export "STAGE_UUID_PARSE") i32 (i32.const 52))
  (global (export "STAGE_TO_TITLE_CASE") i32 (i32.const 53))
  (global (export "STAGE_CESU8_TO_UTF8") i32 (i32.const 54))
  (global (export "STAGE_ESCAPE_TEXT") i32 (i32.const 55))
  (global (export "STAGE_UNESCAPE_TEXT") i32 (i32.const 56))
  (global (export "STAGE_HTTP_DATE_PARSE") i32 (i32.const 57))
  (global (export "STAGE_HTTP_DATE_FORMAT") i32 (i32.const 58))
  (global (export "STAGE_DNS_HEADER_DECODE") i32 (i32.const 59))
  (global (export "STAGE_DNS_HEADER_ENCODE") i32 (i32.const 60))
  (global (export "STAGE_TLS_RECORD_DECODE") i32 (i32.const 61))
  (global (export "STAGE_HPACK_HUFFMAN_DECODE") i32 (i32.const 62))
  (global (export "STAGE_HTTP_NEXT_HEADER") i32 (i32.const 63))
  (global (export "STAGE_HTTP_CHUNK_SCAN") i32 (i32.const 64))
  (global (export "STAGE_DNS_NAME_DECOMPRESS") i32 (i32.const 65))
  (global (export "STAGE_TLS_CLIENTHELLO_SCAN") i32 (i32.const 66))
  (global (export "STAGE_GENERIC_TLV_DECODE") i32 (i32.const 67))
  (global (export "STAGE_GENERIC_TLV_ENCODE") i32 (i32.const 68))
  (global (export "STAGE_FRAME_HEADER_DECODE") i32 (i32.const 69))
  (global (export "STAGE_FRAME_HEADER_ENCODE") i32 (i32.const 70))
  (global (export "STAGE_DNS_QUESTION_NEXT") i32 (i32.const 71))
  (global (export "STAGE_DNS_RR_NEXT") i32 (i32.const 72))
  (global (export "STAGE_ENDIAN_READ") i32 (i32.const 73))
  (global (export "STAGE_ENDIAN_WRITE") i32 (i32.const 74))
  (global (export "STAGE_RSA_PKCS1_VERIFY") i32 (i32.const 75))
  (global (export "STAGE_RSA_PKCS1_EMIT") i32 (i32.const 76))
  (global (export "STAGE_OAUTH_URL_ENCODE") i32 (i32.const 77))
  (global (export "STAGE_HTTP_CLASSIFY_BODY") i32 (i32.const 78))
  (global (export "STAGE_URL_SCAN") i32 (i32.const 79))
  (global (export "STAGE_X509_CERT_SCAN") i32 (i32.const 80))
  (global (export "STAGE_HTTP2_FRAME_HEADER_DECODE") i32 (i32.const 81))
  (global (export "STAGE_HTTP3_FRAME_HEADER_DECODE") i32 (i32.const 82))
  (global (export "STAGE_TLS_CERT_LIST_SCAN") i32 (i32.const 83))
  (global (export "STAGE_TLS_SNI_HOST") i32 (i32.const 84))
  (global (export "STAGE_RFC3339_PARSE") i32 (i32.const 85))
  (global (export "STAGE_HTTP_LOWERCASE_HEADER") i32 (i32.const 86))
  (global (export "STAGE_TLS_ALPN_NEXT") i32 (i32.const 87))
  (global (export "STAGE_DER_SEQUENCE_DECODE") i32 (i32.const 88))
  (global (export "STAGE_HTTP_VALIDATE_HEADERS") i32 (i32.const 89))
  (global (export "STAGE_HTTP_PARSE_CONTENT_LENGTH") i32 (i32.const 90))
  (global (export "STAGE_DER_INTEGER_DECODE") i32 (i32.const 91))
  (global (export "STAGE_DER_OCTET_STRING_DECODE") i32 (i32.const 92))
  (global (export "STAGE_DER_TIME_DECODE") i32 (i32.const 93))
  (global (export "STAGE_TLS_CERT_ENTRY_NEXT") i32 (i32.const 94))
  (global (export "STAGE_OCI_REFERENCE_SCAN") i32 (i32.const 95))
  (global (export "STAGE_SSH_AUTH_KEY_SCAN") i32 (i32.const 96))
  (global (export "STAGE_DER_BIT_STRING_DECODE") i32 (i32.const 97))
  (global (export "STAGE_UTF8_SCAN") i32 (i32.const 98))
  (global (export "STAGE_HPACK_HEADER_BLOCK_SCAN") i32 (i32.const 99))
  (global (export "STAGE_WS_PARSE_HEADER") i32 (i32.const 100))
  (global (export "STAGE_WS_WRITE_FRAME_HEADER") i32 (i32.const 101))
  (global (export "STAGE_TLS_DNS_NAME_NORMALIZE") i32 (i32.const 102))
  (global (export "STAGE_DER_OID_ROOT_DECODE") i32 (i32.const 103))
  (global (export "STAGE_DER_OID_NEXT_ARC") i32 (i32.const 104))
  (global (export "STAGE_JSON_EMIT_STRING") i32 (i32.const 105))
  (global (export "STAGE_PEM_FIND_BOUNDARIES") i32 (i32.const 106))
  (global (export "STAGE_URI_SCAN_PATH_QUERY") i32 (i32.const 107))
  (global (export "STAGE_FORM_URLENCODED_NEXT_PAIR") i32 (i32.const 108))
  (global (export "STAGE_BASE64URL_ENCODE") i32 (i32.const 109))
  (global (export "STAGE_BASE64URL_DECODE") i32 (i32.const 110))
  (global (export "STAGE_MAC_SCAN") i32 (i32.const 111))
  (global (export "STAGE_UTF8_SCAN_SIMD") i32 (i32.const 112))
  (global (export "STAGE_TOML_SCAN_SCALAR") i32 (i32.const 113))
  (global (export "STAGE_TOML_SCAN_KEY_VALUE") i32 (i32.const 114))
  (global (export "STAGE_YAML_SCAN_LINE") i32 (i32.const 115))
  (global (export "STAGE_OAUTH_JSON_VALUE") i32 (i32.const 116))
  (global (export "STAGE_OAUTH_PARSE_HTTP_RESPONSE") i32 (i32.const 117))
  (global (export "STAGE_SDK_SEED_SHAPE32") i32 (i32.const 118))
  (global (export "STAGE_CRC32_UNROLLED") i32 (i32.const 119))
  (global (export "STAGE_ADLER32_VEC") i32 (i32.const 120))
  (global (export "STAGE_BASE32HEX_ENCODE") i32 (i32.const 121))
  (global (export "STAGE_BASE32HEX_DECODE") i32 (i32.const 122))
  (global (export "STAGE_VARINT_DECODE") i32 (i32.const 123))
  (global (export "STAGE_VARINT_ENCODE") i32 (i32.const 124))
  (global (export "STAGE_TLS_RECORD_HEADER_ENCODE") i32 (i32.const 125))
  (global (export "STAGE_TLS_HANDSHAKE_HEADER_DECODE") i32 (i32.const 126))
  (global (export "STAGE_HTTP_FIND_CRLF") i32 (i32.const 127))
  (global (export "STAGE_JSON_UNESCAPE_STRING") i32 (i32.const 128))
  (global (export "STAGE_HTTP_FIND_DOUBLE_CRLF") i32 (i32.const 129))
  (global (export "STAGE_TLS_EXTENSION_NEXT") i32 (i32.const 130))
  (global (export "STAGE_QPACK_PREFIX_INT_DECODE") i32 (i32.const 131))
  (global (export "STAGE_QUIC_VARINT_DECODE") i32 (i32.const 132))
  (global (export "STAGE_QUIC_VARINT_ENCODE") i32 (i32.const 133))
  (global (export "STAGE_YAML_SCAN_DOCUMENT") i32 (i32.const 134))
  (global (export "STAGE_KV_COLON_SCAN") i32 (i32.const 135))
  (global (export "STAGE_UNQUOTE_SPAN") i32 (i32.const 136))
  (global (export "STAGE_BRACKET_LIST_NEXT") i32 (i32.const 137))
  (global (export "STAGE_HOST_PORT_SCAN") i32 (i32.const 138))
  (global (export "STAGE_X25519_SCALAR_MULT") i32 (i32.const 139))
  (global (export "STAGE_AES256_ENCRYPT") i32 (i32.const 140))
  (global (export "STAGE_UI_LAYOUT") i32 (i32.const 141))
  (global (export "STAGE_UI_PAINT") i32 (i32.const 142))
  (global (export "STAGE_UI_EVENT") i32 (i32.const 143))
  (global (export "STAGE_WASM_CALL") i32 (i32.const 144))
  (global (export "STAGE_EDGERUN_COMPILE") i32 (i32.const 145))
  ;; ── Stage function type ──
  ;; (input_pipe, output_pipe, config_ptr, config_len, scratch, scap, state_ptr) -> result
  (type $stage_fn (func (param i32 i32 i32 i32 i32 i32 i32) (result i32)))

  ;; ── Pipeline descriptor layout ──
  ;; +0:  magic      i32
  ;; +4:  version    i32
  ;; +8:  pipe_cap   i32  — intermediate pipe capacity
  ;; +12: stage_count i32
  ;; +16: tick       i32  — incremented per pipeline_run call
  ;; +20: stages[] — each:
  ;;   +0:  stage_type i32
  ;;   +4:  config     i32
  ;;   +8:  config_len i32
  ;;   +12: state_ptr  i32
  ;;   total: 16 bytes
  (func (export "PIPELINE_MAGIC")   (result i32) i32.const 0x50495045)
  (func (export "PIPELINE_VERSION") (result i32) i32.const 1)

   ;; PD_* and PS_* offset globals are defined in out/gen/config.wat (from package.json) — always available

  ;; pipeline_create(pipe_cap, stage_count) → desc_ptr | -1
  (func $pipeline_create (export "pipeline_create") (param $pcap i32) (param $count i32) (result i32)
    (local $desc i32) (local $sz i32)
    (local.set $sz (i32.add (global.get $PD_STAGES)
      (i32.mul (local.get $count) (global.get $PS_SIZE))))
    (local.set $desc (call $pipe_alloc (local.get $sz)))
    (if (i32.eq (local.get $desc) (i32.const -1))
      (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $desc) (i32.const 0x50495045))
    (i32.store offset=4 (local.get $desc) (i32.const 1))
    (i32.store offset=8 (local.get $desc) (local.get $pcap))
    (i32.store offset=12 (local.get $desc) (local.get $count))
    (i32.store offset=16 (local.get $desc) (i32.const 0))
    (i32.store offset=20 (local.get $desc) (i32.const 0))
    local.get $desc)

  ;; pipeline_set_stage(desc, index, stage_type, config, config_len)
  (func $pipeline_set_stage (export "pipeline_set_stage")
    (param $desc i32) (param $idx i32) (param $stype i32)
    (param $cfg i32) (param $clen i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store (i32.add (local.get $desc) (local.get $slot)) (local.get $stype))
    (i32.store offset=4 (i32.add (local.get $desc) (local.get $slot)) (local.get $cfg))
    (i32.store offset=8 (i32.add (local.get $desc) (local.get $slot)) (local.get $clen)))

  ;; pipeline_set_stage_state(desc, index, state_ptr)
  (func (export "pipeline_set_stage_state")
    (param $desc i32) (param $idx i32) (param $state i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.store offset=12 (i32.add (local.get $desc) (local.get $slot)) (local.get $state)))

  (func (export "pipeline_get_stage_type") (param $desc i32) (param $idx i32) (result i32)
    (local $slot i32)
    (local.set $slot
      (i32.add (global.get $PD_STAGES)
        (i32.mul (local.get $idx) (global.get $PS_SIZE))))
    (i32.load (i32.add (local.get $desc) (local.get $slot))))

  (func (export "pipeline_get_tick") (param $desc i32) (result i32)
    (i32.load offset=16 (local.get $desc)))

  (func (export "pipeline_set_frame_size") (param $desc i32) (param $frame i32)
    (i32.store offset=20 (local.get $desc) (local.get $frame)))

  (func (export "pipeline_get_frame_size") (param $desc i32) (result i32)
    (i32.load offset=20 (local.get $desc)))

  ;; pipeline_run(desc, input_pipe, output_pipe, scratch, scap) → OK | MORE | error
  ;; Fuses consecutive batch stages (state_ptr==0) by reusing a single intermediate pipe.
  ;; When frame_size > 0, intermediate pipes use aligned capacity for zero-copy SIMD.
  (func $pipeline_run (export "pipeline_run")
    (param $desc i32) (param $input i32) (param $output i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $count i32) (local $pcap i32) (local $frame i32)
    (local $i i32) (local $stype i32)
    (local $out i32) (local $prev i32) (local $result i32)
    (local $stages i32) (local $slot i32)
    (local $cfg i32) (local $clen i32) (local $state i32)
    (local $snapshot i32) (local $last_i i32)
    (local $reusable i32) (local $in_batch i32)

    (local.set $count (i32.load offset=12 (local.get $desc)))
    (local.set $last_i (i32.sub (local.get $count) (i32.const 1)))
    (local.set $pcap (i32.load offset=8 (local.get $desc)))
    (local.set $frame (i32.load offset=20 (local.get $desc)))
    (local.set $stages (i32.add (local.get $desc) (global.get $PD_STAGES)))
    (local.set $prev (local.get $input))

    ;; Increment tick for this run
    (i32.store offset=16 (local.get $desc)
      (i32.add (i32.load offset=16 (local.get $desc)) (i32.const 1)))

    ;; Save heap snapshot before allocating intermediate pipes
    (local.set $snapshot (call $pipe_snapshot))

    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $count)))
        (local.set $stype (i32.load (i32.add (local.get $stages) (local.get $slot))))
        (local.set $cfg (i32.load offset=4 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $clen (i32.load offset=8 (i32.add (local.get $stages) (local.get $slot))))
        (local.set $state (i32.load offset=12 (i32.add (local.get $stages) (local.get $slot))))

        ;; Write current tick to state[0] if stage has state
        (if (local.get $state)
          (then (i32.store (local.get $state) (i32.load offset=16 (local.get $desc)))))

        ;; Determine output pipe.
        ;; Optimisation: consecutive batch stages reuse a single intermediate pipe.
        (if (i32.eq (local.get $i) (local.get $last_i))
          (then
            (local.set $out (local.get $output))
            (local.set $in_batch (i32.const 0)))
          (else
            (if (i32.and (local.get $in_batch) (i32.eqz (local.get $state)))
              (then
                ;; Consecutive batch stage: reuse the same pipe as prev and out.
                ;; The stage reads from prev (draining it, auto-reset) then writes to out.
                (local.set $out (local.get $prev)))
              (else
                (if (local.get $frame)
                  (then
                    (local.set $out (call $pipe_create_aligned (local.get $pcap) (local.get $frame)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done))))
                  (else
                    (local.set $out (call $pipe_create (local.get $pcap)))
                    (if (i32.eq (local.get $out) (i32.const -1))
                      (then (local.set $result (i32.const -1)) (br $done)))))
                (if (i32.eqz (local.get $state))
                  (then
                    (local.set $reusable (local.get $out))
                    (local.set $in_batch (i32.const 1))))))))

        ;; Call stage via dispatch table
        (local.set $result
          (call_indirect (type $stage_fn)
            (local.get $prev) (local.get $out) (local.get $cfg) (local.get $clen)
            (local.get $scratch) (local.get $scap) (local.get $state)
            (local.get $stype)))

        ;; Break on error or yield
        (if (i32.or
              (i32.lt_s (local.get $result) (i32.const 0))
              (i32.eq (local.get $result) (global.get $STATUS_MORE)))
          (then (br $done)))

        ;; Stage completed — reset result to OK for pipeline return value
        (local.set $result (global.get $STATUS_OK))

        ;; Close previous intermediate if it was a distinct pipe (not fused/reused)
        (if (i32.and (local.get $i) (i32.ne (local.get $prev) (local.get $out)))
          (then (call $pipe_close (local.get $prev))))

        ;; End batch run if current stage has state (streaming)
        (if (local.get $state)
          (then (local.set $in_batch (i32.const 0))))

        (local.set $prev (local.get $out))
        (local.set $slot (i32.add (local.get $slot) (global.get $PS_SIZE)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))

    ;; On full success, restore heap to free intermediate pipes.
    ;; On MORE (yield), intermediate pipes must persist for next call.
    (if (i32.eq (local.get $result) (global.get $STATUS_OK))
      (then (call $pipe_restore (local.get $snapshot))))
    local.get $result)

  ;; ── Stage boilerplate helpers ──

  ;; stage_read_input: read all available data from input pipe into 0x3000.
  ;; Returns length read, or 0 if no data.
  (func $stage_read_input (export "stage_read_input")
    (param $input i32) (param $scratch i32) (param $scap i32) (result i32)
    (local $len_slot i32) (local $read i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (global.get $SCRATCH_BUF) (local.get $read)))
    local.get $read)

  ;; stage_write_result: unpack i64 result (high32=status, low32=out_len),
  ;; write to output pipe. Returns written length or negative error.
  (func $stage_write_result (export "stage_write_result")
    (param $output i32) (param $result i64) (param $buf i32) (result i32)
    (local $status i32) (local $out_len i32)
    (local.set $status (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $out_len (i32.wrap_i64 (local.get $result)))
    (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $out_len)))
    local.get $out_len)

  ;; stage_write_output: write known-length buffer to output pipe, return length.
  (func $stage_write_output (export "stage_write_output")
    (param $output i32) (param $buf i32) (param $len i32) (result i32)
    (drop (call $pipe_write (local.get $output) (local.get $buf) (local.get $len)))
    local.get $len)

  ;; ── Built-in passthrough stage (table index 0) ──
  (func $stage_passthrough
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $pipe_drain (local.get $input) (local.get $output) (local.get $scratch) (local.get $scap)))

  (elem (i32.const 0) $stage_passthrough)
