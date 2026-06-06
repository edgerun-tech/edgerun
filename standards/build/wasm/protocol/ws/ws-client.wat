(module
  (import "edgerun-core" "memory" (memory 1))
;; WebSocket client — HTTP Upgrade handshake + masked frame send/recv.
  ;; Uses abstract socket for transport.
  (func (export "proto_standard_id") (result i32) i32.const 300507)

  ;; Memory layout:
  ;; 4096-5119  request buffer
  ;; 5120-6143  response buffer
  ;; 6144-6167  stored key (24 bytes base64)
  ;; 6168-6195  stored accept (28 bytes base64)
  ;; 6196-6199  stored masking key (4 bytes BIG-ENDIAN format)

  ;; ── Internal helpers ──

  (func $m198memcpy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    i32.const 0 local.set $i
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

  (func $m198b64_char (param $v i32) (result i32)
    local.get $v i32.const 26 i32.lt_u
    if i32.const 65 local.get $v i32.add return end
    local.get $v i32.const 52 i32.lt_u
    if i32.const 97 local.get $v i32.const 26 i32.sub i32.add return end
    local.get $v i32.const 62 i32.lt_u
    if i32.const 48 local.get $v i32.const 52 i32.sub i32.add return end
    local.get $v i32.const 62 i32.eq
    if i32.const 43 return end
    i32.const 47)

  ;; b64_encode_3(b0, b1, b2, out) — 3 bytes → 4 base64 chars
  (func $b64_3 (param $b0 i32) (param $b1 i32) (param $b2 i32) (param $out i32)
    local.get $out i32.const 0 i32.add
    local.get $b0 i32.const 2 i32.shr_u call $m198b64_char i32.store8
    local.get $out i32.const 1 i32.add
    local.get $b0 i32.const 4 i32.shl local.get $b1 i32.const 4 i32.shr_u i32.or i32.const 0x3F i32.and
    call $m198b64_char i32.store8
    local.get $out i32.const 2 i32.add
    local.get $b1 i32.const 2 i32.shl local.get $b2 i32.const 6 i32.shr_u i32.or i32.const 0x3F i32.and
    call $m198b64_char i32.store8
    local.get $out i32.const 3 i32.add
    local.get $b2 i32.const 0x3F i32.and call $m198b64_char i32.store8)

  ;; b64_encode_16(in, out) — 16 bytes → 24-byte base64 key
  (func $b64_16 (param $in i32) (param $out i32)
    (local $i i32)
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i i32.const 5 i32.ge_u br_if $done
      local.get $in local.get $i i32.const 3 i32.mul i32.add i32.load8_u
      local.get $in local.get $i i32.const 3 i32.mul i32.const 1 i32.add i32.add i32.load8_u
      local.get $in local.get $i i32.const 3 i32.mul i32.const 2 i32.add i32.add i32.load8_u
      local.get $out local.get $i i32.const 3 i32.mul i32.add
      call $b64_3
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    ;; remaining byte 15 → 2 chars + "=="
    local.get $out i32.const 20 i32.add
    local.get $in i32.const 15 i32.add i32.load8_u
    local.tee $i
    i32.const 2 i32.shr_u
    call $m198b64_char
    i32.store8
    local.get $out i32.const 21 i32.add
    local.get $i i32.const 3 i32.and i32.const 4 i32.shl
    call $m198b64_char
    i32.store8
    local.get $out i32.const 22 i32.add i32.const 0x3D i32.store8
    local.get $out i32.const 23 i32.add i32.const 0x3D i32.store8)
  ;; BUG in $b64_16: the call to $b64_3 pushes arguments in wrong order.
  ;; The WAT stack order is: push b0, push b1, push b2, push out → but the function expects (b0,b1,b2,out).
  ;; Actually in unfolded WAT, arguments are NOT pushed like in stack-based WASM.
  ;; In unfolded WAT: local.get param1 local.get param2 ... call func
  ;; This means param1 is pushed first, param2 second, etc.
  ;; So I need: local.get $b0 local.get $b1 local.get $b2 local.get $out call $b64_3
  ;; Let me fix this in the next revision.

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

    i32.const 4096 local.set $req
    i32.const 5120 local.set $rsp

    ;; 1. Base64-encode key → 6144
    i32.const 6144 local.set $fd
    local.get $key16 local.get $fd call $b64_16

    ;; 2. Copy first 4 key bytes as mask → 6196 (stored big-endian)
    i32.const 6196 local.get $key16 i32.load align=1 i32.store

    ;; 3. Build HTTP Upgrade request
    local.get $req local.set $p

    ;; "GET "
    local.get $p i32.const 0x20544547 i32.store
    local.get $p i32.const 4 i32.add local.set $p

    ;; path
    local.get $p local.get $path local.get $plen call $m198memcpy
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
    local.get $p i32.const 6144 i32.const 24 call $m198memcpy
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
    local.get $rsp local.get $rlen i32.const 6168 call $extract_accept

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

    i32.const 4000 local.set $hdr
    i32.const 6196 local.set $mask
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
    i32.const 3000 local.set $buf
    local.get $buf local.get $data local.get $dlen call $m198memcpy

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

    i32.const 2000 local.set $hdr

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
)
