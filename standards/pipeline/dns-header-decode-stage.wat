;; DNS Header Decode Stage — slot 59
  ;; Stage type: batch (state=0)
  ;; Input:  12-byte raw DNS header via input pipe
  ;; Output: 52-byte parse record {id, flags, qr, opcode, aa, tc, rd, ra, rcode, qdcount, ancount, nscount, arcount}
  ;; Calls $dns_header_decode from protocol/dns.wat

  (func $process_dns_header_decode (export "process_dns_header_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.lt_u (local.get $read) (i32.const 12)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $status (call $dns_header_decode (i32.const 0x3000) (local.get $read) (local.get $scratch)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 52)))
    i32.const 52)
