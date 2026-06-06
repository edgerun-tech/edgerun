(module
  (import "edgerun-core" "memory" (memory 1))
;; Abstract socket layer — transport-agnostic byte stream I/O.
  ;;
  ;; Socket types define the transport. Config structs are fixed-size
  ;; records stored in linear memory, passed by pointer to the host's
  ;; sock_open implementation.
  ;;
  ;; Socket type  Config struct (ptr+len)
  ;; ───────────  ─────────────────────────────────
  ;; TCP=0        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
  ;; TLS=1        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
  ;; CDP=2        ws_url_ptr[4] ws_url_len[4]              (8 bytes)
  ;; SOCKS5=3     proxy_host_ptr[4] proxy_host_len[4]
  ;;              proxy_port[2] pad[2]
  ;;              target_host_ptr[4] target_host_len[4]
  ;;              target_port[2] pad[2]                    (24 bytes)
  ;; HTTP_PROXY=4 proxy_host_ptr[4] proxy_host_len[4]
  ;;              proxy_port[2] pad[2]
  ;;              target_host_ptr[4] target_host_len[4]
  ;;              target_port[2] pad[2]                    (24 bytes)
  ;; UDP=5        host_ptr[4] host_len[4] port[2] pad[2]   (12 bytes)
  (func (export "proto_standard_id") (result i32) i32.const 300505)

  ;; ── Socket type constants ──
  (func (export "SOCK_TCP")        (result i32) i32.const 0)
  (func (export "SOCK_TLS")        (result i32) i32.const 1)
  (func (export "SOCK_CDP")        (result i32) i32.const 2)
  (func (export "SOCK_SOCKS5")     (result i32) i32.const 3)
  (func (export "SOCK_HTTP_PROXY") (result i32) i32.const 4)
  (func (export "SOCK_UDP")        (result i32) i32.const 5)

  ;; ── Config struct sizes ──
  (func (export "SOCK_CFG_TCP_SIZE")        (result i32) i32.const 12)
  (func (export "SOCK_CFG_TLS_SIZE")        (result i32) i32.const 12)
  (func (export "SOCK_CFG_CDP_SIZE")        (result i32) i32.const 8)
  (func (export "SOCK_CFG_SOCKS5_SIZE")     (result i32) i32.const 24)
  (func (export "SOCK_CFG_HTTP_PROXY_SIZE") (result i32) i32.const 24)
  (func (export "SOCK_CFG_UDP_SIZE")        (result i32) i32.const 12)

  ;; ── SOCKS5 handshake builders ──

  ;; socks5_build_greeting(out_ptr, out_cap) -> i64
  ;; Builds SOCKS5 greeting: [0x05, 0x01, 0x00] (no auth).
  ;; Returns packed (status, length)
  (func (export "socks5_build_greeting")
    (param $out i32) (param $ocap i32) (result i64)
    local.get $ocap i32.const 3 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add i32.const 5 i32.store8
    local.get $out i32.const 1 i32.add i32.const 1 i32.store8
    local.get $out i32.const 2 i32.add i32.const 0 i32.store8
    i64.const 0 i32.const 3 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; socks5_parse_greeting_response(data_ptr, data_len) -> i32
  ;; Returns 0 if server accepted no-auth, -1 on error.
  (func (export "socks5_parse_greeting_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 2 i32.lt_u
    if i32.const -1 return end
    local.get $data i32.load8_u offset=0 i32.const 5 i32.ne
    if i32.const -1 return end
    local.get $data i32.load8_u offset=1 i32.const 0 i32.ne
    if i32.const -1 return end
    i32.const 0)

  ;; socks5_build_connect(out_ptr, out_cap, host_ptr, host_len, port) -> i64
  ;; Builds SOCKS5 connect request with domain name.
  ;; Returns packed (status, length)
  (func (export "socks5_build_connect")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    (local $need i32) (local $i i32)

    i32.const 7  ;; ver+cmd+rsv+atype+len = 5 bytes, +hlen + 2 port
    local.get $hlen
    i32.add
    local.set $need

    local.get $ocap local.get $need i32.lt_u
    if i64.const -2 return end

    local.get $out i32.const 0 i32.add i32.const 5 i32.store8     ;; ver = 5
    local.get $out i32.const 1 i32.add i32.const 1 i32.store8     ;; cmd = CONNECT
    local.get $out i32.const 2 i32.add i32.const 0 i32.store8     ;; rsv = 0
    local.get $out i32.const 3 i32.add i32.const 3 i32.store8     ;; atyp = DOMAINNAME
    local.get $out i32.const 4 i32.add local.get $hlen i32.store8 ;; domain length

    ;; copy host name
    i32.const 0 local.set $i
    block $cpy_done
    loop $cpy
      local.get $i local.get $hlen i32.ge_u br_if $cpy_done
      local.get $out i32.const 5 i32.add local.get $i i32.add
      local.get $host local.get $i i32.add i32.load8_u
      i32.store8
      local.get $i i32.const 1 i32.add local.set $i
      br $cpy
    end
    end

    ;; port (big-endian)
    local.get $out i32.const 5 i32.add local.get $hlen i32.add
    local.get $port i32.const 8 i32.shr_u i32.const 255 i32.and
    i32.store8
    local.get $out i32.const 6 i32.add local.get $hlen i32.add
    local.get $port i32.const 255 i32.and
    i32.store8

    i64.const 0
    local.get $need
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; socks5_parse_connect_response(data_ptr, data_len) -> i32
  ;; Returns 0 if connect succeeded, -1 on error.
  ;; On success, the proxy has connected to the target.
  (func (export "socks5_parse_connect_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 10 i32.lt_u
    if i32.const -1 return end
    local.get $data i32.load8_u offset=0 i32.const 5 i32.ne
    if i32.const -1 return end
    local.get $data i32.load8_u offset=1 i32.const 0 i32.ne  ;; rep must be 0 (succeeded)
    if i32.const -1 return end
    i32.const 0)

  ;; ── HTTP CONNECT proxy handshake builders ──

  ;; http_proxy_build_request(out_ptr, out_cap, host_ptr, host_len, port) -> i64
  ;; Builds "CONNECT host:port HTTP/1.1\r\nHost: host:port\r\n\r\n"
  ;; Returns packed (status, length)
  (func (export "http_proxy_build_request")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    (local $o i32)

    ;; "CONNECT "
    local.get $o i32.const 7 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 67 i32.store8   ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 79 i32.store8   ;; 'O'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 78 i32.store8   ;; 'N'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 78 i32.store8   ;; 'N'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 69 i32.store8   ;; 'E'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 67 i32.store8   ;; 'C'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8   ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8   ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; host
    local.get $o local.get $hlen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $host local.get $out local.get $o local.get $hlen call $m166memcpy_to_off
    local.get $o local.get $hlen i32.add local.set $o

    ;; ":port "
    local.get $o i32.const 1 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 58 i32.store8   ;; ':'
    local.get $o i32.const 1 i32.add local.set $o

    local.get $port local.get $out local.get $o call $m166emit_u32_dec
    local.set $o

    local.get $o i32.const 1 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 32 i32.store8   ;; ' '
    local.get $o i32.const 1 i32.add local.set $o

    ;; "HTTP/1.1\r\n"
    local.get $o i32.const 10 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 72 i32.store8   ;; 'H'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8   ;; 'T'
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 84 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 80 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 47 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 46 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 49 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8   ;; CR
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8   ;; LF
    local.get $o i32.const 1 i32.add local.set $o

    ;; "Host: "
    local.get $o i32.const 6 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 72 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 111 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 115 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 116 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 58 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 32 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    ;; host again
    local.get $o local.get $hlen i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $host local.get $out local.get $o local.get $hlen call $m166memcpy_to_off
    local.get $o local.get $hlen i32.add local.set $o

    ;; ":port"
    local.get $out local.get $o i32.add i32.const 58 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    local.get $port local.get $out local.get $o call $m166emit_u32_dec
    local.set $o

    ;; "\r\n\r\n"
    local.get $o i32.const 4 i32.add local.get $ocap i32.gt_u
    if i64.const -2 return end
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 13 i32.store8
    local.get $o i32.const 1 i32.add local.set $o
    local.get $out local.get $o i32.add i32.const 10 i32.store8
    local.get $o i32.const 1 i32.add local.set $o

    i64.const 0
    local.get $o
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)

  ;; http_proxy_parse_response(data_ptr, data_len) -> i32
  ;; Check for "200" in HTTP response line.
  ;; Returns 0 if connected, -1 on error.
  (func (export "http_proxy_parse_response")
    (param $data i32) (param $dlen i32) (result i32)
    local.get $dlen i32.const 12 i32.lt_u
    if i32.const -1 return end
    ;; check for "HTTP/1.1 200" or "HTTP/1.0 200"
    local.get $data i32.load8_u offset=0 i32.const 72 i32.ne  ;; 'H'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=9 i32.const 50 i32.ne  ;; '2'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=10 i32.const 48 i32.ne ;; '0'
    if i32.const -1 return end
    local.get $data i32.load8_u offset=11 i32.const 48 i32.ne ;; '0'
    if i32.const -1 return end
    i32.const 0)

  ;; ── Config struct builders ──

  ;; sock_cfg_build_tcp(out, cap, host_ptr, host_len, port) -> i64
  ;; Writes TCP config struct, returns (0, 12)
  (func (export "sock_cfg_build_tcp")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    local.get $ocap i32.const 12 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $host i32.store
    local.get $out i32.const 4 i32.add local.get $hlen i32.store
    local.get $out i32.const 8 i32.add local.get $port i32.store16
    i64.const 0 i32.const 12 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; sock_cfg_build_tls(out, cap, host_ptr, host_len, port) -> i64
  ;; TLS config is same layout as TCP (host+port+SNI)
  (func (export "sock_cfg_build_tls")
    (param $out i32) (param $ocap i32)
    (param $host i32) (param $hlen i32)
    (param $port i32) (result i64)
    local.get $ocap i32.const 12 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $host i32.store
    local.get $out i32.const 4 i32.add local.get $hlen i32.store
    local.get $out i32.const 8 i32.add local.get $port i32.store16
    i64.const 0 i32.const 12 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; sock_cfg_build_socks5(out, cap,
  ;;                       proxy_host, proxy_hlen, proxy_port,
  ;;                       target_host, target_hlen, target_port) -> i64
  (func (export "sock_cfg_build_socks5")
    (param $out i32) (param $ocap i32)
    (param $phost i32) (param $phlen i32) (param $pport i32)
    (param $thost i32) (param $thlen i32) (param $tport i32) (result i64)
    local.get $ocap i32.const 24 i32.lt_u
    if i64.const -2 return end
    local.get $out i32.const 0 i32.add local.get $phost i32.store
    local.get $out i32.const 4 i32.add local.get $phlen i32.store
    local.get $out i32.const 8 i32.add local.get $pport i32.store16
    local.get $out i32.const 12 i32.add local.get $thost i32.store
    local.get $out i32.const 16 i32.add local.get $thlen i32.store
    local.get $out i32.const 20 i32.add local.get $tport i32.store16
    i64.const 0 i32.const 24 i64.extend_i32_u i64.const 32 i64.shl i64.or)

  ;; ── helpers ──

  ;; memcpy_to_off(src, dst, dst_off, len)
  (func $m166memcpy_to_off (param $src i32) (param $dst i32) (param $off i32) (param $len i32)
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
  (func $m166emit_u32_dec (param $n i32) (param $out i32) (param $off i32) (result i32)
    (local $buf i32) (local $digits i32) (local $d i32)
    i32.const 768 local.set $buf
    i32.const 0 local.set $digits
    local.get $n i32.eqz
    if
      local.get $out local.get $off i32.add i32.const 48 i32.store8
      local.get $off i32.const 1 i32.add
      return
    end
    block $extract_done
    loop $extract
      local.get $n i32.eqz br_if $extract_done
      local.get $buf local.get $digits i32.add
      local.get $n i32.const 10 i32.rem_u i32.const 48 i32.add
      i32.store8
      local.get $digits i32.const 1 i32.add local.set $digits
      local.get $n i32.const 10 i32.div_u local.set $n
      br $extract
    end
    end
    block $emit_done
    loop $emit_loop
      local.get $d local.get $digits i32.ge_u br_if $emit_done
      local.get $out local.get $off i32.add local.get $d i32.add
      local.get $buf local.get $digits i32.const 1 i32.sub local.get $d i32.sub i32.add
      i32.load8_u
      i32.store8
      local.get $d i32.const 1 i32.add local.set $d
      br $emit_loop
    end
    end
    local.get $off local.get $digits i32.add)
)
