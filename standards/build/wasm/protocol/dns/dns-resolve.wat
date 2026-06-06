;; DNS A-record resolver — self-contained, uses abstract socket for UDP.
  (func (export "proto_abi_version") (result i32) i32.const 2)
  (func (export "proto_standard_id") (result i32) i32.const 300506)

  ;; dns_resolve_a(host_ptr, host_len,
  ;;               dns_host_ptr, dns_host_len, dns_port,
  ;;               out_ip_ptr, out_ip_cap) -> i64
  ;;
  ;; Builds DNS A-record query, sends via UDP abstract socket,
  ;; parses response for A records. out_ip stores 4-byte IPs.
  ;; Returns packed (status, ip_count).
  (func (export "dns_resolve_a")
    (param $host i32) (param $hlen i32)
    (param $dns_host i32) (param $dns_hlen i32) (param $dns_port i32)
    (param $out i32) (param $ocap i32)
    (result i64)
    (local $qbuf i32) (local $rbuf i32) (local $qlen i32)
    (local $fd i32) (local $rc i32) (local $rlen i32)
    (local $i i32) (local $j i32) (local $b i32)
    (local $llen i32) (local $ip_count i32)
    (local $ans_count i32)

    i32.const 4096 local.set $qbuf
    i32.const 5120 local.set $rbuf

    ;; ── 1. Build DNS header (12 bytes) ──
    local.get $qbuf i32.const 0 i32.add i32.const 0x1234 i32.store16  ;; ID
    local.get $qbuf i32.const 2 i32.add i32.const 0x0100 i32.store16  ;; flags: RD=1
    local.get $qbuf i32.const 4 i32.add i32.const 1 i32.store16       ;; qdcount=1
    local.get $qbuf i32.const 6 i32.add i32.const 0 i32.store16       ;; ancount=0
    local.get $qbuf i32.const 8 i32.add i32.const 0 i32.store16       ;; nscount=0
    local.get $qbuf i32.const 10 i32.add i32.const 0 i32.store16      ;; arcount=0
    i32.const 12 local.set $qlen

    ;; ── 2. Encode hostname as DNS name ──
    i32.const 0 local.set $i
    block $encode_done
    loop $encode
      local.get $i local.get $hlen i32.ge_u
      if
        local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
        local.get $qlen i32.const 1 i32.add local.set $qlen
        br $encode_done
      end
      i32.const 0 local.set $llen
      block $llen_done
      loop $llen_loop
        local.get $i local.get $llen i32.add local.get $hlen i32.ge_u
        br_if $llen_done
        local.get $host local.get $i local.get $llen i32.add i32.add i32.load8_u
        i32.const 46 i32.eq
        br_if $llen_done
        local.get $llen i32.const 1 i32.add local.set $llen
        br $llen_loop
      end
      end
      local.get $qbuf local.get $qlen i32.add local.get $llen i32.store8
      local.get $qlen i32.const 1 i32.add local.set $qlen
      i32.const 0 local.set $j
      block $lbl_done
      loop $lbl_loop
        local.get $j local.get $llen i32.ge_u br_if $lbl_done
        local.get $qbuf local.get $qlen i32.add
        local.get $host local.get $i local.get $j i32.add i32.add i32.load8_u
        i32.store8
        local.get $qlen i32.const 1 i32.add local.set $qlen
        local.get $j i32.const 1 i32.add local.set $j
        br $lbl_loop
      end
      end
      local.get $i local.get $llen i32.add i32.const 1 i32.add local.set $i
      br $encode
    end
    end

    ;; ── 3. Append QTYPE=A(1), QCLASS=IN(1) ──
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
    local.get $qbuf local.get $qlen i32.const 1 i32.add i32.add i32.const 1 i32.store8
    local.get $qlen i32.const 2 i32.add local.set $qlen
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.store8
    local.get $qbuf local.get $qlen i32.const 1 i32.add i32.add i32.const 1 i32.store8
    local.get $qlen i32.const 2 i32.add local.set $qlen

    ;; ── 4. Build UDP config right after query ──
    local.get $qbuf local.get $qlen i32.add i32.const 0 i32.add local.get $dns_host i32.store
    local.get $qbuf local.get $qlen i32.add i32.const 4 i32.add local.get $dns_hlen i32.store
    local.get $qbuf local.get $qlen i32.add i32.const 8 i32.add local.get $dns_port i32.store16

    ;; ── 5. sock_open(SOCK_UDP=5, cfg, 12) ──
    i32.const 5 local.get $qbuf local.get $qlen i32.add i32.const 12 call $sock_open
    local.tee $fd
    i32.const 0 i32.lt_s
    if i64.const -2 return end

    ;; ── 6. sock_send(fd, qbuf, qlen) ──
    local.get $fd local.get $qbuf local.get $qlen call $sock_send
    i32.const 0 i32.lt_s
    if local.get $fd call $sock_close drop i64.const -3 return end

    ;; ── 7. sock_recv into rbuf ──
    local.get $fd local.get $rbuf i32.const 512 call $sock_recv
    local.tee $rc
    i32.const 0 i32.le_s
    if local.get $fd call $sock_close drop i64.const -4 return end
    local.get $rc local.set $rlen
    local.get $fd call $sock_close drop

    ;; ── 8. Parse response header ──
    local.get $rlen i32.const 12 i32.lt_u
    if i64.const -5 return end
    local.get $rbuf i32.load16_u i32.const 0x1234 i32.ne
    if i64.const -5 return end
    local.get $rbuf i32.load8_u offset=2 i32.const 0x80 i32.and i32.eqz
    if i64.const -5 return end
    local.get $rbuf i32.load8_u offset=3 i32.const 15 i32.and i32.const 0 i32.ne
    if i64.const -5 return end
    local.get $rbuf i32.load16_u offset=6
    local.tee $ans_count
    i32.eqz
    if i64.const -6 return end

    ;; ── 9. Skip question section ──
    local.get $rbuf i32.load16_u offset=4
    local.set $rc
    i32.const 12 local.set $i
    block $q_done
    loop $q_loop
      local.get $rc i32.eqz br_if $q_done
      local.get $i local.get $rlen i32.ge_u
      if i64.const -5 return end
      local.get $rbuf local.get $i i32.add i32.load8_u
      i32.const 0xc0 i32.and i32.const 0xc0 i32.eq
      if
        local.get $i i32.const 2 i32.add local.set $i
      else
        block $qname_break
        loop $qname_loop
          local.get $i local.get $rlen i32.ge_u
          if i64.const -5 return end
          local.get $rbuf local.get $i i32.add i32.load8_u
          local.tee $b
          i32.eqz
          if
            local.get $i i32.const 1 i32.add local.set $i
            br $qname_break
          end
          local.get $i local.get $b i32.add i32.const 1 i32.add local.set $i
          br $qname_loop
        end
        end
      end
      local.get $i i32.const 4 i32.add local.set $i
      local.get $rc i32.const 1 i32.sub local.set $rc
      br $q_loop
    end
    end

    ;; ── 10. Parse answer section — extract A records ──
    i32.const 0 local.set $ip_count
    block $ans_done
    loop $ans_loop
      local.get $ans_count i32.eqz br_if $ans_done
      local.get $i i32.const 12 i32.add local.get $rlen i32.gt_u
      br_if $ans_done

      ;; skip RR name (compressed or inline)
      local.get $rbuf local.get $i i32.add i32.load8_u
      i32.const 0xc0 i32.and i32.const 0xc0 i32.eq
      if
        local.get $i i32.const 2 i32.add local.set $i
      else
        block $rr_name_break
        loop $rr_name_loop
          local.get $i local.get $rlen i32.ge_u br_if $ans_done
          local.get $rbuf local.get $i i32.add i32.load8_u
          local.tee $b
          i32.eqz
          if
            local.get $i i32.const 1 i32.add local.set $i
            br $rr_name_break
          end
          local.get $i local.get $b i32.add i32.const 1 i32.add local.set $i
          br $rr_name_loop
        end
        end
      end

      ;; read TYPE(2) + CLASS(2) + TTL(4) + RDLENGTH(2) starting at $i
      local.get $i i32.const 10 i32.add local.get $rlen i32.gt_u
      br_if $ans_done
      local.get $rbuf local.get $i i32.add i32.load16_u
      i32.const 1 i32.ne
      if
        ;; not A record — skip by rdlength
        local.get $rbuf local.get $i i32.const 8 i32.add i32.add i32.load16_u
        local.set $b
        local.get $i i32.const 10 i32.add local.get $b i32.add local.set $i
        local.get $ans_count i32.const 1 i32.sub local.set $ans_count
        br $ans_loop
      end

      ;; TYPE is A — skip CLASS(2) and TTL(4) to rdlength at $i+6
      local.get $rbuf local.get $i i32.const 6 i32.add i32.add i32.load16_u
      local.tee $b
      i32.const 4 i32.ne
      if
        local.get $i i32.const 8 i32.add local.get $b i32.add local.set $i
        local.get $ans_count i32.const 1 i32.sub local.set $ans_count
        br $ans_loop
      end

      ;; rdlength=4 — read IP at $i+8
      local.get $i i32.const 8 i32.add local.get $rlen i32.gt_u
      br_if $ans_done

      local.get $ip_count local.get $ocap i32.ge_u
      br_if $ans_done

      local.get $out local.get $ip_count i32.const 2 i32.shl i32.add
      local.get $i i32.const 8 i32.add local.get $rbuf i32.add i32.load
      i32.store

      local.get $ip_count i32.const 1 i32.add local.set $ip_count
      local.get $i i32.const 12 i32.add local.set $i
      local.get $ans_count i32.const 1 i32.sub local.set $ans_count
      br $ans_loop
    end
    end

    local.get $ip_count i32.eqz
    if i64.const -6 return end

    i64.const 0
    local.get $ip_count
    i64.extend_i32_u
    i64.const 32
    i64.shl
    i64.or)