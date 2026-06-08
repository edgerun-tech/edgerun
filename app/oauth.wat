;; OAuth 2.0 authorization code flow + PKCE — URL building and token body.
  ;; Uses memory for output buffers — caller reads from linear memory.

  ;; Data tables for URL construction

  ;; oauth_build_auth_url(client_id_ptr, client_id_len, scope_ptr, scope_len,
  ;;                       state_ptr, state_len,
  ;;                       challenge_ptr, challenge_len,
  ;;                       out_ptr, out_cap) -> status:i32, written:i32 packed as i64
  ;; Builds the full authorization URL with all query parameters.
  (func (export "oauth_build_auth_url") (param $cid i32) (param $cidl i32)
                                        (param $scp i32) (param $scpl i32)
                                        (param $st i32) (param $stl i32)
                                        (param $ch i32) (param $chl i32)
                                        (param $out i32) (param $ocap i32) (result i64)
    (local $o i32)
    (local $total i32)

    ;; estimate total length
    i32.const 512
    call $strlen        ;; auth prefix
    local.get $cidl
    i32.add
    i32.const 648
    call $strlen        ;; redirect_uri tag
    i32.add
    local.get $scpl
    i32.add
    i32.const 832
    call $strlen        ;; state tag
    i32.add
    local.get $stl
    i32.add
    i32.const 864
    call $strlen        ;; challenge tag
    i32.add
    local.get $chl
    i32.add
    i32.const 896
    call $strlen        ;; method tag
    i32.add
    local.set $total

    local.get $ocap
    local.get $total
    i32.lt_u
    if
      i64.const -1
      return
    end

    i32.const 0
    local.set $o

    ;; copy prefix
    i32.const 512
    local.get $o
    local.get $out
    i32.add
    i32.const 512
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 512
    call $strlen
    i32.add
    local.set $o

    ;; copy client_id
    local.get $cid
    local.get $out
    local.get $o
    i32.add
    local.get $cidl
    call $m137memcpy
    local.get $o
    local.get $cidl
    i32.add
    local.set $o

    ;; copy redirect_uri tag + scope tag
    i32.const 648
    local.get $out
    local.get $o
    i32.add
    i32.const 648
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 648
    call $strlen
    i32.add
    local.set $o

    ;; copy scope
    local.get $scp
    local.get $out
    local.get $o
    i32.add
    local.get $scpl
    call $m137memcpy
    local.get $o
    local.get $scpl
    i32.add
    local.set $o

    ;; copy state tag + state
    i32.const 832
    local.get $out
    local.get $o
    i32.add
    i32.const 832
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 832
    call $strlen
    i32.add
    local.set $o

    local.get $st
    local.get $out
    local.get $o
    i32.add
    local.get $stl
    call $m137memcpy
    local.get $o
    local.get $stl
    i32.add
    local.set $o

    ;; copy challenge tag + challenge
    i32.const 864
    local.get $out
    local.get $o
    i32.add
    i32.const 864
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 864
    call $strlen
    i32.add
    local.set $o

    local.get $ch
    local.get $out
    local.get $o
    i32.add
    local.get $chl
    call $m137memcpy
    local.get $o
    local.get $chl
    i32.add
    local.set $o

    ;; copy method tag
    i32.const 896
    local.get $out
    local.get $o
    i32.add
    i32.const 896
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 896
    call $strlen
    i32.add
    local.set $o

    ;; Done
    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; oauth_build_token_body(client_id, cid_len, code, code_len,
  ;;                        verifier, verifier_len,
  ;;                        out_ptr, out_cap) -> status:i32, written:i32 packed as i64
  (func (export "oauth_build_token_body") (param $cid i32) (param $cidl i32)
                                          (param $code i32) (param $codel i32)
                                          (param $ver i32) (param $verl i32)
                                          (param $out i32) (param $ocap i32) (result i64)
    (local $o i32)
    (local $total i32)

    i32.const 928
    call $strlen       ;; grant_type + client_id tag
    local.get $cidl
    i32.add
    i32.const 976
    call $strlen       ;; redirect_uri + code tag
    i32.add
    local.get $codel
    i32.add
    i32.const 1072
    call $strlen       ;; verifier tag
    i32.add
    local.get $verl
    i32.add
    local.set $total

    local.get $ocap
    local.get $total
    i32.lt_u
    if
      i64.const -1
      return
    end

    i32.const 0
    local.set $o

    ;; grant_type prefix
    i32.const 928
    local.get $out
    local.get $o
    i32.add
    i32.const 928
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 928
    call $strlen
    i32.add
    local.set $o

    ;; client_id
    local.get $cid
    local.get $out
    local.get $o
    i32.add
    local.get $cidl
    call $m137memcpy
    local.get $o
    local.get $cidl
    i32.add
    local.set $o

    ;; redirect_uri prefix
    i32.const 976
    local.get $out
    local.get $o
    i32.add
    i32.const 976
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 976
    call $strlen
    i32.add
    local.set $o

    ;; code
    local.get $code
    local.get $out
    local.get $o
    i32.add
    local.get $codel
    call $m137memcpy
    local.get $o
    local.get $codel
    i32.add
    local.set $o

    ;; verifier tag
    i32.const 1072
    local.get $out
    local.get $o
    i32.add
    i32.const 1072
    call $strlen
    call $m137memcpy_fixed
    local.get $o
    i32.const 1072
    call $strlen
    i32.add
    local.set $o

    ;; verifier
    local.get $ver
    local.get $out
    local.get $o
    i32.add
    local.get $verl
    call $m137memcpy
    local.get $o
    local.get $verl
    i32.add
    local.set $o

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; --- helpers ---

  ;; memcpy(src, dst, len)
  (func $m137memcpy (param $src i32) (param $dst i32) (param $len i32)
    local.get $dst
    local.get $src
    local.get $len
    call $memcpy)

  ;; memcpy_fixed(src, dst, len)
  (func $m137memcpy_fixed (param $src i32) (param $dst i32) (param $len i32)
    local.get $dst
    local.get $src
    local.get $len
    call $memcpy)


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
  (func $oauth_url_encode (export "oauth_url_encode")
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
    local.get $path local.get $out local.get $o local.get $plen call $m138memcpy_to_off
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
    local.get $host local.get $out local.get $o local.get $hlen call $m138memcpy_to_off
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
    local.get $blen local.get $out local.get $o call $m138emit_u32_dec
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
    local.get $body local.get $out local.get $o local.get $blen call $m138memcpy_to_off
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
      call $m138memcpy
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
    ;; sock_open not yet implemented — skip socket
    ;; i32.const 1 local.get $cfg_off i32.const 12 call $sock_open
    ;; local.tee $fd
    ;; i32.const 0 i32.lt_s
    ;; if i64.const -2 return end
    i32.const -1 local.set $fd

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
      local.get $fd call $sock_close
      i64.const -1 return
    end

    ;; extract written request length (high 32 bits)
    local.get $tmp_i64 i64.const 32 i64.shr_u i32.wrap_i64
    local.set $n

    ;; 4. sock_send(fd, request, n)
    local.get $fd local.get $req_off local.get $n call $sock_send
    i32.const 0 i32.lt_s
    if
      local.get $fd call $sock_close
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
      local.get $fd call $sock_close
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
      local.get $fd call $sock_close
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
      local.get $fd call $sock_close
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
    local.get $fd call $sock_close

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
  (func $m138memcpy (param $src i32) (param $dst i32) (param $len i32)
    local.get $dst
    local.get $src
    local.get $len
    call $memcpy)

  ;; memcpy_to_off(src, dst, dst_off, len) — copies len bytes from src to dst+dst_off
  (func $m138memcpy_to_off (param $src i32) (param $dst i32) (param $off i32) (param $len i32)
    local.get $dst
    local.get $off
    local.get $src
    i32.const 0
    local.get $len
    call $memcpy_off)

  ;; emit_u32_dec(n, out, offset) -> new_offset
  ;; Renders n as decimal ASCII at out[offset..] and returns new offset
  (func $m138emit_u32_dec (param $n i32) (param $out i32) (param $off i32) (result i32)
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

;; OAuth PKCE (RFC 7636) utility functions.
  ;; Pure computation — caller provides randomness, WAT transforms.

  ;; unreserved chars for verifier: A-Z a-z 0-9 - . _ ~

  ;; pkce_bias_verifier(random_ptr, random_len, out_ptr, out_cap) -> status:i32, len:i32 packed as i64
  ;; Bias-reduces random bytes into unreserved RFC 3986 charset.
  ;; Returns (0, verifier_len) on success, (-1, 0) on error.
  (func (export "pkce_bias_verifier") (param $rand i32) (param $rlen i32) (param $out i32) (param $ocap i32) (result i64)
    (local $i i32)
    (local $j i32)
    (local $b i32)

    ;; verify capacity ≥ rlen
    local.get $ocap
    local.get $rlen
    i32.lt_u
    if
      i64.const -1
      return
    end

    i32.const 0
    local.set $j

    block $done
    loop $loop
      local.get $j
      local.get $rlen
      i32.ge_u
      br_if $done

      local.get $rand
      local.get $j
      i32.add
      i32.load8_u
      local.set $b

      ;; bias-reduce: index = b % 66 (unreserved_chars_len)
      local.get $b
      i32.const 66
      i32.rem_u
      local.set $i

      i32.const 256
      local.get $i
      i32.add
      i32.load8_u
      local.get $out
      local.get $j
      i32.add
      i32.store8

      local.get $j
      i32.const 1
      i32.add
      local.set $j
      br $loop
    end
    end

    i64.const 0
    local.get $j
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; pkce_challenge(verifier_ptr, verifier_len, out_ptr, out_cap) -> i32 status
  ;; Computes SHA-256(verifier) → base64url encode → stores at out_ptr.
  ;; Status: 0=ok, -1=need hash, -2=need base64, -3=output too small.
  ;;
  ;; NOTE: SHA-256 is NOT implemented here — caller must provide hash via
  ;;       pkce_challenge_prehashed(sha256_out32, out_ptr, out_cap).
  (func (export "pkce_challenge_prehashed") (param $hash i32) (param $out i32) (param $ocap i32) (result i32)
    (local $src_i i32)
    (local $dst_i i32)
    (local $b0 i32) (local $b1 i32) (local $b2 i32)
    (local $triple i32)
    ;; SHA-256 hash is 32 bytes → base64url = 43 chars (ceil(32*4/3) = 43)
    local.get $ocap
    i32.const 43
    i32.lt_u
    if
      i32.const -3
      return
    end

    i32.const 0
    local.set $src_i
    i32.const 0
    local.set $dst_i

    ;; encode 30 bytes as 10 groups of 3 → 40 chars (no padding)
    block $enc_done
    loop $enc_loop
      local.get $src_i
      i32.const 30
      i32.ge_u
      br_if $enc_done

      ;; load 3 bytes
      local.get $hash
      local.get $src_i
      i32.add
      i32.load8_u
      local.set $b0
      local.get $hash
      local.get $src_i
      i32.const 1
      i32.add
      i32.add
      i32.load8_u
      local.set $b1
      local.get $hash
      local.get $src_i
      i32.const 2
      i32.add
      i32.add
      i32.load8_u
      local.set $b2
      local.get $src_i
      i32.const 3
      i32.add
      local.set $src_i

      local.get $b0
      i32.const 16
      i32.shl
      local.get $b1
      i32.const 8
      i32.shl
      i32.or
      local.get $b2
      i32.or
      local.set $triple

      ;; 4 base64url chars
      local.get $out
      local.get $dst_i
      i32.add
      local.get $triple
      i32.const 18
      i32.shr_u
      i32.const 63
      i32.and
      call $m139b64url_char
      i32.store8
      local.get $dst_i
      i32.const 1
      i32.add
      local.set $dst_i

      local.get $out
      local.get $dst_i
      i32.add
      local.get $triple
      i32.const 12
      i32.shr_u
      i32.const 63
      i32.and
      call $m139b64url_char
      i32.store8
      local.get $dst_i
      i32.const 1
      i32.add
      local.set $dst_i

      local.get $out
      local.get $dst_i
      i32.add
      local.get $triple
      i32.const 6
      i32.shr_u
      i32.const 63
      i32.and
      call $m139b64url_char
      i32.store8
      local.get $dst_i
      i32.const 1
      i32.add
      local.set $dst_i

      local.get $out
      local.get $dst_i
      i32.add
      local.get $triple
      i32.const 63
      i32.and
      call $m139b64url_char
      i32.store8
      local.get $dst_i
      i32.const 1
      i32.add
      local.set $dst_i

      br $enc_loop
    end
    end

    ;; remaining 2 bytes (positions 30,31) → 3 base64url chars
    local.get $hash
    i32.const 30
    i32.add
    i32.load8_u
    local.set $b0
    local.get $hash
    i32.const 31
    i32.add
    i32.load8_u
    local.set $b1

    local.get $out
    local.get $dst_i
    i32.add
    local.get $b0
    i32.const 2
    i32.shl
    local.get $b1
    i32.const 6
    i32.shr_u
    i32.or
    i32.const 63
    i32.and
    call $m139b64url_char
    i32.store8
    local.get $dst_i
    i32.const 1
    i32.add
    local.set $dst_i

    local.get $out
    local.get $dst_i
    i32.add
    local.get $b1
    i32.const 2
    i32.shl
    i32.const 63
    i32.and
    call $m139b64url_char
    i32.store8
    local.get $dst_i
    i32.const 1
    i32.add
    local.set $dst_i

    local.get $out
    local.get $dst_i
    i32.add
    local.get $b1
    i32.const 4
    i32.shr_u
    i32.const 15
    i32.and
    call $m139b64url_char
    i32.store8
    local.get $dst_i
    i32.const 1
    i32.add
    local.set $dst_i

    i32.const 0)

  (func $m139b64url_char (param $n i32) (result i32)
    local.get $n
    i32.const 26
    i32.lt_u
    if (result i32)
      local.get $n
      i32.const 65
      i32.add
    else
      local.get $n
      i32.const 52
      i32.lt_u
      if (result i32)
        local.get $n
        i32.const 26
        i32.sub
        i32.const 97
        i32.add
      else
        local.get $n
        i32.const 62
        i32.lt_u
        if (result i32)
          local.get $n
          i32.const 52
          i32.sub
          i32.const 48
          i32.add
        else
          local.get $n
          i32.const 62
          i32.eq
          if (result i32)
            i32.const 45
          else
            i32.const 95
          end
        end
      end
    end)

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

  (data (i32.const 512) "https://account.jagex.com/oauth2/auth?response_type=code&client_id=")
  (data (i32.const 608) "com_jagex_auth_desktop_launcher")
  (data (i32.const 648) "&redirect_uri=https://secure.runescape.com/m=weblogin/launcher-redirect&scope=")
  (data (i32.const 768) "openid+offline+gamesso.token.create+user.profile.read")
  (data (i32.const 832) "&prompt=login&state=")
  (data (i32.const 864) "&code_challenge=")
  (data (i32.const 896) "&code_challenge_method=S256")
  (data (i32.const 928) "grant_type=authorization_code&client_id=")
  (data (i32.const 976) "&redirect_uri=https://secure.runescape.com/m=weblogin/launcher-redirect&code=")
  (data (i32.const 1072) "&code_verifier=")
(data (i32.const 256) "0123456789ABCDEF")
  (data (i32.const 2048) "access_token")
  (data (i32.const 2064) "session_id")
  (data (i32.const 2080) "/oauth2/token")
  (data (i32.const 256) "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")
