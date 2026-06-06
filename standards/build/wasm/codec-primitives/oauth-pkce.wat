(module
  ;; OAuth PKCE (RFC 7636) utility functions.
  ;; Pure computation — caller provides randomness, WAT transforms.
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300501)

  ;; unreserved chars for verifier: A-Z a-z 0-9 - . _ ~
  (data (i32.const 256) "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")

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
      call $b64url_char
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
      call $b64url_char
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
      call $b64url_char
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
      call $b64url_char
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
    call $b64url_char
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
    call $b64url_char
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
    call $b64url_char
    i32.store8
    local.get $dst_i
    i32.const 1
    i32.add
    local.set $dst_i

    i32.const 0)

  (func $b64url_char (param $n i32) (result i32)
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
)