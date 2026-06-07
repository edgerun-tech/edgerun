;; ws-accept — WebSocket accept handshake (key + GUID → SHA-1 → base64)


  (global $MSG_ADDR i32 (i32.const 62000))
  (global $DIGEST_ADDR i32 (i32.const 62100))

  (func $b64val (param $ch i32) (result i32)
    local.get $ch
    i32.const 65
    i32.ge_u
    local.get $ch
    i32.const 90
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 65
      i32.sub
      return
    end
    local.get $ch
    i32.const 97
    i32.ge_u
    local.get $ch
    i32.const 122
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 71
      i32.sub
      return
    end
    local.get $ch
    i32.const 48
    i32.ge_u
    local.get $ch
    i32.const 57
    i32.le_u
    i32.and
    if
      local.get $ch
      i32.const 4
      i32.add
      return
    end
    local.get $ch
    i32.const 43
    i32.eq
    if
      i32.const 62
      return
    end
    local.get $ch
    i32.const 47
    i32.eq
    if
      i32.const 63
      return
    end
    i32.const -1)

  ;; base64 encode imported from encoding-base64 as $base64_encode

  (func (export "ws_accept_key") (param $in_ptr i32) (param $in_len i32) (param $out_ptr i32) (param $out_cap i32) (result i64)
    (local $i i32)
    (local $v i32)
    (local $out_i i32)
    (local $result i64)
    (local $digest i32)
    local.get $in_len
    i32.const 24
    i32.ne
    if
      (return (call $pack (i32.const 4) (i32.const 0)))
    end
    local.get $in_ptr
    i32.const 22
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    local.get $in_ptr
    i32.const 23
    i32.add
    i32.load8_u
    i32.const 61
    i32.ne
    i32.or
    if
      (return (call $pack (i32.const 3) (i32.const 0)))
    end
    (loop $validate
      local.get $in_ptr
      local.get $i
      i32.add
      i32.load8_u
      call $b64val
      local.set $v
      local.get $v
      i32.const 0
      i32.lt_s
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 21
      i32.eq
      local.get $v
      i32.const 15
      i32.and
      i32.const 0
      i32.ne
      i32.and
      if
        (return (call $pack (i32.const 3) (i32.const 0)))
      end
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      local.get $i
      i32.const 22
      i32.lt_u
      br_if $validate)
    local.get $out_cap
    i32.const 28
    i32.lt_u
    if
      (return (call $pack (i32.const 2) (i32.const 0)))
    end
    ;; Assemble 60-byte message: key + GUID
    (local.set $i (i32.const 0))
    (loop $copy_key
      (i32.store8
        (i32.add (global.get $MSG_ADDR) (local.get $i))
        (i32.load8_u (i32.add (local.get $in_ptr) (local.get $i))))
      (local.set $i (i32.add (local.get $i) (i32.const 1)))
      (br_if $copy_key (i32.lt_u (local.get $i) (i32.const 24))))
    ;; Compute SHA-1 of message
    (local.set $result
      (call $sha1 (global.get $MSG_ADDR) (i32.const 60) (global.get $DIGEST_ADDR)))
    (local.set $v (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
    (if (local.get $v)
      (then (return (call $pack (local.get $v) (i32.const 0)))))
    (local.set $digest (global.get $DIGEST_ADDR))
    ;; Base64-encode the 20-byte digest into 28-byte output (standard base64 with padding)
    local.get $digest i32.const 20 local.get $out_ptr local.get $out_cap
    call $base64_encode
    local.set $result
    local.get $result
    i64.const 32
    i64.shr_u
    i32.wrap_i64
    if (result i64)
      local.get $result
    else
      (call $pack (i32.const 0) (i32.const 28))
    end)

;; WebSocket client — HTTP Upgrade handshake + masked frame send/recv.
  ;; Uses abstract socket for transport.
  ;; Memory layout (safe zone 0x30000+, after LUTs at 0x1000-0x2FFF):
  ;; 0x30500  request buffer (1024B)
  ;; 0x30900  response buffer (1024B)
  ;; 0x30D00  stored key (24B base64)
  ;; 0x30D18  stored accept (28B base64)
  ;; 0x30D34  stored masking key (4B BE)
  ;; 0x30000  scratch for payload (1024B)
  ;; 0x30400  frame header (256B)

  ;; ── Internal helpers ──

  ;; memcpy imported from edgerun-core as $memcpy(dst, src, len)

  ;; base64 functions now imported from encoding-base64 as $base64_encode
  ;; (replaces the buggy local $b64_3/$b64_16 implementations)

  ;; ── apply_mask(ptr, len, mask_be) ──
  ;; XOR mask with data. Mask is a big-endian i32 (byte0=MSB, byte3=LSB).
  (func $apply_mask (param $ptr i32) (param $len i32) (param $mask i32)
    (local $i i32) (local $shift i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $ptr local.get $i i32.add
      local.get $ptr local.get $i i32.add i32.load8_u
      local.get $mask
      local.get $i i32.const 3 i32.and i32.const 3 i32.xor i32.const 3 i32.shl
      i32.shr_u
      i32.const 0xFF i32.and
      i32.xor
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end)

  ;; ── ws_connect ──
  (func (export "ws_connect")
    (param $sock_type i32) (param $cfg i32) (param $cfg_len i32)
    (param $path i32) (param $plen i32)
    (param $key16 i32)
    (result i64)
    (local $req i32) (local $rsp i32) (local $p i32)
    (local $fd i32) (local $rc i32) (local $rlen i32)
    (local $b i32)

    i32.const 0x30500 local.set $req
    i32.const 0x30900 local.set $rsp

    ;; 1. Base64-encode key → 0x30D00 (24 bytes padding)
    local.get $key16 i32.const 16 i32.const 0x30D00 i32.const 24
    call $base64_encode drop

    ;; 2. Copy first 4 key bytes as mask → 6196 (stored big-endian)
    i32.const 0x30D34 local.get $key16 i32.load align=1 i32.store

    ;; 3. Build HTTP Upgrade request
    local.get $req local.set $p

    ;; "GET "
    local.get $p i32.const 0x20544547 i32.store
    local.get $p i32.const 4 i32.add local.set $p

    ;; path
    local.get $p local.get $path local.get $plen call $memcpy
    local.get $p local.get $plen i32.add local.set $p

    ;; " HTTP/1.1\r\n" — 11 bytes
    local.get $p i32.const 0 i32.add i32.const 0x20 i32.store8
    local.get $p i32.const 1 i32.add i32.const 0x48 i32.store8
    local.get $p i32.const 2 i32.add i32.const 0x54 i32.store8
    local.get $p i32.const 3 i32.add i32.const 0x54 i32.store8
    local.get $p i32.const 4 i32.add i32.const 0x50 i32.store8
    local.get $p i32.const 5 i32.add i32.const 0x2F i32.store8
    local.get $p i32.const 6 i32.add i32.const 0x31 i32.store8
    local.get $p i32.const 7 i32.add i32.const 0x2E i32.store8
    local.get $p i32.const 8 i32.add i32.const 0x31 i32.store8
    local.get $p i32.const 9 i32.add i32.const 0x0D i32.store8
    local.get $p i32.const 10 i32.add i32.const 0x0A i32.store8
    local.get $p i32.const 11 i32.add local.set $p

    ;; "Upgrade: websocket\r\n" — 20 bytes
    local.get $p i32.const 0 i32.add i32.const 0x72677055 i32.store
    local.get $p i32.const 4 i32.add i32.const 0x3A656461 i32.store
    local.get $p i32.const 8 i32.add i32.const 0x62657720 i32.store
    local.get $p i32.const 12 i32.add i32.const 0x6B636F73 i32.store
    local.get $p i32.const 16 i32.add i32.const 0x0A0D7465 i32.store
    local.get $p i32.const 20 i32.add local.set $p

    ;; "Connection: Upgrade\r\n" — 21 bytes
    local.get $p i32.const 0 i32.add i32.const 0x6E6E6F43 i32.store
    local.get $p i32.const 4 i32.add i32.const 0x69746365 i32.store
    local.get $p i32.const 8 i32.add i32.const 0x203A6E6F i32.store
    local.get $p i32.const 12 i32.add i32.const 0x72677055 i32.store
    local.get $p i32.const 16 i32.add i32.const 0x0D656461 i32.store
    local.get $p i32.const 20 i32.add i32.const 0x0A i32.store8
    local.get $p i32.const 21 i32.add local.set $p

    ;; "Sec-WebSocket-Key: " — 19 bytes
    local.get $p i32.const 0 i32.add i32.const 0x2D636553 i32.store
    local.get $p i32.const 4 i32.add i32.const 0x53626557 i32.store
    local.get $p i32.const 8 i32.add i32.const 0x656B636F i32.store
    local.get $p i32.const 12 i32.add i32.const 0x654B2D74 i32.store
    local.get $p i32.const 16 i32.add i32.const 0x79 i32.store8
    local.get $p i32.const 17 i32.add i32.const 0x3A i32.store8
    local.get $p i32.const 18 i32.add i32.const 0x20 i32.store8
    local.get $p i32.const 19 i32.add local.set $p

    ;; key (24 bytes from 6144)
    local.get $p i32.const 0x30D00 i32.const 24 call $memcpy
    local.get $p i32.const 24 i32.add local.set $p

    ;; "\r\n"
    local.get $p i32.const 0 i32.add i32.const 0x0A0D i32.store16
    local.get $p i32.const 2 i32.add local.set $p

    ;; "Sec-WebSocket-Version: 13\r\n" — 27 bytes
    local.get $p i32.const 0 i32.add i32.const 0x2D636553 i32.store
    local.get $p i32.const 4 i32.add i32.const 0x53626557 i32.store
    local.get $p i32.const 8 i32.add i32.const 0x656B636F i32.store
    local.get $p i32.const 12 i32.add i32.const 0x654B2D74 i32.store
    local.get $p i32.const 16 i32.add i32.const 0x3A79 i32.store16  ;; "y:"
    local.get $p i32.const 18 i32.add i32.const 0x20 i32.store8
    local.get $p i32.const 19 i32.add i32.const 0x31 i32.store8
    local.get $p i32.const 20 i32.add i32.const 0x33 i32.store8
    local.get $p i32.const 21 i32.add i32.const 0x0D i32.store8
    local.get $p i32.const 22 i32.add i32.const 0x0A i32.store8
    local.get $p i32.const 23 i32.add local.set $p

    ;; final "\r\n"
    local.get $p i32.const 0 i32.add i32.const 0x0A0D i32.store16
    local.get $p i32.const 2 i32.add local.set $p

    ;; 4. sock_open
    local.get $sock_type local.get $cfg local.get $cfg_len call $sock_open
    local.tee $fd
    i32.const 0 i32.lt_s
    if i64.const -2 return end

    ;; 5. Send request
    local.get $fd local.get $req local.get $p local.get $req i32.sub call $sock_send
    i32.const 0 i32.lt_s
    if local.get $fd call $sock_close drop i64.const -3 return end

    ;; 6. Receive response (up to 1024 bytes)
    local.get $fd local.get $rsp i32.const 1024 call $sock_recv
    local.tee $rc
    i32.const 0 i32.le_s
    if local.get $fd call $sock_close drop i64.const -4 return end
    local.get $rc local.set $rlen

    ;; 7. Validate 101 status line
    local.get $rlen i32.const 14 i32.lt_u
    if local.get $fd call $sock_close drop i64.const -5 return end

    ;; "HTTP/1.1 101\r\n" — 14 bytes minimum
    local.get $rsp i32.load i32.const 0x50545448 i32.ne
    if local.get $fd call $sock_close drop i64.const -5 return end
    local.get $rsp i32.load offset=4 i32.const 0x312E312F i32.ne
    if local.get $fd call $sock_close drop i64.const -5 return end
    local.get $rsp i32.load8_u offset=8 i32.const 0x20 i32.ne
    if local.get $fd call $sock_close drop i64.const -5 return end
    local.get $rsp i32.load8_u offset=9 i32.const 0x31 i32.ne
    if local.get $fd call $sock_close drop i64.const -6 return end
    local.get $rsp i32.load8_u offset=10 i32.const 0x30 i32.ne
    if local.get $fd call $sock_close drop i64.const -6 return end
    local.get $rsp i32.load8_u offset=11 i32.const 0x31 i32.ne
    if local.get $fd call $sock_close drop i64.const -6 return end

    ;; 8. Scan headers for required values
    local.get $rsp local.get $rlen call $scan_ws_headers
    i32.eqz
    if local.get $fd call $sock_close drop i64.const -6 return end

    ;; 9. Extract Sec-WebSocket-Accept value → 6168
    local.get $rsp local.get $rlen i32.const 0x30D18 call $extract_accept

    ;; 10. Return packed(0, fd)
    i64.const 0
    local.get $fd
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; ── scan_ws_headers(rsp, rlen) -> bool ──
  ;; Returns 1 if both "upgrade: websocket" and "connection: upgrade" found.
  (func $scan_ws_headers (param $rsp i32) (param $rlen i32) (result i32)
    (local $i i32) (local $b i32)
    (local $found_upgrade i32) (local $found_conn i32)

    i32.const 14 local.set $i
    i32.const 0 local.set $found_upgrade
    i32.const 0 local.set $found_conn

    block $done
    loop $scan
      local.get $i local.get $rlen i32.ge_u br_if $done

      ;; Check end-of-headers (\r\n\r\n)
      local.get $rsp local.get $i i32.add i32.load8_u i32.const 0x0D i32.eq
      if
        local.get $i i32.const 1 i32.add local.get $rlen i32.lt_u
        if
          local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0x0A i32.eq
          if
            local.get $i i32.const 2 i32.add local.get $rlen i32.lt_u
            if
              local.get $rsp local.get $i i32.const 2 i32.add i32.add i32.load8_u i32.const 0x0D i32.eq
              if
                local.get $i i32.const 3 i32.add local.get $rlen i32.lt_u
                if
                  local.get $rsp local.get $i i32.const 3 i32.add i32.add i32.load8_u i32.const 0x0A i32.eq
                  if
                    local.get $found_upgrade i32.eqz if i32.const 0 return end
                    local.get $found_conn i32.eqz if i32.const 0 return end
                    i32.const 1 return
                  end
                end
              end
            end
          end
        end
      end

      ;; Check "Upgrade:" header name (case-insensitive)
      local.get $i i32.const 8 i32.add local.get $rlen i32.le_u
      if
        local.get $rsp local.get $i i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x55 i32.eq
        if
          local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x50 i32.eq
          if
            local.get $rsp local.get $i i32.const 2 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x47 i32.eq
            if
              local.get $rsp local.get $i i32.const 3 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x52 i32.eq
              if
                local.get $rsp local.get $i i32.const 4 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x41 i32.eq
                if
                  local.get $rsp local.get $i i32.const 5 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x44 i32.eq
                  if
                    local.get $rsp local.get $i i32.const 6 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x45 i32.eq
                    if
                      local.get $rsp local.get $i i32.const 7 i32.add i32.add i32.load8_u i32.const 0x3A i32.eq
                      if
                        i32.const 1 local.set $found_upgrade
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end

      ;; Check "Connection:" header name (case-insensitive)
      local.get $i i32.const 11 i32.add local.get $rlen i32.le_u
      if
        local.get $rsp local.get $i i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x43 i32.eq
        if
          local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x4F i32.eq
          if
            local.get $rsp local.get $i i32.const 2 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x4E i32.eq
            if
              local.get $rsp local.get $i i32.const 3 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x4E i32.eq
              if
                local.get $rsp local.get $i i32.const 4 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x45 i32.eq
                if
                  local.get $rsp local.get $i i32.const 5 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x43 i32.eq
                  if
                    local.get $rsp local.get $i i32.const 6 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x54 i32.eq
                    if
                      local.get $rsp local.get $i i32.const 7 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x49 i32.eq
                      if
                        local.get $rsp local.get $i i32.const 8 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x4F i32.eq
                        if
                          local.get $rsp local.get $i i32.const 9 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x4E i32.eq
                          if
                            local.get $rsp local.get $i i32.const 10 i32.add i32.add i32.load8_u i32.const 0x3A i32.eq
                            if
                              i32.const 1 local.set $found_conn
                            end
                          end
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end

      ;; Advance to next line
      block $next
      loop $crlf
        local.get $i local.get $rlen i32.ge_u br_if $done
        local.get $rsp local.get $i i32.add i32.load8_u i32.const 0x0D i32.eq
        if
          local.get $i i32.const 1 i32.add local.get $rlen i32.lt_u
          if
            local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0x0A i32.eq
            if local.get $i i32.const 2 i32.add local.set $i br $next end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $crlf
      end
      end
      br $scan
    end
    end
    i32.const 0)

  ;; ── extract_accept(rsp, rlen, out) ──
  ;; Finds Sec-WebSocket-Accept header value, copies to out (max 28 bytes).
  (func $extract_accept (param $rsp i32) (param $rlen i32) (param $out i32)
    (local $i i32) (local $j i32) (local $b i32)

    i32.const 14 local.set $i
    block $done
    loop $scan
      local.get $i local.get $rlen i32.ge_u br_if $done

      ;; Check "Sec-WebSocket-Accept:" (22 bytes, case-insensitive)
      local.get $i i32.const 22 i32.add local.get $rlen i32.le_u
      if
        local.get $rsp local.get $i i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x53 i32.eq
        if
          local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x45 i32.eq
          if
            local.get $rsp local.get $i i32.const 2 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x43 i32.eq
            if
              local.get $rsp local.get $i i32.const 3 i32.add i32.add i32.load8_u i32.const 0x2D i32.eq
              if
                ;; Found "Sec-" — scan forward for "Accept:"
                local.get $i i32.const 18 i32.add local.set $i  ;; skip to "Accept:" check
                ;; Actually, let me just scan for "Accept:" at $i+18
                ;; The header is "Sec-WebSocket-Accept:", so after "Sec-",
                ;; "WebSocket-Accept:" = 17 bytes, so skip to $i+4+17 = $i+21 for the colon
                ;; Actually, the full header is 22 bytes:
                ;; S-e-c---W-e-b-S-o-c-k-e-t---A-c-c-e-p-t-:
                ;; 0 1 2 3 4 5 6 7 8 9 ...
                ;; I already checked "Sec-" at positions i..i+3.
                ;; Now check "WebSocket-Accept:" at positions i+4..i+21
                ;; But this nested scanning is getting very long. Let me simplify.
                ;; Just check positions i+4..i+7 for "WebS".
                local.get $rsp local.get $i i32.const 4 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x57 i32.eq
                if
                  local.get $rsp local.get $i i32.const 8 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x6F i32.eq
                  if
                    local.get $rsp local.get $i i32.const 12 i32.add i32.add i32.load8_u i32.const 0xDF i32.and i32.const 0x41 i32.eq
                    if
                      ;; Found the header. Skip past ": " to value.
                      local.get $rsp local.get $i i32.const 21 i32.add i32.add i32.load8_u i32.const 0x3A i32.eq
                      if
                        local.get $i i32.const 22 i32.add local.set $i  ;; past the colon
                        ;; skip whitespace
                        loop $skip_sp
                          local.get $i local.get $rlen i32.ge_u br_if $done
                          local.get $rsp local.get $i i32.add i32.load8_u local.tee $b
                          i32.const 0x20 i32.eq
                          if local.get $i i32.const 1 i32.add local.set $i br $skip_sp end
                          local.get $b i32.const 0x09 i32.eq
                          if local.get $i i32.const 1 i32.add local.set $i br $skip_sp end
                        end
                        ;; copy value until \r or up to 28 bytes
                        i32.const 0 local.set $j
                        loop $copy
                          local.get $i local.get $j i32.add local.get $rlen i32.ge_u br_if $done
                          local.get $rsp local.get $i local.get $j i32.add i32.add i32.load8_u
                          local.tee $b
                          i32.const 0x0D i32.eq
                          br_if $done
                          local.get $out local.get $j i32.add local.get $b i32.store8
                          local.get $j i32.const 1 i32.add local.set $j
                          local.get $j i32.const 28 i32.gt_u br_if $done
                          br $copy
                        end
                      end
                    end
                  end
                end
              end
            end
          end
        end
      end

      ;; Advance to next line
      block $next
      loop $crlf
        local.get $i local.get $rlen i32.ge_u br_if $done
        local.get $rsp local.get $i i32.add i32.load8_u i32.const 0x0D i32.eq
        if
          local.get $i i32.const 1 i32.add local.get $rlen i32.lt_u
          if
            local.get $rsp local.get $i i32.const 1 i32.add i32.add i32.load8_u i32.const 0x0A i32.eq
            if local.get $i i32.const 2 i32.add local.set $i br $next end
          end
        end
        local.get $i i32.const 1 i32.add local.set $i
        br $crlf
      end
      end
      br $scan
    end
    end)

  ;; ── ws_send(fd, opcode, data_ptr, data_len) -> status ──
  (func $ws_send (export "ws_send")
    (param $fd i32) (param $opcode i32) (param $data i32) (param $dlen i32)
    (result i32)
    (local $hdr i32) (local $hlen i32) (local $mask i32)
    (local $i i32) (local $buf i32)

    i32.const 0x30400 local.set $hdr
    i32.const 0x30D34 local.set $mask
    local.get $mask i32.load local.set $mask

    ;; Byte 0: FIN(1) | opcode
    local.get $hdr i32.const 0x80 local.get $opcode i32.or i32.store8

    ;; Byte 1+: MASK(1) | payload length
    local.get $dlen i32.const 126 i32.lt_u
    if
      local.get $hdr i32.const 1 i32.add
      i32.const 0x80 local.get $dlen i32.or i32.store8
      i32.const 2 local.set $hlen
    else
      local.get $dlen i32.const 65536 i32.lt_u
      if
        local.get $hdr i32.const 1 i32.add i32.const 0xFE i32.store8
        local.get $hdr i32.const 2 i32.add
        local.get $dlen i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
        local.get $hdr i32.const 3 i32.add
        local.get $dlen i32.const 0xFF i32.and i32.store8
        i32.const 4 local.set $hlen
      else
        local.get $hdr i32.const 1 i32.add i32.const 0xFF i32.store8
        local.get $hdr i32.const 2 i32.add i32.const 0 i32.store8
        local.get $hdr i32.const 3 i32.add i32.const 0 i32.store8
        local.get $hdr i32.const 4 i32.add i32.const 0 i32.store8
        local.get $hdr i32.const 5 i32.add i32.const 0 i32.store8
        local.get $hdr i32.const 6 i32.add
        local.get $dlen i32.const 24 i32.shr_u i32.const 0xFF i32.and i32.store8
        local.get $hdr i32.const 7 i32.add
        local.get $dlen i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8
        local.get $hdr i32.const 8 i32.add
        local.get $dlen i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8
        local.get $hdr i32.const 9 i32.add
        local.get $dlen i32.const 0xFF i32.and i32.store8
        i32.const 10 local.set $hlen
      end
    end

    ;; Append masking key (4 bytes big-endian)
    local.get $hdr local.get $hlen i32.add
    local.get $mask i32.const 24 i32.shr_u i32.const 0xFF i32.and i32.store8   ;; byte 0 of mask
    local.get $hdr local.get $hlen i32.const 1 i32.add i32.add
    local.get $mask i32.const 16 i32.shr_u i32.const 0xFF i32.and i32.store8   ;; byte 1
    local.get $hdr local.get $hlen i32.const 2 i32.add i32.add
    local.get $mask i32.const 8 i32.shr_u i32.const 0xFF i32.and i32.store8    ;; byte 2
    local.get $hdr local.get $hlen i32.const 3 i32.add i32.add
    local.get $mask i32.const 0xFF i32.and i32.store8                           ;; byte 3
    local.get $hlen i32.const 4 i32.add local.set $hlen

    ;; Send header
    local.get $fd local.get $hdr local.get $hlen call $sock_send
    i32.const 0 i32.lt_s
    if i32.const -7 return end

    ;; Payload too large?
    local.get $dlen i32.const 1024 i32.gt_u
    if i32.const -8 return end

    ;; Copy data to scratch buffer for masking
    i32.const 0x30000 local.set $buf
    local.get $buf local.get $data local.get $dlen call $memcpy

    ;; Mask payload in-place
    local.get $buf local.get $dlen local.get $mask call $apply_mask

    ;; Send masked payload
    local.get $fd local.get $buf local.get $dlen call $sock_send
    i32.const 0 i32.lt_s
    if i32.const -7 return end

    i32.const 0)

  ;; ── ws_recv(fd, out_ptr, out_cap) -> i64 ──
  ;; Returns packed(status, opcode, payload_len).
  (func (export "ws_recv")
    (param $fd i32) (param $out i32) (param $ocap i32)
    (result i64)
    (local $hdr i32) (local $i i32)
    (local $b0 i32) (local $b1 i32) (local $opcode i32)
    (local $plen i32) (local $hlen i32)
    (local $mask i32) (local $mask_present i32)
    (local $rc i32)

    i32.const 0x30100 local.set $hdr

    ;; Read first 2 bytes (frame prefix)
    local.get $fd local.get $hdr i32.const 2 call $sock_recv
    local.tee $rc
    i32.const 2 i32.ne
    if i64.const -9 return end

    local.get $hdr i32.load8_u offset=0 local.set $b0
    local.get $hdr i32.load8_u offset=1 local.set $b1

    local.get $b0 i32.const 0x0F i32.and local.set $opcode
    local.get $b1 i32.const 0x80 i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    local.set $mask_present
    local.get $b1 i32.const 0x7F i32.and local.set $plen

    ;; Decode payload length
    local.get $plen i32.const 126 i32.eq
    if
      local.get $fd local.get $hdr i32.const 2 i32.add i32.const 2 call $sock_recv
      local.tee $rc
      i32.const 2 i32.ne
      if i64.const -9 return end
      local.get $hdr i32.load8_u offset=2 i32.const 8 i32.shl
      local.get $hdr i32.load8_u offset=3 i32.or
      local.set $plen
      i32.const 4 local.set $hlen
    else
      local.get $plen i32.const 127 i32.eq
      if
        local.get $fd local.get $hdr i32.const 2 i32.add i32.const 8 call $sock_recv
        local.tee $rc
        i32.const 8 i32.ne
        if i64.const -9 return end
        local.get $hdr i32.load8_u offset=2 i32.const 24 i32.shl
        local.get $hdr i32.load8_u offset=3 i32.const 16 i32.shl
        i32.or
        local.get $hdr i32.load8_u offset=4 i32.const 8 i32.shl
        i32.or
        local.get $hdr i32.load8_u offset=5 i32.or
        if i64.const -9 return end    ;; high dword must be 0
        local.get $hdr i32.load8_u offset=6 i32.const 24 i32.shl
        local.get $hdr i32.load8_u offset=7 i32.const 16 i32.shl
        i32.or
        local.get $hdr i32.load8_u offset=8 i32.const 8 i32.shl
        i32.or
        local.get $hdr i32.load8_u offset=9 i32.or
        local.set $plen
        i32.const 10 local.set $hlen
      else
        i32.const 2 local.set $hlen
      end
    end

    ;; Read masking key if present
    local.get $mask_present
    if
      local.get $fd local.get $hdr local.get $hlen i32.add i32.const 4 call $sock_recv
      local.tee $rc
      i32.const 4 i32.ne
      if i64.const -9 return end
      local.get $hdr local.get $hlen i32.add i32.load local.set $mask
      local.get $hlen i32.const 4 i32.add local.set $hlen
    end

    ;; Validate payload fits in out buffer
    local.get $plen local.get $ocap i32.gt_u
    if i64.const -10 return end

    ;; Read payload
    local.get $fd local.get $out local.get $plen call $sock_recv
    local.tee $rc
    local.get $plen i32.ne
    if i64.const -9 return end

    ;; Unmask if present
    local.get $mask_present
    if
      local.get $out local.get $plen local.get $mask call $apply_mask
    end

    ;; Return packed(status=0, opcode, payload_len)
    i64.const 0
    local.get $opcode
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or
    local.get $plen
    i64.extend_i32_u
    i64.const 48
    i64.shl
    i64.or)

  ;; ── ws_close(fd) -> status ──
  (func (export "ws_close")
    (param $fd i32)
    (result i32)
    local.get $fd i32.const 8 i32.const 0 i32.const 0 call $ws_send
    drop
    local.get $fd call $sock_close)

;; Standard ID removed — merged into single module

  ;; Valid opcodes: 0 (continuation), 1 (text), 2 (binary),
  ;; 8 (close), 9 (ping), 10 (pong).
  (func $is_valid_opcode (param $opcode i32) (result i32)
    (i32.or
      (i32.or
        (i32.or
          (i32.eq (local.get $opcode) (i32.const 0))
          (i32.eq (local.get $opcode) (i32.const 1)))
        (i32.eq (local.get $opcode) (i32.const 2)))
      (i32.or
        (i32.or
          (i32.eq (local.get $opcode) (i32.const 8))
          (i32.eq (local.get $opcode) (i32.const 9)))
        (i32.eq (local.get $opcode) (i32.const 10)))))

  (func (export "ws_decode_prefix") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $opcode i32)
    (local $code i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b0
    i32.const 15
    i32.and
    local.set $opcode
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    local.get $opcode
    call $is_valid_opcode
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end
    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $opcode
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $code
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $code
    i32.const 126
    i32.eq
    if (result i32)
      i32.const 2
    else
      local.get $code
      i32.const 127
      i32.eq
      if (result i32)
        i32.const 8
      else
        i32.const 0
      end
    end
    i32.store
    i32.const 0)

  (func (export "ws_decode_payload_len") (param $ptr i32) (param $len i32) (param $code i32) (param $max_len i32) (param $out i32) (result i32)
    (local $low i32)
    (local $high i32)
    (local $extra i32)
    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 2
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
      else
        local.get $code
        i32.const 127
        i32.ne
        if
          i32.const 3
          return
        end
        local.get $len
        i32.const 8
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 1
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
      end
    end
    local.get $code
    i32.const 126
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $out
    local.get $low
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $extra
    i32.store
    i32.const 0)

  (func $ws_apply_mask (export "ws_apply_mask_in_place") (param $payload_ptr i32) (param $payload_len i32) (param $mask i32) (result i32)
    (local $i i32)
    (local $shift i32)
    loop $mask_loop
      local.get $i
      local.get $payload_len
      i32.ge_u
      if
        i32.const 0
        return
      end
      local.get $i
      i32.const 3
      i32.and
      i32.const 3
      i32.xor
      i32.const 8
      i32.mul
      local.set $shift
      local.get $payload_ptr
      local.get $i
      i32.add
      local.get $payload_ptr
      local.get $i
      i32.add
      i32.load8_u
      local.get $mask
      local.get $shift
      i32.shr_u
      i32.const 255
      i32.and
      i32.xor
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $mask_loop
    end
    i32.const 0)

  (func $ws_parse_header (export "ws_parse_header") (param $ptr i32) (param $len i32) (param $max_len i32) (param $out i32) (result i32)
    (local $b0 i32)
    (local $b1 i32)
    (local $code i32)
    (local $extra i32)
    (local $masked i32)
    (local $header_len i32)
    (local $low i32)
    (local $high i32)
    local.get $len
    i32.const 2
    i32.lt_u
    if
      i32.const 1
      return
    end
    local.get $ptr
    i32.load8_u
    local.set $b0
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    local.set $b1
    local.get $b1
    i32.const 127
    i32.and
    local.set $code
    i32.const 2
    local.set $header_len
    local.get $b1
    i32.const 128
    i32.and
    if (result i32)
      i32.const 1
    else
      i32.const 0
    end
    local.set $masked

    local.get $b0
    i32.const 15
    i32.and
    i32.const 0
    i32.eq
    local.get $b0
    i32.const 15
    i32.and
    i32.const 1
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 2
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 9
    i32.eq
    i32.or
    local.get $b0
    i32.const 15
    i32.and
    i32.const 10
    i32.eq
    i32.or
    i32.eqz
    if
      i32.const 3
      return
    end
    local.get $b0
    i32.const 15
    i32.and
    i32.const 8
    i32.ge_u
    if
      local.get $b0
      i32.const 128
      i32.and
      i32.eqz
      if
        i32.const 3
        return
      end
      local.get $code
      i32.const 125
      i32.gt_u
      if
        i32.const 3
        return
      end
    end

    local.get $code
    i32.const 126
    i32.lt_u
    if
      local.get $code
      local.set $low
      i32.const 0
      local.set $high
      i32.const 0
      local.set $extra
    else
      local.get $code
      i32.const 126
      i32.eq
      if
        local.get $len
        i32.const 4
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 0
        local.set $high
        i32.const 2
        local.set $extra
        i32.const 4
        local.set $header_len
      else
        local.get $len
        i32.const 10
        i32.lt_u
        if
          i32.const 1
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 128
        i32.and
        if
          i32.const 3
          return
        end
        local.get $ptr
        i32.const 2
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 3
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 4
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 5
        i32.add
        i32.load8_u
        i32.or
        local.set $high
        local.get $ptr
        i32.const 6
        i32.add
        i32.load8_u
        i32.const 24
        i32.shl
        local.get $ptr
        i32.const 7
        i32.add
        i32.load8_u
        i32.const 16
        i32.shl
        i32.or
        local.get $ptr
        i32.const 8
        i32.add
        i32.load8_u
        i32.const 8
        i32.shl
        i32.or
        local.get $ptr
        i32.const 9
        i32.add
        i32.load8_u
        i32.or
        local.set $low
        i32.const 8
        local.set $extra
        i32.const 10
        local.set $header_len
      end
    end

    local.get $code
    i32.const 126
    i32.eq
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    local.get $code
    i32.const 127
    i32.eq
    local.get $high
    i32.eqz
    local.get $low
    i32.const 65536
    i32.lt_u
    i32.and
    i32.and
    i32.or
    if
      i32.const 3
      return
    end
    local.get $high
    i32.eqz
    i32.eqz
    local.get $low
    local.get $max_len
    i32.gt_u
    i32.or
    if
      i32.const 4
      return
    end
    local.get $masked
    if
      local.get $len
      local.get $header_len
      i32.const 4
      i32.add
      i32.lt_u
      if
        i32.const 1
        return
      end
    end

    local.get $out
    local.get $b0
    i32.const 128
    i32.and
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $b0
    i32.const 112
    i32.and
    i32.const 4
    i32.shr_u
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $b0
    i32.const 15
    i32.and
    i32.store
    local.get $out
    i32.const 12
    i32.add
    local.get $masked
    i32.store
    local.get $out
    i32.const 16
    i32.add
    local.get $low
    i32.store
    local.get $out
    i32.const 20
    i32.add
    local.get $high
    i32.store
    local.get $out
    i32.const 24
    i32.add
    local.get $header_len
    local.get $masked
    if (result i32) i32.const 4 else i32.const 0 end
    i32.add
    i32.store
    local.get $out
    i32.const 28
    i32.add
    local.get $masked
    if (result i32)
      local.get $ptr
      local.get $header_len
      i32.add
      i32.load8_u
      i32.const 24
      i32.shl
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 1
      i32.add
      i32.load8_u
      i32.const 16
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 2
      i32.add
      i32.load8_u
      i32.const 8
      i32.shl
      i32.or
      local.get $ptr
      local.get $header_len
      i32.add
      i32.const 3
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    i32.const 0)

  (func $ws_write_hdr (export "ws_write_frame_header") (param $opcode i32) (param $flags i32) (param $low i32) (param $high i32) (param $mask_present i32) (param $mask i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    call $is_valid_opcode
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    if
      local.get $flags
      i32.const 128
      i32.and
      i32.eqz
      local.get $high
      i32.const 0
      i32.ne
      i32.or
      local.get $low
      i32.const 125
      i32.gt_u
      i32.or
      if
        i32.const 3
        i32.const 0
        call $pack
        return
      end
    end
    local.get $high
    i32.const 2147483648
    i32.ge_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
      else
        i32.const 10
        local.set $written
      end
    end
    local.get $mask_present
    if
      local.get $written
      i32.const 4
      i32.add
      local.set $written
    end
    local.get $cap
    local.get $written
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $flags
    i32.const 240
    i32.and
    local.get $opcode
    i32.const 15
    i32.and
    i32.or
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $mask_present
    if (result i32) i32.const 128 else i32.const 0 end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 2
    i32.eq
    if (result i32)
      local.get $low
    else
      local.get $written
      local.get $mask_present
      if (result i32) i32.const 4 else i32.const 0 end
      i32.sub
      i32.const 4
      i32.eq
      if (result i32) i32.const 126 else i32.const 127 end
    end
    i32.or
    i32.store8
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 4
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $low
      i32.store8
    end
    local.get $written
    local.get $mask_present
    if (result i32) i32.const 4 else i32.const 0 end
    i32.sub
    i32.const 10
    i32.eq
    if
      local.get $out
      i32.const 2
      i32.add
      local.get $high
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 3
      i32.add
      local.get $high
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 4
      i32.add
      local.get $high
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 5
      i32.add
      local.get $high
      i32.store8
      local.get $out
      i32.const 6
      i32.add
      local.get $low
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 7
      i32.add
      local.get $low
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 8
      i32.add
      local.get $low
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      i32.const 9
      i32.add
      local.get $low
      i32.store8
    end
    local.get $mask_present
    if
      local.get $out
      local.get $written
      i32.const 4
      i32.sub
      i32.add
      local.get $mask
      i32.const 24
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 3
      i32.sub
      i32.add
      local.get $mask
      i32.const 16
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 2
      i32.sub
      i32.add
      local.get $mask
      i32.const 8
      i32.shr_u
      i32.store8
      local.get $out
      local.get $written
      i32.const 1
      i32.sub
      i32.add
      local.get $mask
      i32.store8
    end
    i32.const 0
    local.get $written
    call $pack)

  (func (export "ws_parse_close_payload") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    local.get $len
    i32.const 125
    i32.gt_u
    if
      i32.const 3
      return
    end
    local.get $out
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) i32.const 1 else i32.const 0 end
    i32.store
    local.get $out
    i32.const 4
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32)
      local.get $ptr
      i32.load8_u
      i32.const 8
      i32.shl
      local.get $ptr
      i32.const 1
      i32.add
      i32.load8_u
      i32.or
    else
      i32.const 0
    end
    i32.store
    local.get $out
    i32.const 8
    i32.add
    local.get $len
    i32.const 2
    i32.ge_u
    if (result i32) local.get $len i32.const 2 i32.sub else i32.const 0 end
    i32.store
    i32.const 0)

  (func (export "ws_write_close_payload") (param $code i32) (param $reason_ptr i32) (param $reason_len i32) (param $out i32) (param $cap i32) (result i64)
    (local $i i32)
    local.get $code
    i32.const 65535
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $reason_len
    i32.const 123
    i32.gt_u
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $cap
    local.get $reason_len
    i32.const 2
    i32.add
    i32.lt_u
    if
      i32.const 2
      i32.const 0
      call $pack
      return
    end
    local.get $out
    local.get $code
    i32.const 8
    i32.shr_u
    i32.store8
    local.get $out
    i32.const 1
    i32.add
    local.get $code
    i32.store8
    loop $copy
      local.get $i
      local.get $reason_len
      i32.ge_u
      if
        i32.const 0
        local.get $reason_len
        i32.const 2
        i32.add
        call $pack
        return
      end
      local.get $out
      i32.const 2
      i32.add
      local.get $i
      i32.add
      local.get $reason_ptr
      local.get $i
      i32.add
      i32.load8_u
      i32.store8
      local.get $i
      i32.const 1
      i32.add
      local.set $i
      br $copy
    end
    i32.const 0
    i32.const 0
    call $pack)

  (func $ws_write_srv_hdr (export "ws_write_server_frame_header") (param $opcode i32) (param $low i32) (param $high i32) (param $out i32) (param $cap i32) (result i64)
    (local $written i32)
    local.get $opcode
    call $is_valid_opcode
    i32.eqz
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.eqz
    local.get $low
    i32.const 125
    i32.gt_u
    i32.and
    i32.and
    local.get $opcode
    i32.const 8
    i32.ge_u
    local.get $high
    i32.const 0
    i32.ne
    i32.and
    i32.or
    if
      i32.const 3
      i32.const 0
      call $pack
      return
    end
    local.get $high
    i32.eqz
    local.get $low
    i32.const 126
    i32.lt_u
    i32.and
    if
      i32.const 2
      local.set $written
      local.get $cap
      local.get $written
      i32.lt_u
      if
        i32.const 2
        i32.const 0
        call $pack
        return
      end
      local.get $out
      local.get $opcode
      i32.const 128
      i32.or
      i32.store8
      local.get $out
      i32.const 1
      i32.add
      local.get $low
      i32.store8
    else
      local.get $high
      i32.eqz
      local.get $low
      i32.const 65536
      i32.lt_u
      i32.and
      if
        i32.const 4
        local.set $written
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 126
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $low
        i32.store8
      else
        i32.const 10
        local.set $written
        local.get $high
        i32.const 2147483648
        i32.ge_u
        if
          i32.const 3
          i32.const 0
          call $pack
          return
        end
        local.get $cap
        local.get $written
        i32.lt_u
        if
          i32.const 2
          i32.const 0
          call $pack
          return
        end
        local.get $out
        local.get $opcode
        i32.const 128
        i32.or
        i32.store8
        local.get $out
        i32.const 1
        i32.add
        i32.const 127
        i32.store8
        local.get $out
        i32.const 2
        i32.add
        local.get $high
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 3
        i32.add
        local.get $high
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 4
        i32.add
        local.get $high
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 5
        i32.add
        local.get $high
        i32.store8
        local.get $out
        i32.const 6
        i32.add
        local.get $low
        i32.const 24
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 7
        i32.add
        local.get $low
        i32.const 16
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 8
        i32.add
        local.get $low
        i32.const 8
        i32.shr_u
        i32.store8
        local.get $out
        i32.const 9
        i32.add
        local.get $low
        i32.store8
      end
    end
    i32.const 0
    local.get $written
    call $pack)

;; Standard ID removed — merged into single module

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_encode — payload → WS frame (pure transform, batch)
  ;;
  ;; Config (optional, 4 bytes): [opcode:i32]
  ;;   +0: opcode (1=text, 2=binary, default=2). 0 defaults to binary.
  ;;
  ;; Reads payload from input pipe, wraps in WS server frame (FIN, no mask),
  ;; writes complete WS frame to output pipe.
  ;;
  ;; Signature: (input, output, cfg, clen, scratch, scap, state) → bytes_written
  ;; Batch stage: state is ignored (pass 0).
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $opcode i32) (local $n i32) (local $result i64)
    (local $status i32) (local $hdr_len i32)

    (local.set $opcode (i32.const 2))
    (if (i32.ge_u (local.get $clen) (i32.const 4))
      (then
        (local.set $opcode (i32.load (local.get $cfg)))
        (if (i32.eqz (local.get $opcode))
          (then (local.set $opcode (i32.const 2))))))

    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    ;; Read payload into scratch+16 (leave 16 bytes for max header)
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Write WS server frame header to scratch[0..9] (max 10 bytes, no mask)
    (local.set $result (call $ws_write_srv_hdr
      (local.get $opcode) (local.get $n) (i32.const 0)
      (local.get $scratch) (i32.const 10)))
    (local.set $status (i32.wrap_i64 (local.get $result)))
    (if (i32.ne (local.get $status) (i32.const 0))
      (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $hdr_len (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))

    ;; Write header to output
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $hdr_len)))
    ;; Write payload to output
    (drop (call $pipe_write (local.get $output)
      (i32.add (local.get $scratch) (i32.const 16)) (local.get $n)))
    (i32.add (local.get $hdr_len) (local.get $n)))

  ;; ════════════════════════════════════════════════════════════════
  ;; ws_decode — WS frame → payload (pure transform, streaming)
  ;;
  ;; Config: none (pass 0, 0).
  ;;
  ;; State layout (148 bytes):
  ;;   +0:   tick      i32 (RO)
  ;;   +4:   (reserved)
  ;;   +8:   dbuf_len  i32
  ;;   +12:  dbuf[136] decode buffer (partial frame data)
  ;;
  ;; Thin wrapper around $decode_frame_socket with dbuf_off=8 and no send_pipe.
  ;; ════════════════════════════════════════════════════════════════

  (func (export "ws_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $decode_frame_socket
      (local.get $input) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 8) (local.get $output) (i32.const 0)))

  (func $process_ws_encode (export "process_ws_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $ws_encode
      (local.get $input) (local.get $output) (local.get $cfg) (local.get $clen)
      (local.get $scratch) (local.get $scap) (local.get $state)))

  (func $process_ws_decode (export "process_ws_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (call $ws_decode
      (local.get $input) (local.get $output) (local.get $cfg) (local.get $clen)
      (local.get $scratch) (local.get $scap) (local.get $state)))

  ;; ════════════════════════════════════════════════════════════════
  ;; process_ws_frame — bidirectional WS framing (socket mode)
  ;; Config: pointer to socket handle (4 bytes)
  ;; State layout (148 bytes):
  ;;   +0:   tick          i32 (RO)
  ;;   +4:   phase         i32 (0=idle, 1=awaiting)
  ;;   +8:   start_tick    i32
  ;;   +12:  timeout_ticks i32
  ;;   +16:  dbuf_len      i32
  ;;   +20:  dbuf[128]     decode buffer
  ;;
  ;; Reads payload from input, wraps in WS frame, writes to socket send pipe.
  ;; Reads raw bytes from socket recv pipe, decodes WS frames, writes payload
  ;; to output pipe. Ping→Pong, Close→Close echo.
  ;; ════════════════════════════════════════════════════════════════

  (func (export "process_ws_frame")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $sock i32) (local $send_pipe i32) (local $recv_pipe i32)
    (local $phase i32) (local $elapsed i32) (local $sent i32)
    (local $n i32) (local $result i64) (local $status i32)

    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $sock (i32.load (local.get $cfg)))
    (local.set $send_pipe (i32.load (local.get $sock)))
    (local.set $recv_pipe (i32.load offset=4 (local.get $sock)))
    (local.set $phase (i32.load offset=4 (local.get $state)))

    ;; Phase 1 (awaiting response)
    (if (i32.eq (local.get $phase) (i32.const 1))
      (then
        (local.set $elapsed
          (i32.sub (i32.load (local.get $state)) (i32.load offset=8 (local.get $state))))
        (if (i32.load offset=12 (local.get $state))
          (then
            (if (i32.ge_u (local.get $elapsed) (i32.load offset=12 (local.get $state)))
              (then (return (global.get $STATUS_TIMEOUT))))))

        ;; Decode from recv pipe
        (local.set $n (call $decode_frame_socket
          (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
          (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
        (if (i32.gt_s (local.get $n) (i32.const 0))
          (then
            (if (i32.ne (local.get $n) (global.get $STATUS_MORE))
              (then
                (i32.store offset=4 (local.get $state) (i32.const 0))
                (return (local.get $n))))))
        (if (i32.lt_s (local.get $n) (i32.const 0))
          (then (return (local.get $n))))
        (return (global.get $STATUS_MORE))))

    ;; Phase 0 (idle): encode + try decode

    ;; Encode: read payload, wrap in WS frame, send
    (if (i32.lt_u (local.get $scap) (i32.const 17))
      (then (return (i32.const -1))))
    (local.set $n (call $pipe_read (local.get $input)
      (i32.add (local.get $scratch) (i32.const 16))
      (i32.sub (local.get $scap) (i32.const 16))))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (local.set $sent (local.get $n))
        (local.set $result (call $ws_write_srv_hdr
          (i32.const 2) (local.get $n) (i32.const 0)
          (local.get $scratch) (i32.const 10)))
        (local.set $status (i32.wrap_i64 (local.get $result)))
        (if (i32.eqz (local.get $status))
          (then
            (local.set $n (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
            (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $n)))
            (drop (call $pipe_write (local.get $send_pipe)
              (i32.add (local.get $scratch) (i32.const 16)) (local.get $sent)))))))

    ;; Decode from recv pipe
    (local.set $n (call $decode_frame_socket
      (local.get $recv_pipe) (local.get $scratch) (local.get $scap)
      (local.get $state) (i32.const 20) (local.get $output) (local.get $send_pipe)))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (if (i32.ne (local.get $n) (global.get $STATUS_MORE))
          (then (return (local.get $n))))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))

    ;; Decode returned MORE
    (if (local.get $sent)
      (then
        (i32.store offset=4 (local.get $state) (i32.const 1))
        (i32.store offset=8 (local.get $state) (i32.load (local.get $state)))))
    (global.get $STATUS_MORE))

  ;; ── $decode_frame_socket — same as ws_decode but reads from recv pipe ──
  ;; and handles control frames (ping→pong, close→close echo).
  ;; dbuf stored at state + dbuf_off (rather than fixed offset +8).
  (func $decode_frame_socket
    (param $recv i32) (param $scratch i32) (param $scap i32)
    (param $state i32) (param $dbuf_off i32)
    (param $output i32) (param $send_pipe i32) (result i32)
    (local $dbuf_len i32) (local $n i32) (local $total i32) (local $r i32)
    (local $hdr_out i32)
    (local $fin i32) (local $opcode i32) (local $masked i32)
    (local $hdr_len i32) (local $payload_len i32) (local $mask_key i32)
    (local $frame_size i32) (local $off i32)
    (local $result i64) (local $status i32) (local $written i32)
    (local $dbuf_cap i32)

    (if (i32.lt_u (local.get $scap) (i32.const 32))
      (then (return (i32.const -1))))
    (local.set $dbuf_cap (i32.sub (i32.const 144) (local.get $dbuf_off)))
    (local.set $dbuf_len (i32.load (i32.add (local.get $state) (local.get $dbuf_off))))

    (if (i32.gt_u (local.get $dbuf_len) (i32.const 0))
      (then
        (call $memcpy (local.get $scratch)
          (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $dbuf_len))))

    (local.set $n (call $pipe_read (local.get $recv)
      (i32.add (local.get $scratch) (local.get $dbuf_len))
      (i32.sub (local.get $scap) (local.get $dbuf_len))))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (local.set $n (i32.const 0))))
    (local.set $total (i32.add (local.get $dbuf_len) (local.get $n)))

    (if (i32.lt_u (local.get $total) (i32.const 2))
      (then
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (local.set $hdr_out (i32.sub (local.get $scap) (i32.const 32)))
    (local.set $r (call $ws_parse_header
      (local.get $scratch) (local.get $total) (local.get $scap) (local.get $hdr_out)))

    (if (i32.eq (local.get $r) (i32.const 1))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (if (i32.eq (local.get $r) (i32.const 3)) (then (return (i32.const -3))))
    (if (i32.eq (local.get $r) (i32.const 4)) (then (return (i32.const -4))))

    (local.set $fin    (i32.load offset=0 (local.get $hdr_out)))
    (local.set $opcode (i32.load offset=8 (local.get $hdr_out)))
    (local.set $masked (i32.load offset=12 (local.get $hdr_out)))
    (local.set $payload_len (i32.load offset=16 (local.get $hdr_out)))
    (local.set $hdr_len (i32.load offset=24 (local.get $hdr_out)))
    (local.set $mask_key (i32.load offset=28 (local.get $hdr_out)))

    (local.set $frame_size (i32.add (local.get $hdr_len) (local.get $payload_len)))
    (if (i32.gt_u (local.get $frame_size) (local.get $total))
      (then
        (if (i32.gt_u (local.get $total) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (local.get $scratch) (local.get $total))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $total))
        (return (global.get $STATUS_MORE))))

    (local.set $off (local.get $hdr_len))

    (if (local.get $masked)
      (then
        (drop (call $ws_apply_mask
          (i32.add (local.get $scratch) (local.get $off))
          (local.get $payload_len) (local.get $mask_key)))))

    (block $ctrl
      (if (i32.and (i32.ge_u (local.get $opcode) (i32.const 1))
                   (i32.le_u (local.get $opcode) (i32.const 2)))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 0))
        (then
          (drop (call $pipe_write (local.get $output)
            (i32.add (local.get $scratch) (local.get $off))
            (local.get $payload_len)))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 9))
        (then
          (if (local.get $send_pipe)
            (then
              (local.set $result (call $ws_write_hdr
                (i32.const 10) (i32.const 128)
                (local.get $payload_len) (i32.const 0)
                (i32.const 0) (i32.const 0)
                (local.get $scratch) (i32.const 10)))
              (local.set $status (i32.wrap_i64 (local.get $result)))
              (if (i32.eqz (local.get $status))
                (then
                  (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
                  (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
                  (drop (call $pipe_write (local.get $send_pipe)
                    (i32.add (local.get $scratch) (local.get $off))
                    (local.get $payload_len)))))))
          (br $ctrl)))

      (if (i32.eq (local.get $opcode) (i32.const 8))
        (then
          (if (local.get $send_pipe)
            (then
              (local.set $result (call $ws_write_hdr
                (i32.const 8) (i32.const 128)
                (local.get $payload_len) (i32.const 0)
                (i32.const 0) (i32.const 0)
                (local.get $scratch) (i32.const 10)))
              (local.set $status (i32.wrap_i64 (local.get $result)))
              (if (i32.eqz (local.get $status))
                (then
                  (local.set $written (i32.wrap_i64 (i64.shr_u (local.get $result) (i64.const 32))))
                  (drop (call $pipe_write (local.get $send_pipe) (local.get $scratch) (local.get $written)))
                  (drop (call $pipe_write (local.get $send_pipe)
                    (i32.add (local.get $scratch) (local.get $off))
                    (local.get $payload_len)))))))
          (br $ctrl))))

    ;; Save overflow
    (local.set $off (i32.add (local.get $off) (local.get $payload_len)))
    (local.set $n (i32.sub (local.get $total) (local.get $off)))
    (if (i32.gt_u (local.get $n) (i32.const 0))
      (then
        (if (i32.gt_u (local.get $n) (local.get $dbuf_cap))
          (then (return (i32.const -5))))
        (call $memcpy (i32.add (local.get $state) (i32.add (local.get $dbuf_off) (i32.const 4)))
          (i32.add (local.get $scratch) (local.get $off)) (local.get $n))
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (local.get $n)))
      (else
        (i32.store (i32.add (local.get $state) (local.get $dbuf_off)) (i32.const 0))))

    (if (i32.le_u (local.get $opcode) (i32.const 2))
      (then (return (local.get $payload_len))))
    (if (i32.eq (local.get $opcode) (i32.const 9))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $opcode) (i32.const 8))
      (then (return (i32.const 1))))
    (i32.const 1))

  ;; memcpy imported from edgerun-core as $memcpy(dst, src, len)

  (data (i32.const 62024) "258EAFA5-E914-47DA-95CA-C5AB0DC85B11")
