(module
  ;; OAuth 2.0 authorization code flow + PKCE — URL building and token body.
  ;; Uses memory for output buffers — caller reads from linear memory.
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300502)

  ;; Data tables for URL construction
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
    call $strlen_at        ;; auth prefix
    local.get $cidl
    i32.add
    i32.const 648
    call $strlen_at        ;; redirect_uri tag
    i32.add
    local.get $scpl
    i32.add
    i32.const 832
    call $strlen_at        ;; state tag
    i32.add
    local.get $stl
    i32.add
    i32.const 864
    call $strlen_at        ;; challenge tag
    i32.add
    local.get $chl
    i32.add
    i32.const 896
    call $strlen_at        ;; method tag
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 512
    call $strlen_at
    i32.add
    local.set $o

    ;; copy client_id
    local.get $cid
    local.get $out
    local.get $o
    i32.add
    local.get $cidl
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 648
    call $strlen_at
    i32.add
    local.set $o

    ;; copy scope
    local.get $scp
    local.get $out
    local.get $o
    i32.add
    local.get $scpl
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 832
    call $strlen_at
    i32.add
    local.set $o

    local.get $st
    local.get $out
    local.get $o
    i32.add
    local.get $stl
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 864
    call $strlen_at
    i32.add
    local.set $o

    local.get $ch
    local.get $out
    local.get $o
    i32.add
    local.get $chl
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 896
    call $strlen_at
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
    call $strlen_at       ;; grant_type + client_id tag
    local.get $cidl
    i32.add
    i32.const 976
    call $strlen_at       ;; redirect_uri + code tag
    i32.add
    local.get $codel
    i32.add
    i32.const 1072
    call $strlen_at       ;; verifier tag
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 928
    call $strlen_at
    i32.add
    local.set $o

    ;; client_id
    local.get $cid
    local.get $out
    local.get $o
    i32.add
    local.get $cidl
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 976
    call $strlen_at
    i32.add
    local.set $o

    ;; code
    local.get $code
    local.get $out
    local.get $o
    i32.add
    local.get $codel
    call $memcpy
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
    call $strlen_at
    call $memcpy_fixed
    local.get $o
    i32.const 1072
    call $strlen_at
    i32.add
    local.set $o

    ;; verifier
    local.get $ver
    local.get $out
    local.get $o
    i32.add
    local.get $verl
    call $memcpy
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

  ;; strlen_at(addr) -> length (finds null terminator)
  (func $strlen_at (param $addr i32) (result i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $addr
      local.get $i
      i32.add
      i32.load8_u
      i32.eqz
      br_if $done
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $loop
    end
    end
    local.get $i)

  ;; memcpy(src, dst, len)
  (func $memcpy (param $src i32) (param $dst i32) (param $len i32)
    (local $i i32)
    block $done
    loop $loop
      local.get $i
      local.get $len
      i32.ge_u
      br_if $done
      local.get $dst
      local.get $i
      i32.add
      local.get $src
      local.get $i
      i32.add
      i32.load8_u
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $loop
    end
    end)

  ;; memcpy_fixed(src, dst, len) — source is a fixed data addr (same module)
  (func $memcpy_fixed (param $src i32) (param $dst i32) (param $len i32)
    local.get $src
    local.get $dst
    local.get $len
    call $memcpy)
)