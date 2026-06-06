  (import "edgerun" "to_lower" (func $m109lower (param i32) (result i32)))
  (import "http" "is_tchar" (func $is_tchar (param i32) (result i32)))

(func $byte_lower_at (param $ptr i32) (param $off i32) (result i32)
    (call $m109lower (i32.load8_u (i32.add (local.get $ptr) (local.get $off)))))

  (func $is_ctl_or_space (param $b i32) (result i32)
    (i32.or (i32.le_u (local.get $b) (i32.const 32)) (i32.eq (local.get $b) (i32.const 127))))

  (func (export "http_header_name_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 1))))
      (if (i32.eqz (call $is_tchar (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $scan)
    i32.const 1)

  (func (export "http_header_value_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (loop $scan
      (if (i32.ge_u (local.get $i) (local.get $len)) (then (return (i32.const 1))))
      (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
      (if
        (i32.eqz
          (i32.or
            (i32.eq (local.get $b) (i32.const 9))
            (i32.and (i32.ge_u (local.get $b) (i32.const 32)) (i32.le_u (local.get $b) (i32.const 126)))))
        (then (return (i32.const 0))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      br $scan)
    i32.const 1)

  ;; Method classes: 0 invalid, 1 GET, 2 HEAD, 3 POST, 4 PUT, 5 DELETE,
  ;; 6 CONNECT, 7 OPTIONS, 8 TRACE, 9 PATCH, 255 extension token.
  (func (export "http_method_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $valid_done
      (loop $valid
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $valid_done)))
        (if (i32.eqz (call $is_tchar (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $valid))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 3))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 103))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116)))))
      (then (return (i32.const 1))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 4))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 104))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 97))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 100))))))
      (then (return (i32.const 2))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 4))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 111))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 115))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 116))))))
      (then (return (i32.const 3))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 3))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 117))
            (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116)))))
      (then (return (i32.const 4))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 6))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 100))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 101))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 108))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 101))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 116))
                  (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 101))))))))
      (then (return (i32.const 5))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 7))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 99))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 111))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 110))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 110))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 101))
                  (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 99))
                    (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 6)) (i32.const 116)))))))))
      (then (return (i32.const 6))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 7))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 111))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 112))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 105))
                (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 111))
                  (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 5)) (i32.const 110))
                    (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 6)) (i32.const 115)))))))))
      (then (return (i32.const 7))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 5))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 116))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 114))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 97))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 99))
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 101)))))))
      (then (return (i32.const 8))))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 5))
        (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 112))
          (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 97))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 99))
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 104)))))))
      (then (return (i32.const 9))))
    i32.const 255)

  ;; bit 0: method can carry a request body; bit 1: method expects response body.
  (func (export "http_method_body_flags") (param $method_class i32) (result i32)
    (local $flags i32)
    (local.set $flags (i32.const 2))
    (if (i32.eq (local.get $method_class) (i32.const 2)) (then (local.set $flags (i32.const 0))))
    (if
      (i32.or
        (i32.or (i32.eq (local.get $method_class) (i32.const 3)) (i32.eq (local.get $method_class) (i32.const 4)))
        (i32.or (i32.eq (local.get $method_class) (i32.const 6)) (i32.or (i32.eq (local.get $method_class) (i32.const 9)) (i32.eq (local.get $method_class) (i32.const 255)))))
      (then (local.set $flags (i32.or (local.get $flags) (i32.const 1)))))
    local.get $flags)

  ;; Status classes: 0 invalid, 1 informational, 2 success, 3 redirection,
  ;; 4 client error, 5 server error.
  (func (export "http_status_classify") (param $code i32) (result i32)
    (if (i32.lt_u (local.get $code) (i32.const 100)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $code) (i32.const 600)) (then (return (i32.const 0))))
    (i32.div_u (local.get $code) (i32.const 100)))

  ;; Request target classes: 0 invalid, 1 origin-form path, 2 absolute http(s),
  ;; 3 asterisk, 4 relative target accepted by node builder normalization.
  (func (export "http_request_target_classify") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $b i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (block $scan_done
      (loop $scan
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $scan_done)))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (call $is_ctl_or_space (local.get $b)) (then (return (i32.const 0))))
        (if (i32.eq (local.get $b) (i32.const 35)) (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        br $scan))
    (if
      (i32.and (i32.eq (local.get $len) (i32.const 1)) (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 42)))
      (then (return (i32.const 3))))
    (if (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)) (then (return (i32.const 1))))
    (if
      (i32.and
        (i32.ge_u (local.get $len) (i32.const 7))
        (i32.and
          (i32.and
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 0)) (i32.const 104))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 1)) (i32.const 116)))
            (i32.and (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 2)) (i32.const 116))
              (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 3)) (i32.const 112))))
          (i32.or
            (i32.and
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 58))
              (i32.and
                (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 47))
                (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 47))))
            (i32.and
              (i32.ge_u (local.get $len) (i32.const 8))
              (i32.and
                (i32.eq (call $byte_lower_at (local.get $ptr) (i32.const 4)) (i32.const 115))
                (i32.and
                  (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 58))
                  (i32.and
                    (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 47))
                    (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 7))) (i32.const 47)))))))))
      (then (return (i32.const 2))))
    i32.const 4)

  ;; HTTP/1 body states: 0 no body, 1 fixed empty, 2 fixed length,
  ;; 3 chunked, 4 close-delimited.
  (func (export "http1_body_state")
    (param $method_class i32) (param $status_code i32) (param $has_chunked i32)
    (param $content_length_present i32) (param $content_length_low32 i32)
    (result i32)
    (if (i32.eq (local.get $method_class) (i32.const 2)) (then (return (i32.const 0))))
    (if (i32.and (i32.ge_u (local.get $status_code) (i32.const 100)) (i32.lt_u (local.get $status_code) (i32.const 200)))
      (then (return (i32.const 0))))
    (if (i32.or (i32.eq (local.get $status_code) (i32.const 204)) (i32.eq (local.get $status_code) (i32.const 304)))
      (then (return (i32.const 0))))
    (if (local.get $has_chunked) (then (return (i32.const 3))))
    (if (local.get $content_length_present)
      (then
        (if (i32.eqz (local.get $content_length_low32))
          (then (return (i32.const 1)))
          (else (return (i32.const 2))))))
    i32.const 4)

  (func (export "http2_frame_type_classify") (param $frame_type i32) (result i32)
    (if (result i32) (i32.le_u (local.get $frame_type) (i32.const 9))
      (then (local.get $frame_type))
      (else (i32.const 255))))

  (func (export "http2_error_classify") (param $code i32) (result i32)
    (if
      (i32.or
        (i32.or (i32.le_u (local.get $code) (i32.const 9)) (i32.eq (local.get $code) (i32.const 11)))
        (i32.or (i32.eq (local.get $code) (i32.const 12)) (i32.eq (local.get $code) (i32.const 13))))
      (then (return (local.get $code))))
    i32.const 2)

  ;; Returns HTTP/2 error code: 0 ok, 1 PROTOCOL_ERROR, 6 FRAME_SIZE_ERROR.
  (func (export "http2_frame_semantics")
    (param $frame_class i32) (param $flags i32) (param $stream_id i32)
    (param $payload_len i32) (param $first_u32 i32)
    (result i32)
    (if (i32.eq (local.get $frame_class) (i32.const 255)) (then (return (i32.const 0))))
    (if
      (i32.or
        (i32.or (i32.eq (local.get $frame_class) (i32.const 0)) (i32.eq (local.get $frame_class) (i32.const 1)))
        (i32.or (i32.eq (local.get $frame_class) (i32.const 2)) (i32.or (i32.eq (local.get $frame_class) (i32.const 3)) (i32.eq (local.get $frame_class) (i32.const 9)))))
      (then (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 1))))))
    (if (i32.eq (local.get $frame_class) (i32.const 2))
      (then (if (i32.ne (local.get $payload_len) (i32.const 5)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 3))
      (then (if (i32.ne (local.get $payload_len) (i32.const 4)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 4))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.and (i32.and (local.get $flags) (i32.const 1)) (i32.ne (local.get $payload_len) (i32.const 0)))
          (then (return (i32.const 6))))
        (if (i32.and (i32.eqz (i32.and (local.get $flags) (i32.const 1))) (i32.ne (i32.rem_u (local.get $payload_len) (i32.const 6)) (i32.const 0)))
          (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 5))
      (then
        (if (i32.eqz (local.get $stream_id)) (then (return (i32.const 1))))
        (if (i32.ge_u (local.get $payload_len) (i32.const 4))
          (then
            (if
              (i32.or
                (i32.eqz (i32.and (local.get $first_u32) (i32.const 0x7fffffff)))
                (i32.eqz (i32.and (local.get $first_u32) (i32.const 1))))
              (then (return (i32.const 1))))))))
    (if (i32.eq (local.get $frame_class) (i32.const 6))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.ne (local.get $payload_len) (i32.const 8)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 7))
      (then
        (if (i32.ne (local.get $stream_id) (i32.const 0)) (then (return (i32.const 1))))
        (if (i32.lt_u (local.get $payload_len) (i32.const 8)) (then (return (i32.const 6))))))
    (if (i32.eq (local.get $frame_class) (i32.const 8))
      (then
        (if (i32.ne (local.get $payload_len) (i32.const 4)) (then (return (i32.const 6))))
        (if (i32.eqz (i32.and (local.get $first_u32) (i32.const 0x7fffffff))) (then (return (i32.const 1))))))
    i32.const 0)

  ;; HTTP/3 frame classes: 0 DATA, 1 HEADERS, 2 SETTINGS, 3 CANCEL_PUSH,
  ;; 4 PUSH_PROMISE, 5 MAX_PUSH_ID, 6 GOAWAY, 7 STREAMS_BLOCKED,
  ;; 8 reserved/grease, 9 unknown extension.
  (func (export "http3_frame_type_classify") (param $value i64) (result i32)
    (if (i64.eq (local.get $value) (i64.const 0)) (then (return (i32.const 0))))
    (if (i64.eq (local.get $value) (i64.const 1)) (then (return (i32.const 1))))
    (if (i64.eq (local.get $value) (i64.const 3)) (then (return (i32.const 3))))
    (if (i64.eq (local.get $value) (i64.const 4)) (then (return (i32.const 2))))
    (if (i64.eq (local.get $value) (i64.const 5)) (then (return (i32.const 4))))
    (if (i64.eq (local.get $value) (i64.const 7)) (then (return (i32.const 5))))
    (if (i64.eq (local.get $value) (i64.const 8)) (then (return (i32.const 6))))
    (if (i64.eq (local.get $value) (i64.const 9)) (then (return (i32.const 7))))
    (if
      (i32.or
        (i64.eq (i64.rem_u (local.get $value) (i64.const 31)) (i64.const 2))
        (i64.eq (i64.rem_u (local.get $value) (i64.const 31)) (i64.const 6)))
      (then (return (i32.const 8))))
    i32.const 9)

  ;; Returns 0 ok or 10 H3_FRAME_UNEXPECTED.
  ;; stream_kind: 0 control, 1 request/response, 2 push, 3 qpack.
  (func (export "http3_frame_stream_semantics") (param $stream_kind i32) (param $frame_class i32) (result i32)
    (if (i32.eq (local.get $stream_kind) (i32.const 0))
      (then
        (if
          (i32.or
            (i32.or (i32.eq (local.get $frame_class) (i32.const 2)) (i32.eq (local.get $frame_class) (i32.const 6)))
            (i32.or (i32.eq (local.get $frame_class) (i32.const 5)) (i32.eq (local.get $frame_class) (i32.const 3))))
          (then (return (i32.const 0)))
          (else (return (i32.const 10))))))
    (if
      (i32.or (i32.eq (local.get $stream_kind) (i32.const 1)) (i32.eq (local.get $stream_kind) (i32.const 2)))
      (then
        (if (i32.or (i32.eq (local.get $frame_class) (i32.const 0)) (i32.eq (local.get $frame_class) (i32.const 1)))
          (then (return (i32.const 0)))
          (else (return (i32.const 10))))))
    i32.const 10)

  ;; Route state from node HTTP/3 path/goaway fields:
  ;; 0 no active path, 1 active path and stream allowed, 2 active but going away
  ;; still allows this stream, 3 active but GOAWAY rejects this stream.
  (func (export "http3_route_state")
    (param $active_path_present i32) (param $going_away_present i32)
    (param $stream_id i64) (param $goaway_stream_id i64)
    (result i32)
    (if (i32.eqz (local.get $active_path_present)) (then (return (i32.const 0))))
    (if (i32.eqz (local.get $going_away_present)) (then (return (i32.const 1))))
    (if (i64.le_u (local.get $stream_id) (local.get $goaway_stream_id))
      (then (return (i32.const 2))))
    i32.const 3)

  (func (export "http3_status_classify") (param $code i32) (result i32)
    (call $http_status_classify_public (local.get $code)))

  (func $http_status_classify_public (param $code i32) (result i32)
    (if (i32.lt_u (local.get $code) (i32.const 100)) (then (return (i32.const 0))))
    (if (i32.ge_u (local.get $code) (i32.const 600)) (then (return (i32.const 0))))
    (i32.div_u (local.get $code) (i32.const 100)))
