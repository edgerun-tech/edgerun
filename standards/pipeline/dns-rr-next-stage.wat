;; DNS RR Next Stage — slot 72
  ;; Input:  DNS message bytes via input pipe (offset embedded in first 4 bytes)
  ;; Output: 32-byte record {name_start, name_wire_len, type, class, ttl, rdlen, rdata_start, next_offset}
  ;; Calls $dns_rr_next from protocol/dns.wat

  (func $process_dns_rr_next (export "process_dns_rr_next")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $start i32) (local $status i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.lt_u (local.get $read) (i32.const 8)) (then (return (i32.const 0))))
    (local.set $start (i32.load (i32.const 0x3000)))
    (local.set $status (call $dns_rr_next (i32.add (i32.const 0x3000) (i32.const 4)) (i32.sub (local.get $read) (i32.const 4)) (local.get $start) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 32)))
    i32.const 32)
