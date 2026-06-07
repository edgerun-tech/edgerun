;; Pipeline Core — pipeline_run + dispatch table

    ;; Standard ID removed — merged into single module

  ;; ── Stage dispatch table ──
  (table (export "stage_table") 144 funcref)

  ;; ── Stage type constants (dispatch table indices) ──
  (func (export "STAGE_PASSTHROUGH") (result i32) i32.const 0)
  (func (export "STAGE_HEX_ENCODE")  (result i32) i32.const 1)
  (func (export "STAGE_HEX_DECODE")  (result i32) i32.const 2)
  (func (export "STAGE_B64_ENCODE")  (result i32) i32.const 3)
  (func (export "STAGE_B64_DECODE")  (result i32) i32.const 4)
  (func (export "STAGE_TRANSPORT")   (result i32) i32.const 5)
  (func (export "STAGE_MUX_STATIC")  (result i32) i32.const 6)
  (func (export "STAGE_DEMUX_STATIC") (result i32) i32.const 7)
  (func (export "STAGE_MUX_DYNAMIC") (result i32) i32.const 8)
  (func (export "STAGE_DEMUX_DYNAMIC") (result i32) i32.const 9)
  (func (export "STAGE_WS_FRAME")     (result i32) i32.const 10)
  (func (export "STAGE_WS_ENCODE")    (result i32) i32.const 11)
  (func (export "STAGE_WS_DECODE")    (result i32) i32.const 12)
  (func (export "STAGE_EXEC")         (result i32) i32.const 13)
  (func (export "STAGE_FRAME_PACER")  (result i32) i32.const 15)

  ;; ── Edgerun pipeline stages (compiler/interpreter) ──
  (func (export "STAGE_EDGERUN_PARSE") (result i32) i32.const 46)
  (func (export "STAGE_EDGERUN_EXEC")  (result i32) i32.const 47)

  ;; ── Queue/Buffer/CDC stages ──
  (func (export "STAGE_QUEUE")  (result i32) i32.const 48)
  (func (export "STAGE_BUFFER") (result i32) i32.const 49)
  (func (export "STAGE_CDC")    (result i32) i32.const 50)
  (func (export "STAGE_SHA256")       (result i32) i32.const 16)
  (func (export "STAGE_HMAC_SHA256")  (result i32) i32.const 17)
  (func (export "STAGE_SHA1")         (result i32) i32.const 18)
  (func (export "STAGE_SHA512")       (result i32) i32.const 19)
  (func (export "STAGE_SHA384")       (result i32) i32.const 20)
  (func (export "STAGE_AES128_GCM_ENCRYPT") (result i32) i32.const 21)
  (func (export "STAGE_AES128_GCM_DECRYPT") (result i32) i32.const 22)
  (func (export "STAGE_AES128_CTR_XOR")    (result i32) i32.const 23)
  (func (export "STAGE_DJB2_HASH")         (result i32) i32.const 24)
  (func (export "STAGE_INET_CHECKSUM")     (result i32) i32.const 25)
  (func (export "STAGE_CRC32_BZIP")        (result i32) i32.const 26)
  (func (export "STAGE_DEFLATE_DECODE")    (result i32) i32.const 27)
  (func (export "STAGE_DEFLATE_ENCODE")    (result i32) i32.const 28)
  (func (export "STAGE_GZIP_DECODE")       (result i32) i32.const 29)
  (func (export "STAGE_GZIP_ENCODE")       (result i32) i32.const 30)
  (func (export "STAGE_ZLIB_DECODE")       (result i32) i32.const 31)
  (func (export "STAGE_ZLIB_ENCODE")       (result i32) i32.const 32)
  (func (export "STAGE_JSON_PARSE")        (result i32) i32.const 33)
  (func (export "STAGE_PERCENT_DECODE")    (result i32) i32.const 34)
  (func (export "STAGE_PERCENT_ENCODE")    (result i32) i32.const 35)
  (func (export "STAGE_UTF8_REPAIR")       (result i32) i32.const 36)
  (func (export "STAGE_TO_LOWER")          (result i32) i32.const 37)
  (func (export "STAGE_TO_UPPER")          (result i32) i32.const 38)
  (func (export "STAGE_HEX_DECODE_COMPAT") (result i32) i32.const 39)
  (func (export "STAGE_CP1252_DECODE")     (result i32) i32.const 40)
  (func (export "STAGE_PEM_COMPACT")       (result i32) i32.const 41)
  (func (export "STAGE_AES128_ENCRYPT")    (result i32) i32.const 42)
  (func (export "STAGE_AES128_DECRYPT")    (result i32) i32.const 43)
  (func (export "STAGE_HTTP_REQUEST_LINE_PARSE") (result i32) i32.const 44)
  (func (export "STAGE_HTTP_STATUS_LINE_PARSE")  (result i32) i32.const 45)
  (func (export "STAGE_UUID_FORMAT")       (result i32) i32.const 51)
  (func (export "STAGE_UUID_PARSE")        (result i32) i32.const 52)
  (func (export "STAGE_TO_TITLE_CASE")     (result i32) i32.const 53)
  (func (export "STAGE_CESU8_TO_UTF8")     (result i32) i32.const 54)
  (func (export "STAGE_ESCAPE_TEXT")       (result i32) i32.const 55)
  (func (export "STAGE_UNESCAPE_TEXT")     (result i32) i32.const 56)
  (func (export "STAGE_HTTP_DATE_PARSE")   (result i32) i32.const 57)
  (func (export "STAGE_HTTP_DATE_FORMAT")  (result i32) i32.const 58)
  (func (export "STAGE_DNS_HEADER_DECODE") (result i32) i32.const 59)
  (func (export "STAGE_DNS_HEADER_ENCODE") (result i32) i32.const 60)
  (func (export "STAGE_TLS_RECORD_DECODE") (result i32) i32.const 61)
  (func (export "STAGE_HPACK_HUFFMAN_DECODE") (result i32) i32.const 62)
  (func (export "STAGE_HTTP_NEXT_HEADER")     (result i32) i32.const 63)
  (func (export "STAGE_HTTP_CHUNK_SCAN")      (result i32) i32.const 64)
  (func (export "STAGE_DNS_NAME_DECOMPRESS")  (result i32) i32.const 65)
  (func (export "STAGE_TLS_CLIENTHELLO_SCAN") (result i32) i32.const 66)
  (func (export "STAGE_GENERIC_TLV_DECODE")   (result i32) i32.const 67)
  (func (export "STAGE_GENERIC_TLV_ENCODE")   (result i32) i32.const 68)
  (func (export "STAGE_FRAME_HEADER_DECODE")  (result i32) i32.const 69)
  (func (export "STAGE_FRAME_HEADER_ENCODE")  (result i32) i32.const 70)
  (func (export "STAGE_DNS_QUESTION_NEXT")    (result i32) i32.const 71)
  (func (export "STAGE_DNS_RR_NEXT")          (result i32) i32.const 72)
  (func (export "STAGE_ENDIAN_READ")          (result i32) i32.const 73)
  (func (export "STAGE_ENDIAN_WRITE")         (result i32) i32.const 74)
  (func (export "STAGE_RSA_PKCS1_VERIFY")     (result i32) i32.const 75)
  (func (export "STAGE_RSA_PKCS1_EMIT")       (result i32) i32.const 76)
  (func (export "STAGE_OAUTH_URL_ENCODE")     (result i32) i32.const 77)
  (func (export "STAGE_HTTP_CLASSIFY_BODY")   (result i32) i32.const 78)
  (func (export "STAGE_URL_SCAN")             (result i32) i32.const 79)
  (func (export "STAGE_X509_CERT_SCAN")       (result i32) i32.const 80)
  (func (export "STAGE_HTTP2_FRAME_HEADER_DECODE")  (result i32) i32.const 81)
  (func (export "STAGE_HTTP3_FRAME_HEADER_DECODE")  (result i32) i32.const 82)
  (func (export "STAGE_TLS_CERT_LIST_SCAN")   (result i32) i32.const 83)
  (func (export "STAGE_TLS_SNI_HOST")         (result i32) i32.const 84)
  (func (export "STAGE_RFC3339_PARSE")        (result i32) i32.const 85)
  (func (export "STAGE_HTTP_LOWERCASE_HEADER") (result i32) i32.const 86)
  (func (export "STAGE_TLS_ALPN_NEXT")        (result i32) i32.const 87)
  (func (export "STAGE_DER_SEQUENCE_DECODE")  (result i32) i32.const 88)
  (func (export "STAGE_HTTP_VALIDATE_HEADERS") (result i32) i32.const 89)
  (func (export "STAGE_HTTP_PARSE_CONTENT_LENGTH") (result i32) i32.const 90)
  (func (export "STAGE_DER_INTEGER_DECODE")   (result i32) i32.const 91)
  (func (export "STAGE_DER_OCTET_STRING_DECODE") (result i32) i32.const 92)
  (func (export "STAGE_DER_TIME_DECODE")      (result i32) i32.const 93)
  (func (export "STAGE_TLS_CERT_ENTRY_NEXT")  (result i32) i32.const 94)
  (func (export "STAGE_OCI_REFERENCE_SCAN")   (result i32) i32.const 95)
  (func (export "STAGE_SSH_AUTH_KEY_SCAN")    (result i32) i32.const 96)
  (func (export "STAGE_DER_BIT_STRING_DECODE") (result i32) i32.const 97)
  (func (export "STAGE_UTF8_SCAN")            (result i32) i32.const 98)
  (func (export "STAGE_HPACK_HEADER_BLOCK_SCAN") (result i32) i32.const 99)
  (func (export "STAGE_WS_PARSE_HEADER")         (result i32) i32.const 100)
  (func (export "STAGE_WS_WRITE_FRAME_HEADER")   (result i32) i32.const 101)
  (func (export "STAGE_TLS_DNS_NAME_NORMALIZE")  (result i32) i32.const 102)
  (func (export "STAGE_DER_OID_ROOT_DECODE")     (result i32) i32.const 103)
  (func (export "STAGE_DER_OID_NEXT_ARC")        (result i32) i32.const 104)
  (func (export "STAGE_JSON_EMIT_STRING")        (result i32) i32.const 105)
  (func (export "STAGE_PEM_FIND_BOUNDARIES")     (result i32) i32.const 106)
  (func (export "STAGE_URI_SCAN_PATH_QUERY")     (result i32) i32.const 107)
  (func (export "STAGE_FORM_URLENCODED_NEXT_PAIR") (result i32) i32.const 108)
  (func (export "STAGE_BASE64URL_ENCODE")        (result i32) i32.const 109)
  (func (export "STAGE_BASE64URL_DECODE")        (result i32) i32.const 110)
  (func (export "STAGE_MAC_SCAN")                (result i32) i32.const 111)
  (func (export "STAGE_UTF8_SCAN_SIMD")          (result i32) i32.const 112)
  (func (export "STAGE_TOML_SCAN_SCALAR")        (result i32) i32.const 113)
  (func (export "STAGE_TOML_SCAN_KEY_VALUE")     (result i32) i32.const 114)
  (func (export "STAGE_YAML_SCAN_LINE")          (result i32) i32.const 115)
  (func (export "STAGE_OAUTH_JSON_VALUE")        (result i32) i32.const 116)
  (func (export "STAGE_OAUTH_PARSE_HTTP_RESPONSE") (result i32) i32.const 117)
  (func (export "STAGE_SDK_SEED_SHAPE32")        (result i32) i32.const 118)
  (func (export "STAGE_CRC32_UNROLLED")          (result i32) i32.const 119)
  (func (export "STAGE_ADLER32_VEC")             (result i32) i32.const 120)
  (func (export "STAGE_BASE32HEX_ENCODE")        (result i32) i32.const 121)
  (func (export "STAGE_BASE32HEX_DECODE")        (result i32) i32.const 122)
  (func (export "STAGE_VARINT_DECODE")           (result i32) i32.const 123)
  (func (export "STAGE_VARINT_ENCODE")           (result i32) i32.const 124)
  (func (export "STAGE_TLS_RECORD_HEADER_ENCODE") (result i32) i32.const 125)
  (func (export "STAGE_TLS_HANDSHAKE_HEADER_DECODE") (result i32) i32.const 126)
  (func (export "STAGE_HTTP_FIND_CRLF")          (result i32) i32.const 127)
  (func (export "STAGE_JSON_UNESCAPE_STRING")    (result i32) i32.const 128)
  (func (export "STAGE_HTTP_FIND_DOUBLE_CRLF")   (result i32) i32.const 129)
  (func (export "STAGE_TLS_EXTENSION_NEXT")      (result i32) i32.const 130)
  (func (export "STAGE_QPACK_PREFIX_INT_DECODE") (result i32) i32.const 131)
  (func (export "STAGE_QUIC_VARINT_DECODE")      (result i32) i32.const 132)
  (func (export "STAGE_QUIC_VARINT_ENCODE")      (result i32) i32.const 133)
  (func (export "STAGE_YAML_SCAN_DOCUMENT")     (result i32) i32.const 134)
  (func (export "STAGE_KV_COLON_SCAN")          (result i32) i32.const 135)
  (func (export "STAGE_UNQUOTE_SPAN")           (result i32) i32.const 136)
  (func (export "STAGE_BRACKET_LIST_NEXT")      (result i32) i32.const 137)
  (func (export "STAGE_HOST_PORT_SCAN")         (result i32) i32.const 138)
  (func (export "STAGE_X25519_SCALAR_MULT")     (result i32) i32.const 139)
  (func (export "STAGE_AES256_ENCRYPT")         (result i32) i32.const 140)
  (func (export "STAGE_DASHBOARD")         (result i32) i32.const 14)

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

   ;; PD_* and PS_* offset globals are defined in runtime/memory-map.wat — included before this fragment.

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

  ;; ── Built-in passthrough stage (table index 0) ──
  (func $stage_passthrough
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $pipe_drain (local.get $input) (local.get $output) (local.get $scratch) (local.get $scap)))

  (elem (i32.const 0) $stage_passthrough)
