(module
  (import "sock" "open" (func $sock_open (param $type i32) (param $cfg i32) (param $cfg_len i32) (result i32)))
  (import "sock" "close" (func $sock_close (param $fd i32) (result i32)))
  (import "sock" "send" (func $sock_send (param $fd i32) (param $buf i32) (param $len i32) (result i32)))
  (import "sock" "recv" (func $sock_recv (param $fd i32) (param $buf i32) (param $cap i32) (result i32)))
  (import "sock" "poll" (func $sock_poll (param $fd i32) (param $timeout i32) (result i32)))

  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300504)

  (data (i32.const 256) "0123456789ABCDEF")

  ;; ================================================================
  ;; oauth_extract_code(http_ptr, http_len, out_ptr, out_cap) -> i64
  ;; Scan HTTP GET request for '?code=' or '&code=' parameter.
  ;; Returns packed (status, code_len), copies code value to out.
  ;; status: 0=found, -1=not found, -2=output too small
  ;; ================================================================
  (func (export "oauth_extract_code")
    (param $http i32) (param $hlen i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $i i32) (local $clen i32) (local $b i32)

    block $done
    loop $scan
      local.get $i local.get $hlen i32.ge_u
      if i64.const -1 return end

      local.get $http local.get $i i32.add i32.load8_u local.set $b
      local.get $b i32.const 99 i32.ne  ;; 'c'
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $i i32.const 4 i32.add local.get $hlen i32.ge_u
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $http local.get $i i32.add i32.load8_u offset=1
      i32.const 111 i32.ne  ;; 'o'
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $http local.get $i i32.add i32.load8_u offset=2
      i32.const 100 i32.ne  ;; 'd'
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $http local.get $i i32.add i32.load8_u offset=3
      i32.const 101 i32.ne  ;; 'e'
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $http local.get $i i32.add i32.load8_u offset=4
      i32.const 61 i32.ne  ;; '='
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      ;; check prefix is '?' or '&' (or start of string)
      local.get $i i32.const 0 i32.gt_u
      if
        local.get $http local.get $i i32.sub i32.const 1 i32.add i32.load8_u
        local.set $b
        local.get $b i32.const 63 i32.ne  ;; '?'
        if
          local.get $b i32.const 38 i32.ne  ;; '&'
          if local.get $i i32.const 1 i32.add local.set $i br $scan end
        end
      end

      ;; found code= — extract value
      local.get $i i32.const 5 i32.add local.set $i
      loop $val
        local.get $i local.get $hlen i32.ge_u br_if $done
        local.get $http local.get $i i32.add i32.load8_u local.set $b
        local.get $i i32.const 1 i32.add local.set $i
        local.get $b i32.const 38 i32.eq br_if $done  ;; '&'
        local.get $b i32.const 32 i32.eq br_if $done  ;; ' '
        local.get $b i32.const 13 i32.eq br_if $done  ;; CR
        local.get $b i32.const 10 i32.eq br_if $done  ;; LF
        local.get $clen local.get $ocap i32.ge_u
        if i64.const -2 return end
        local.get $out local.get $clen i32.add local.get $b i32.store8
        local.get $clen i32.const 1 i32.add local.set $clen
        br $val
      end
    end
    end

    i64.const 0
    local.get $clen
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; oauth_json_value(json_ptr, json_len, key_ptr, key_len, out_ptr, out_cap) -> i64
  ;; Find JSON string value by key name. Scans for "key":"value".
  ;; Returns packed (status, value_len)
  ;; ================================================================
  (func $oauth_json_value (export "oauth_json_value")
    (param $json i32) (param $jlen i32)
    (param $key i32) (param $klen i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $i i32) (local $vlen i32) (local $b i32) (local $k i32) (local $mismatch i32)

    block $done
    loop $scan
      local.get $i local.get $jlen i32.ge_u
      if i64.const -1 return end

      local.get $json local.get $i i32.add i32.load8_u
      i32.const 34 i32.ne  ;; '"'
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      ;; compare key
      local.get $i i32.const 1 i32.add local.get $klen i32.add
      local.get $jlen i32.gt_u
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      i32.const 0 local.set $k
      i32.const 0 local.set $mismatch
      block $key_check
      loop $key_cmp
        local.get $k local.get $klen i32.ge_u br_if $key_check
        local.get $json local.get $i i32.const 1 i32.add local.get $k i32.add i32.add i32.load8_u
        local.get $key local.get $k i32.add i32.load8_u
        i32.ne
        if
          i32.const 1 local.set $mismatch
          br $key_check
        end
        local.get $k i32.const 1 i32.add local.set $k
        br $key_cmp
      end
      end

      local.get $mismatch i32.const 0 i32.ne
      if
        ;; skip to end of this key name
        loop $skip_key
          local.get $i local.get $jlen i32.ge_u br_if $done
          local.get $json local.get $i i32.add i32.load8_u
          i32.const 34 i32.eq  ;; '"'
          if
            local.get $i i32.const 1 i32.add local.set $i
            br $scan
          end
          local.get $i i32.const 1 i32.add local.set $i
          br $skip_key
        end
      end

      ;; key matched — skip past colon to value
      local.get $i i32.const 1 local.get $klen i32.add i32.add local.set $i

      loop $skip_colon
        local.get $i local.get $jlen i32.ge_u br_if $done
        local.get $json local.get $i i32.add i32.load8_u local.set $b
        local.get $b i32.const 32 i32.eq  ;; ' '
        if local.get $i i32.const 1 i32.add local.set $i br $skip_colon end
        local.get $b i32.const 58 i32.eq  ;; ':'
        if local.get $i i32.const 1 i32.add local.set $i br $skip_colon end
        local.get $b i32.const 9 i32.eq  ;; tab
        if local.get $i i32.const 1 i32.add local.set $i br $skip_colon end
        br $done  ;; unexpected char
      end

      ;; skip optional whitespace after colon
      loop $skip_ws
        local.get $i local.get $jlen i32.ge_u br_if $done
        local.get $json local.get $i i32.add i32.load8_u i32.const 32 i32.eq
        if local.get $i i32.const 1 i32.add local.set $i br $skip_ws end
      end

      ;; expect opening quote
      local.get $i local.get $jlen i32.ge_u
      if i64.const -1 return end
      local.get $json local.get $i i32.add i32.load8_u i32.const 34 i32.ne  ;; '"'
      if i64.const -1 return end
      local.get $i i32.const 1 i32.add local.set $i

      ;; extract string value
      loop $val
        local.get $i local.get $jlen i32.ge_u br_if $done
        local.get $json local.get $i i32.add i32.load8_u local.set $b
        local.get $i i32.const 1 i32.add local.set $i
        local.get $b i32.const 34 i32.eq br_if $done  ;; closing quote
        ;; handle escaped chars
        local.get $b i32.const 92 i32.eq  ;; '\'
        if
          local.get $i local.get $jlen i32.ge_u br_if $done
          local.get $json local.get $i i32.add i32.load8_u local.set $b
          local.get $i i32.const 1 i32.add local.set $i
        end
        local.get $vlen local.get $ocap i32.ge_u
        if i64.const -2 return end
        local.get $out local.get $vlen i32.add local.get $b i32.store8
        local.get $vlen i32.const 1 i32.add local.set $vlen
        br $val
      end
    end
    end

    i64.const 0
    local.get $vlen
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; oauth_url_encode(src_ptr, src_len, dst_ptr, dst_cap) -> i64
  ;; Percent-encode URL-unsafe characters (RFC 3986).
  ;; Returns packed (status, encoded_len)
  ;; ================================================================
  (func (export "oauth_url_encode")
    (param $src i32) (param $slen i32)
    (param $dst i32) (param $dcap i32)
    (result i64)
    (local $i i32) (local $o i32) (local $b i32) (local $hi i32) (local $lo i32)

    block $done
    loop $loop
      local.get $i local.get $slen i32.ge_u br_if $done
      local.get $src local.get $i i32.add i32.load8_u local.set $b
      local.get $i i32.const 1 i32.add local.set $i

      ;; unreserved: A-Z a-z 0-9 - . _ ~
      block $unreserved
      block $encode
        local.get $b i32.const 65 i32.ge_u
        if
          local.get $b i32.const 90 i32.le_u
          br_if $unreserved
        end
        local.get $b i32.const 97 i32.ge_u
        if
          local.get $b i32.const 122 i32.le_u
          br_if $unreserved
        end
        local.get $b i32.const 48 i32.ge_u
        if
          local.get $b i32.const 57 i32.le_u
          br_if $unreserved
        end
        local.get $b i32.const 45 i32.eq br_if $unreserved  ;; '-'
        local.get $b i32.const 46 i32.eq br_if $unreserved  ;; '.'
        local.get $b i32.const 95 i32.eq br_if $unreserved  ;; '_'
        local.get $b i32.const 126 i32.eq br_if $unreserved  ;; '~'
        br $encode
      end
      ;; percent-encode: %HH
      local.get $o i32.const 2 i32.add local.get $dcap i32.ge_u
      if i64.const -2 return end
      local.get $b i32.const 4 i32.shr_u i32.const 15 i32.and local.set $hi
      local.get $b i32.const 15 i32.and local.set $lo
      local.get $dst local.get $o i32.add i32.const 37 i32.store8  ;; '%'
      local.get $o i32.const 1 i32.add local.set $o
      local.get $dst local.get $o i32.add
      i32.const 256 local.get $hi i32.add i32.load8_u
      i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      local.get $dst local.get $o i32.add
      i32.const 256 local.get $lo i32.add i32.load8_u
      i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      br $loop

      end
      ;; unreserved: copy as-is
      local.get $o local.get $dcap i32.ge_u
      if i64.const -2 return end
      local.get $dst local.get $o i32.add local.get $b i32.store8
      local.get $o i32.const 1 i32.add local.set $o
      br $loop
    end
    end

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; oauth_build_http_request(body_ptr, body_len,
  ;;                         host_ptr, host_len,
  ;;                         path_ptr, path_len,
  ;;                         out_ptr, out_cap) -> i64
  ;; Builds HTTP POST request with headers.
  ;; ================================================================
  (func $oauth_build_http_request (export "oauth_build_http_request")
    (param $body i32) (param $blen i32)
    (param $host i32) (param $hlen i32)
    (param $path i32) (param $plen i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $o i32)

    ;; "POST /"
    local.get $o i32.const 5 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 80 i32.store8  ;; 'P'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 79 i32.store8  ;; 'O'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 83 i32.store8  ;; 'S'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8  ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8  ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; path
    local.get $o local.get $plen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $path local.get $out local.get $o local.get $plen call $memcpy_to_off
    local.get $o local.get $plen i32.add local.set $o

    ;; " HTTP/1.1\r\n"
    local.get $o i32.const 10 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 72 i32.store8  ;; 'H'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8  ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8  ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 80 i32.store8  ;; 'P'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 47 i32.store8  ;; '/'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8  ;; '1'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 46 i32.store8  ;; '.'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8  ;; '1'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8  ;; CR
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8  ;; LF
    local.get $o i32.const 1 i32.add local.set $o

    ;; "Host: "
    local.get $o i32.const 6 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 72 i32.store8  ;; 'H'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8  ;; 'o'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 115 i32.store8  ;; 's'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8  ;; 't'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8  ;; ':'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8  ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; host value
    local.get $o local.get $hlen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $host local.get $out local.get $o local.get $hlen call $memcpy_to_off
    local.get $o local.get $hlen i32.add local.set $o

    ;; "\r\n"
    local.get $o i32.const 2 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; "Content-Type: application/x-www-form-urlencoded\r\n"
    local.get $o i32.const 47 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 67 i32.store8  ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 45 i32.store8  ;; '-'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8  ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 121 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 112 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8  ;; ':'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 97 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 112 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 112 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 108 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 105 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 99 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 97 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 105 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 47 i32.store8  ;; '/'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 120 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 45 i32.store8  ;; '-'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 119 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 119 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 119 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 45 i32.store8  ;; '-'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 102 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 114 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 109 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 45 i32.store8  ;; '-'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 117 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 114 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 108 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 99 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 100 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 100 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; "\r\nContent-Length: "
    local.get $o i32.const 18 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 67 i32.store8  ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 45 i32.store8  ;; '-'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 76 i32.store8  ;; 'L'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 103 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 104 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8  ;; ':'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; Content-Length decimal
    local.get $blen local.get $out local.get $o call $emit_u32_dec
    local.set $o

    ;; "\r\nConnection: close\r\n\r\n"
    local.get $o i32.const 23 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 67 i32.store8  ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 99 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 105 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 110 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8  ;; ':'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 99 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 108 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 115 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 101 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; body
    local.get $o local.get $blen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $body local.get $out local.get $o local.get $blen call $memcpy_to_off
    local.get $o local.get $blen i32.add local.set $o

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; oauth_parse_http_response(resp_ptr, resp_len, body_out_ptr, body_out_cap) -> i64
  ;; Find \r\n\r\n in HTTP response, return body after headers.
  ;; Returns packed (status, body_len)
  ;; ================================================================
  (func $oauth_parse_http_response (export "oauth_parse_http_response")
    (param $resp i32) (param $rlen i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $i i32) (local $blen i32)

    block $done
    loop $scan
      local.get $i i32.const 3 i32.add local.get $rlen i32.ge_u
      if i64.const -1 return end

      local.get $resp local.get $i i32.add i32.load8_u
      i32.const 13 i32.ne  ;; CR
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $resp local.get $i i32.add i32.load8_u offset=1
      i32.const 10 i32.ne  ;; LF
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $resp local.get $i i32.add i32.load8_u offset=2
      i32.const 13 i32.ne  ;; CR
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      local.get $resp local.get $i i32.add i32.load8_u offset=3
      i32.const 10 i32.ne  ;; LF
      if local.get $i i32.const 1 i32.add local.set $i br $scan end

      ;; found \r\n\r\n — body starts at $i+4
      local.get $i i32.const 4 i32.add local.set $i
      local.get $rlen local.get $i i32.sub local.set $blen
      local.get $blen local.get $ocap i32.gt_u
      if i64.const -2 return end
      local.get $resp local.get $i i32.add
      local.get $out
      local.get $blen
      call $memcpy
      br $done
    end
    end

    i64.const 0
    local.get $blen
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; oauth_token_exchange(body_ptr, body_len,
  ;;                      host_ptr, host_len,
  ;;                      path_ptr, path_len,
  ;;                      port,
  ;;                      tok_out, tok_cap,
  ;;                      ses_out, ses_cap) -> i64
  ;;
  ;; HTTPS POST OAuth token exchange via abstract socket.
  ;; Host imports sock_open/send/recv/close provide transport.
  ;;
  ;; Output: tok_out[0..3]=access_token_len, tok_out[4..]=token data
  ;;         ses_out[0..3]=session_id_len,   ses_out[4..]=session data
  ;; Return: (0, access_token_len) packed i64 on success, negative on error
  ;; ================================================================
  (data (i32.const 2048) "access_token")
  (data (i32.const 2064) "session_id")
  (data (i32.const 2080) "/oauth2/token")

  (func (export "oauth_token_exchange")
    (param $body i32) (param $blen i32)
    (param $host i32) (param $hlen i32)
    (param $path i32) (param $plen i32)
    (param $port i32)
    (param $tok_out i32) (param $tok_cap i32)
    (param $ses_out i32) (param $ses_cap i32)
    (result i64)
    (local $req_off i32) (local $req_cap i32)
    (local $cfg_off i32)
    (local $resp_off i32) (local $resp_cap i32)
    (local $body_off i32) (local $rcv i32) (local $n i32)
    (local $fd i32)
    (local $tmp_i64 i64)
    (local $tok_len i32) (local $ses_len i32)

    i32.const 4096 local.set $req_off
    i32.const 2048 local.set $req_cap
    i32.const 6144 local.set $cfg_off
    i32.const 7168 local.set $resp_off
    i32.const 2000 local.set $resp_cap
    i32.const 9216 local.set $body_off

    ;; 1. Build TLS config struct at cfg_off
    local.get $cfg_off i32.const 0 i32.add local.get $host i32.store
    local.get $cfg_off i32.const 4 i32.add local.get $hlen i32.store
    local.get $cfg_off i32.const 8 i32.add local.get $port i32.store16

    ;; 2. sock_open(SOCK_TLS=1, cfg, 12) -> fd or negative error
    i32.const 1 local.get $cfg_off i32.const 12 call $sock_open
    local.tee $fd
    i32.const 0 i32.lt_s
    if i64.const -2 return end

    ;; 3. Build HTTP POST request at req_off
    local.get $body local.get $blen
    local.get $host local.get $hlen
    local.get $path local.get $plen
    local.get $req_off local.get $req_cap
    call $oauth_build_http_request
    local.set $tmp_i64

    ;; check status (low 32 bits)
    local.get $tmp_i64 i32.wrap_i64
    i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close drop
      i64.const -1 return
    end

    ;; extract written request length (high 32 bits)
    local.get $tmp_i64 i64.const 32 i64.shr_u i32.wrap_i64
    local.set $n

    ;; 4. sock_send(fd, request, n)
    local.get $fd local.get $req_off local.get $n call $sock_send
    i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close drop
      i64.const -3 return
    end

    ;; 5. sock_recv loop — receive response into resp_off
    i32.const 0 local.set $n
    block $recv_done
    loop $recv_loop
      local.get $n local.get $resp_cap i32.ge_u
      if i32.const -4 local.set $rcv br $recv_done end

      local.get $fd
      local.get $resp_off local.get $n i32.add
      local.get $resp_cap local.get $n i32.sub
      call $sock_recv
      local.tee $rcv
      i32.const 0 i32.le_s
      if
        local.get $rcv i32.const 0 i32.lt_s
        if i32.const -4 local.set $rcv br $recv_done end
        br $recv_done
      end
      local.get $n local.get $rcv i32.add local.set $n
      br $recv_loop
    end
    end
    local.get $rcv i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close drop
      local.get $rcv i64.extend_i32_s return
    end

    ;; 6. Parse HTTP response — extract body to body_off
    local.get $resp_off local.get $n
    local.get $body_off local.get $resp_cap
    call $oauth_parse_http_response
    local.set $tmp_i64

    local.get $tmp_i64 i32.wrap_i64
    i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close drop
      i64.const -5 return
    end
    local.get $tmp_i64 i64.const 32 i64.shr_u i32.wrap_i64
    local.set $n    ;; body length

    ;; 7. Parse JSON for access_token
    local.get $body_off local.get $n
    i32.const 2048 i32.const 12                        ;; key "access_token"
    local.get $tok_out i32.const 4 i32.add
    local.get $tok_cap i32.const 4 i32.sub
    call $oauth_json_value
    local.set $tmp_i64

    local.get $tmp_i64 i32.wrap_i64
    i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close drop
      i64.const -6 return
    end
    local.get $tmp_i64 i64.const 32 i64.shr_u i32.wrap_i64
    local.set $tok_len
    local.get $tok_out local.get $tok_len i32.store

    ;; 8. Parse JSON for session_id (optional)
    local.get $body_off local.get $n
    i32.const 2064 i32.const 10                        ;; key "session_id"
    local.get $ses_out i32.const 4 i32.add
    local.get $ses_cap i32.const 4 i32.sub
    call $oauth_json_value
    local.set $tmp_i64

    local.get $tmp_i64 i32.wrap_i64
    i32.const 0 i32.lt_s
    if
      i32.const 0 local.set $ses_len
    else
      local.get $tmp_i64 i64.const 32 i64.shr_u i32.wrap_i64
      local.set $ses_len
    end
    local.get $ses_out local.get $ses_len i32.store

    ;; 9. Close socket
    local.get $fd call $sock_close drop

    i64.const 0
    local.get $tok_len
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ================================================================
  ;; helpers
  ;; ================================================================

  ;; memcpy(src, dst, len)
  (func $memcpy (param $src i32) (param $dst i32) (param $len i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $i i32.add
      local.get $src local.get $i i32.add i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; memcpy_to_off(src, dst, dst_off, len) — copies len bytes from src to dst+dst_off
  (func $memcpy_to_off (param $src i32) (param $dst i32) (param $off i32) (param $len i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $dst local.get $off i32.add local.get $i i32.add
      local.get $src local.get $i i32.add i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; emit_u32_dec(n, out, offset) -> new_offset
  ;; Renders n as decimal ASCII at out[offset..] and returns new offset
  (func $emit_u32_dec (param $n i32) (param $out i32) (param $off i32) (result i32)
    (local $buf i32) (local $digits i32) (local $d i32)

    i32.const 768 local.set $buf  ;; scratch
    i32.const 0 local.set $digits

    ;; handle zero
    local.get $n i32.eqz
    if
      local.get $out local.get $off i32.add i32.const 48 i32.store8
      local.get $off i32.const 1 i32.add
      return
    end

    ;; extract digits (reversed)
    block $extract_done
    loop $extract
      local.get $n i32.eqz br_if $extract_done
      local.get $buf local.get $digits i32.add
      local.get $n i32.const 10 i32.rem_u
      i32.const 48 i32.add
      i32.store8
      local.get $digits i32.const 1 i32.add local.set $digits
      local.get $n i32.const 10 i32.div_u local.set $n
      br $extract
    end
    end

    ;; emit in reverse
    block $emit_done
    loop $emit_loop
      local.get $d local.get $digits i32.ge_u br_if $emit_done
      local.get $out local.get $off i32.add local.get $d i32.add
      local.get $buf
      local.get $digits i32.const 1 i32.sub local.get $d i32.sub
      i32.add
      i32.load8_u
      i32.store8
      local.get $d i32.const 1 i32.add local.set $d
      br $emit_loop
    end
    end

    local.get $off
    local.get $digits
    i32.add)

  ;; end helpers
)
