  (import "edgerun" "is_alpha" (func $ascii_alpha (param i32) (result i32)))
  (import "edgerun" "is_digit" (func $ascii_digit (param i32) (result i32)))

;; Compact compatibility surfaces captured from local HTTP, reqwest-shaped
  ;; client glue, JSON schema, unicode, JNI, and wasm-bindgen shared crates.
  ;;
  ;; Return conventions:
  ;; - booleans are 0/1
  ;; - HTTP status class: invalid=0 other=1 success=2 client=4 server=5
  ;; - JSON escape action: raw=0 simple=1 nl=2 cr=3 tab=4 bs=5 ff=6 unicode=7
  ;; - cookie store/send: skip=0 store/send=1

  (func $m50bool (param $x i32) (result i32)
    local.get $x
    i32.const 0
    i32.ne)

  (func $between (param $x i32) (param $lo i32) (param $hi i32) (result i32)
    local.get $x
    local.get $lo
    i32.ge_s
    local.get $x
    local.get $hi
    i32.le_s
    i32.and)



  (export "http_header_name_byte_valid" (func $http_header_name_byte_valid))
  (func $http_header_name_byte_valid (param $b i32) (result i32)
    local.get $b
    call $ascii_alpha
    local.get $b
    call $ascii_digit
    i32.or
    local.get $b
    i32.const 33
    i32.eq
    i32.or
    local.get $b
    i32.const 35
    i32.const 39
    call $between
    i32.or
    local.get $b
    i32.const 42
    i32.eq
    i32.or
    local.get $b
    i32.const 43
    i32.eq
    i32.or
    local.get $b
    i32.const 45
    i32.eq
    i32.or
    local.get $b
    i32.const 46
    i32.eq
    i32.or
    local.get $b
    i32.const 94
    i32.eq
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    local.get $b
    i32.const 96
    i32.eq
    i32.or
    local.get $b
    i32.const 124
    i32.eq
    i32.or
    local.get $b
    i32.const 126
    i32.eq
    i32.or)

  (export "http_header_value_byte_valid" (func $http_header_value_byte_valid))
  (func $http_header_value_byte_valid (param $b i32) (result i32)
    local.get $b
    i32.const 9
    i32.eq
    local.get $b
    i32.const 32
    i32.ge_s
    i32.or)

  (export "http_method_byte_valid" (func $http_method_byte_valid))
  (func $http_method_byte_valid (param $b i32) (result i32)
    local.get $b
    i32.const 33
    i32.const 126
    call $between)

  (export "http_status_class" (func $http_status_class))
  (func $http_status_class (param $code i32) (result i32)
    local.get $code
    i32.const 100
    i32.const 999
    call $between
    i32.eqz
    if (result i32)
      i32.const 0
    else
      local.get $code
      i32.const 200
      i32.const 299
      call $between
      if (result i32)
        i32.const 2
      else
        local.get $code
        i32.const 400
        i32.const 499
        call $between
        if (result i32)
          i32.const 4
        else
          local.get $code
          i32.const 500
          i32.const 599
          call $between
          if (result i32)
            i32.const 5
          else
            i32.const 1
          end
        end
      end
    end)

  (export "http_known_reason" (func $http_known_reason))
  (func $http_known_reason (param $code i32) (result i32)
    local.get $code
    i32.const 101
    i32.eq
    local.get $code
    i32.const 200
    i32.eq
    i32.or
    local.get $code
    i32.const 400
    i32.const 404
    call $between
    i32.or
    local.get $code
    i32.const 426
    i32.eq
    i32.or
    local.get $code
    i32.const 429
    i32.eq
    i32.or
    local.get $code
    i32.const 500
    i32.eq
    i32.or
    local.get $code
    i32.const 502
    i32.eq
    i32.or
    local.get $code
    i32.const 503
    i32.eq
    i32.or)

  (export "http_request_builder_valid" (func $http_request_builder_valid))
  (func $http_request_builder_valid
    (param $state_valid i32)
    (param $has_uri i32)
    (param $method_valid i32)
    (param $headers_valid i32)
    (result i32)
    local.get $state_valid
    call $m50bool
    local.get $has_uri
    call $m50bool
    i32.and
    local.get $method_valid
    call $m50bool
    i32.and
    local.get $headers_valid
    call $m50bool
    i32.and)

  (export "http_response_builder_valid" (func $http_response_builder_valid))
  (func $http_response_builder_valid
    (param $status_valid i32)
    (param $headers_valid i32)
    (result i32)
    local.get $status_valid
    call $m50bool
    local.get $headers_valid
    call $m50bool
    i32.and)

  (export "http_uri_path_mode" (func $http_uri_path_mode))
  (func $http_uri_path_mode
    (param $has_scheme i32)
    (param $has_path_or_query i32)
    (result i32)
    ;; 0=relative/full string, 1=synthesized "/", 2=slice from path/query.
    local.get $has_scheme
    call $m50bool
    i32.eqz
    if (result i32)
      i32.const 0
    else
      local.get $has_path_or_query
      call $m50bool
      if (result i32)
        i32.const 2
      else
        i32.const 1
      end
    end)

  (export "reqwest_cookie_store_action" (func $reqwest_cookie_store_action))
  (func $reqwest_cookie_store_action
    (param $has_host i32)
    (param $has_name i32)
    (param $secure_cookie i32)
    (param $secure_url i32)
    (result i32)
    local.get $has_host
    call $m50bool
    local.get $has_name
    call $m50bool
    i32.and
    local.get $secure_cookie
    call $m50bool
    local.get $secure_url
    call $m50bool
    i32.eqz
    i32.and
    i32.eqz
    i32.and)

  (export "reqwest_cookie_send" (func $reqwest_cookie_send))
  (func $reqwest_cookie_send
    (param $host_matches i32)
    (param $secure_cookie i32)
    (param $secure_url i32)
    (result i32)
    local.get $host_matches
    call $m50bool
    local.get $secure_cookie
    call $m50bool
    local.get $secure_url
    call $m50bool
    i32.eqz
    i32.and
    i32.eqz
    i32.and)

  (export "reqwest_proxy_scheme_valid" (func $reqwest_proxy_scheme_valid))
  (func $reqwest_proxy_scheme_valid (param $scheme_code i32) (result i32)
    ;; 1=http, 2=https. Both are accepted by Proxy::https constructor.
    local.get $scheme_code
    i32.const 1
    i32.eq
    local.get $scheme_code
    i32.const 2
    i32.eq
    i32.or)

  (export "reqwest_https_connect_proxy_valid" (func $reqwest_https_connect_proxy_valid))
  (func $reqwest_https_connect_proxy_valid
    (param $proxy_scheme_code i32)
    (param $proxy_has_host i32)
    (param $target_has_host i32)
    (param $connect_status i32)
    (result i32)
    local.get $proxy_scheme_code
    i32.const 1
    i32.eq
    local.get $proxy_has_host
    call $m50bool
    i32.and
    local.get $target_has_host
    call $m50bool
    i32.and
    local.get $connect_status
    i32.const 200
    i32.const 299
    call $between
    i32.and)

  (export "reqwest_error_for_status" (func $reqwest_error_for_status))
  (func $reqwest_error_for_status (param $code i32) (result i32)
    local.get $code
    i32.const 400
    i32.const 599
    call $between)

  (export "reqwest_http_message_complete" (func $reqwest_http_message_complete))
  (func $reqwest_http_message_complete
    (param $has_terminator i32)
    (param $buffered_body_len i32)
    (param $content_length i32)
    (result i32)
    local.get $has_terminator
    call $m50bool
    local.get $buffered_body_len
    local.get $content_length
    i32.ge_s
    i32.and)

  (export "reqwest_builder_version" (func $reqwest_builder_version))
  (func $reqwest_builder_version
    (param $http1_only i32)
    (param $http2_prior i32)
    (param $http3_prior i32)
    (result i32)
    ;; best=0 http1=1 http2=2 http3=3; later setters win in builder order.
    local.get $http3_prior
    call $m50bool
    if (result i32)
      i32.const 3
    else
      local.get $http2_prior
      call $m50bool
      if (result i32)
        i32.const 2
      else
        local.get $http1_only
        call $m50bool
        if (result i32)
          i32.const 1
        else
          i32.const 0
        end
      end
    end)

  (export "json_escape_action" (func $json_escape_action))
  (func $json_escape_action (param $ch i32) (result i32)
    local.get $ch
    i32.const 34
    i32.eq
    local.get $ch
    i32.const 92
    i32.eq
    i32.or
    if (result i32)
      i32.const 1
    else
      local.get $ch
      i32.const 10
      i32.eq
      if (result i32)
        i32.const 2
      else
        local.get $ch
        i32.const 13
        i32.eq
        if (result i32)
          i32.const 3
        else
          local.get $ch
          i32.const 9
          i32.eq
          if (result i32)
            i32.const 4
          else
            local.get $ch
            i32.const 8
            i32.eq
            if (result i32)
              i32.const 5
            else
              local.get $ch
              i32.const 12
              i32.eq
              if (result i32)
                i32.const 6
              else
                local.get $ch
                i32.const 32
                i32.lt_s
                if (result i32)
                  i32.const 7
                else
                  i32.const 0
                end
              end
            end
          end
        end
      end
    end)

  (export "schemars_instance_json_code" (func $schemars_instance_json_code))
  (func $schemars_instance_json_code (param $kind i32) (result i32)
    ;; null=1 boolean=2 object=3 array=4 number=5 string=6 integer=7
    local.get $kind
    i32.const 1
    i32.const 7
    call $between
    if (result i32)
      local.get $kind
    else
      i32.const 0
    end)

  (export "schemars_schema_field_count" (func $schemars_schema_field_count))
  (func $schemars_schema_field_count
    (param $has_ref i32)
    (param $has_title i32)
    (param $has_description i32)
    (param $has_type i32)
    (param $has_const i32)
    (param $enum_count i32)
    (param $subschema_count i32)
    (param $property_count i32)
    (param $required_count i32)
    (param $has_items i32)
    (param $has_additional i32)
    (result i32)
    local.get $has_ref
    call $m50bool
    local.get $has_title
    call $m50bool
    i32.add
    local.get $has_description
    call $m50bool
    i32.add
    local.get $has_type
    call $m50bool
    i32.add
    local.get $has_const
    call $m50bool
    i32.add
    local.get $enum_count
    i32.const 0
    i32.gt_s
    i32.add
    local.get $subschema_count
    i32.const 0
    i32.gt_s
    i32.add
    local.get $property_count
    i32.const 0
    i32.gt_s
    i32.add
    local.get $required_count
    i32.const 0
    i32.gt_s
    i32.add
    local.get $has_items
    call $m50bool
    i32.add
    local.get $has_additional
    call $m50bool
    i32.add)

  (export "schemars_container_kind" (func $schemars_container_kind))
  (func $schemars_container_kind (param $rust_kind i32) (result i32)
    ;; bool=1 string=2 integer=3 number=4 array=5 map=6 result-anyof=7 object=8
    local.get $rust_kind
    i32.const 1
    i32.const 8
    call $between
    if (result i32)
      local.get $rust_kind
    else
      i32.const 0
    end)

  (export "wbg_schema_version_code" (func $wbg_schema_version_code))
  (func $wbg_schema_version_code (result i32)
    i32.const 2121)

  (export "wbg_export_char_action" (func $wbg_export_char_action))
  (func $wbg_export_char_action (param $b i32) (result i32)
    ;; copy=0 underscore=1 drop=2.
    local.get $b
    call $ascii_alpha
    local.get $b
    call $ascii_digit
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or
    if (result i32)
      i32.const 0
    else
      local.get $b
      i32.const 91
      i32.eq
      local.get $b
      i32.const 93
      i32.eq
      i32.or
      if (result i32)
        i32.const 2
      else
        i32.const 1
      end
    end)

  (export "wbg_named_function_kind" (func $wbg_named_function_kind))
  (func $wbg_named_function_kind (param $kind i32) (result i32)
    ;; new=1 free=2 unwrap=3 upcast=4 field-get=5 field-set=6 dynamic-union=7
    local.get $kind
    i32.const 1
    i32.const 7
    call $between
    if (result i32)
      local.get $kind
    else
      i32.const 0
    end)

  (export "js_ident_start_ascii" (func $js_ident_start_ascii))
  (func $js_ident_start_ascii (param $b i32) (result i32)
    local.get $b
    call $ascii_alpha
    local.get $b
    i32.const 36
    i32.eq
    i32.or
    local.get $b
    i32.const 95
    i32.eq
    i32.or)

  (export "js_ident_continue_ascii" (func $js_ident_continue_ascii))
  (func $js_ident_continue_ascii (param $b i32) (result i32)
    local.get $b
    call $js_ident_start_ascii
    local.get $b
    call $ascii_digit
    i32.or)

  (export "js_keyword_kind" (func $js_keyword_kind))
  (func $js_keyword_kind
    (param $is_keyword i32)
    (param $is_value_like i32)
    (result i32)
    ;; not keyword=0 value-like=1 non-value keyword=2.
    local.get $is_keyword
    call $m50bool
    i32.eqz
    if (result i32)
      i32.const 0
    else
      local.get $is_value_like
      call $m50bool
      if (result i32)
        i32.const 1
      else
        i32.const 2
      end
    end)

  (export "js_ident_repair_action" (func $js_ident_repair_action))
  (func $js_ident_repair_action
    (param $valid_ident i32)
    (param $is_keyword i32)
    (result i32)
    ;; unchanged=0 replace-invalid=1 prefix-underscore=2.
    local.get $valid_ident
    call $m50bool
    i32.eqz
    if (result i32)
      i32.const 1
    else
      local.get $is_keyword
      call $m50bool
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "unicode_terminal_width" (func $unicode_terminal_width))
  (func $unicode_terminal_width (param $code i32) (result i32)
    local.get $code
    i32.const 0
    i32.const 31
    call $between
    local.get $code
    i32.const 127
    i32.const 159
    call $between
    i32.or
    local.get $code
    i32.const 768
    i32.const 879
    call $between
    i32.or
    if (result i32)
      i32.const 0
    else
      local.get $code
      i32.const 4352
      i32.const 11519
      call $between
      local.get $code
      i32.const 12288
      i32.const 40959
      call $between
      i32.or
      if (result i32)
        i32.const 2
      else
        i32.const 1
      end
    end)

  (export "unicode_ident_table_partition" (func $unicode_ident_table_partition))
  (func $unicode_ident_table_partition (param $code i32) (result i32)
    local.get $code
    i32.const 0
    i32.lt_s
    local.get $code
    i32.const 1114111
    i32.gt_s
    i32.or
    if (result i32)
      i32.const 0
    else
      local.get $code
      i32.const 2048
      i32.lt_s
      if (result i32)
        i32.const 1
      else
        local.get $code
        i32.const 65536
        i32.lt_s
        if (result i32)
          i32.const 2
        else
          i32.const 3
        end
      end
    end)

  (export "jni_primitive_size" (func $jni_primitive_size))
  (func $jni_primitive_size (param $kind i32) (param $pointer_size i32) (result i32)
    ;; boolean/byte=1 char/short=2 int/float=4 long/double=8 object=pointer.
    local.get $kind
    i32.const 1
    i32.eq
    local.get $kind
    i32.const 2
    i32.eq
    i32.or
    if (result i32)
      i32.const 1
    else
      local.get $kind
      i32.const 3
      i32.eq
      local.get $kind
      i32.const 4
      i32.eq
      i32.or
      if (result i32)
        i32.const 2
      else
        local.get $kind
        i32.const 5
        i32.eq
        local.get $kind
        i32.const 7
        i32.eq
        i32.or
        if (result i32)
          i32.const 4
        else
          local.get $kind
          i32.const 6
          i32.eq
          local.get $kind
          i32.const 8
          i32.eq
          i32.or
          if (result i32)
            i32.const 8
          else
            local.get $kind
            i32.const 9
            i32.eq
            if (result i32)
              local.get $pointer_size
            else
              i32.const 0
            end
          end
        end
      end
    end)

  (export "jni_status_class" (func $jni_status_class))
  (func $jni_status_class (param $code i32) (result i32)
    ;; JNI_OK=0; known negative errors map to 1..6, unknown to 99.
    local.get $code
    i32.const 0
    i32.eq
    if (result i32)
      i32.const 0
    else
      local.get $code
      i32.const -1
      i32.ge_s
      if (result i32)
        i32.const 99
      else
        local.get $code
        i32.const -6
        i32.ge_s
        if (result i32)
          i32.const 0
          local.get $code
          i32.sub
        else
          i32.const 99
        end
      end
    end)

  (export "jni_version_value" (func $jni_version_value))
  (func $jni_version_value (param $level i32) (result i32)
    local.get $level
    i32.const 11
    i32.eq
    if (result i32)
      i32.const 65537
    else
      local.get $level
      i32.const 12
      i32.eq
      if (result i32)
        i32.const 65538
      else
        local.get $level
        i32.const 14
        i32.eq
        if (result i32)
          i32.const 65540
        else
          local.get $level
          i32.const 16
          i32.eq
          if (result i32)
            i32.const 65542
          else
            local.get $level
            i32.const 18
            i32.eq
            if (result i32)
              i32.const 65544
            else
              local.get $level
              i32.const 9
              i32.eq
              if (result i32)
                i32.const 589824
              else
                local.get $level
                i32.const 10
                i32.eq
                if (result i32)
                  i32.const 655360
                else
                  local.get $level
                  i32.const 19
                  i32.ge_s
                  local.get $level
                  i32.const 24
                  i32.le_s
                  i32.and
                  if (result i32)
                    local.get $level
                    i32.const 16
                    i32.shl
                  else
                    i32.const 0
                  end
                end
              end
            end
          end
        end
      end
    end)

  (export "jni_ref_type_valid" (func $jni_ref_type_valid))
  (func $jni_ref_type_valid (param $ref_type i32) (result i32)
    local.get $ref_type
    i32.const 0
    i32.const 3
    call $between)

  (export "jni_array_release_action" (func $jni_array_release_action))
  (func $jni_array_release_action (param $mode i32) (result i32)
    ;; default=0 copy-back-free, commit=1 copy-back-keep, abort=2 discard-free.
    local.get $mode
    i32.const 1
    i32.eq
    if (result i32)
      i32.const 1
    else
      local.get $mode
      i32.const 2
      i32.eq
      if (result i32)
        i32.const 2
      else
        i32.const 0
      end
    end)

  (export "jni_version_gate" (func $jni_version_gate))
  (func $jni_version_gate (param $api_version i32) (param $requested i32) (result i32)
    local.get $api_version
    local.get $requested
    i32.ge_s)
