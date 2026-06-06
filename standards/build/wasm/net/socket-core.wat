(module
  (import "edgerun-core" "memory" (memory 1))
  (import "pipe-core" "pipe_alloc" (func $pipe_alloc (param i32) (result i32)))
  (import "pipe-core" "pipe_create" (func $pipe_create (param i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_available" (func $pipe_available (param i32) (result i32)))
  (import "pipe-core" "pipe_close" (func $pipe_close (param i32)))
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

  ;; ── Socket abstraction (pipe-based I/O) ──
  ;;
  ;; Socket struct (24 bytes):
  ;;   +0:  send_pipe  — pipe handle for outgoing data
  ;;   +4:  recv_pipe  — pipe handle for incoming data
  ;;   +8:  state      — 0=closed 1=open 2=connecting 3=connected
  ;;   +12: sock_type  — TCP=0 TLS=1 ...
  ;;   +16: cfg_ptr    — pointer to stored config
  ;;   +20: cfg_len    — config length

  (func (export "SOCK_OPEN")       (result i32) i32.const 1)
  (func (export "SOCK_CONNECTING") (result i32) i32.const 2)
  (func (export "SOCK_CONNECTED")  (result i32) i32.const 3)
  (func (export "SOCK_STRUCT_SIZE") (result i32) i32.const 24)

  ;; sock_open(type, cfg_ptr, cfg_len, pipe_cap) → socket_handle | -1
  ;; Allocates a socket struct + two pipes (send + recv).
  (func (export "sock_open")
    (param $type i32) (param $cfg i32) (param $clen i32) (param $pcap i32) (result i32)
    (local $s i32) (local $snd i32) (local $rcv i32)
    (local.set $s (call $pipe_alloc (i32.const 24)))
    (if (i32.eq (local.get $s) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $snd (call $pipe_create (local.get $pcap)))
    (if (i32.eq (local.get $snd) (i32.const -1)) (then (return (i32.const -1))))
    (local.set $rcv (call $pipe_create (local.get $pcap)))
    (if (i32.eq (local.get $rcv) (i32.const -1)) (then (return (i32.const -1))))
    (i32.store offset=0 (local.get $s) (local.get $snd))
    (i32.store offset=4 (local.get $s) (local.get $rcv))
    (i32.store offset=8 (local.get $s) (i32.const 1))  ;; state = open
    (i32.store offset=12 (local.get $s) (local.get $type))
    (i32.store offset=16 (local.get $s) (local.get $cfg))
    (i32.store offset=20 (local.get $s) (local.get $clen))
    local.get $s)

  ;; sock_send(socket, data, len) → status
  ;; Writes data to the socket's send pipe.
  (func (export "sock_send")
    (param $s i32) (param $data i32) (param $len i32) (result i32)
    (call $pipe_write (i32.load offset=0 (local.get $s)) (local.get $data) (local.get $len)))

  ;; sock_recv(socket, dst, max) → bytes_read
  ;; Reads from the socket's recv pipe.
  (func (export "sock_recv")
    (param $s i32) (param $dst i32) (param $max i32) (result i32)
    (call $pipe_read (i32.load offset=4 (local.get $s)) (local.get $dst) (local.get $max)))

  ;; sock_close(socket) — closes both pipes, marks socket closed
  (func (export "sock_close") (param $s i32)
    (call $pipe_close (i32.load offset=0 (local.get $s)))
    (call $pipe_close (i32.load offset=4 (local.get $s)))
    (i32.store offset=8 (local.get $s) (i32.const 0)))

  ;; sock_get_state(socket) → state
  (func (export "sock_get_state") (param $s i32) (result i32)
    (i32.load offset=8 (local.get $s)))

  ;; sock_get_send_pipe(socket) → pipe_handle
  (func (export "sock_get_send_pipe") (param $s i32) (result i32)
    (i32.load offset=0 (local.get $s)))

  ;; sock_get_recv_pipe(socket) → pipe_handle
  (func (export "sock_get_recv_pipe") (param $s i32) (result i32)
    (i32.load offset=4 (local.get $s)))

  ;; sock_pipe(from, to, tmp, tcap) → bytes_piped | error
  ;; Pipes data from from_sock's recv pipe into to_sock's send pipe.
  ;; Uses tmp buffer of tcap bytes as scratch space.
  (func $sock_pipe (export "sock_pipe")
    (param $from i32) (param $to i32) (param $tmp i32) (param $tcap i32) (result i32)
    (local $n i32)
    (local.set $n (call $pipe_read
      (i32.load offset=4 (local.get $from)) (local.get $tmp) (local.get $tcap)))
    (if (i32.le_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (call $pipe_write
      (i32.load offset=0 (local.get $to)) (local.get $tmp) (local.get $n))
    (return (local.get $n)))

  ;; ── Pipeline stage: transport ──
  ;; Reads from input pipe, sends via socket, reads response, writes to output pipe.
  ;; Config: pointer to a 4-byte i32 socket handle.
  ;; (input_pipe, output_pipe, config_ptr, config_len, scratch, scap) → bytes_written | error
  (func (export "process_transport")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (result i32)
    (local $sock i32) (local $n i32)
    (if (i32.lt_u (local.get $clen) (i32.const 4))
      (then (return (i32.const -1))))
    (local.set $sock (i32.load (local.get $cfg)))
    ;; Read data to send from input pipe
    (local.set $n (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (if (i32.gt_s (local.get $n) (i32.const 0))
      (then
        (drop (call $pipe_write (i32.load offset=0 (local.get $sock)) (local.get $scratch) (local.get $n)))))
    ;; Read response from socket recv pipe
    (local.set $n (call $pipe_read (i32.load offset=4 (local.get $sock)) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_s (local.get $n) (i32.const 0))
      (then (return (local.get $n))))
    (if (i32.eqz (local.get $n)) (then (return (i32.const 0))))
    ;; Write response to output pipe
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $n)))
    local.get $n)
)
