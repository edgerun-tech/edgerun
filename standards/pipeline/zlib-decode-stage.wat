;; Zlib Decode Pipeline Stage — slot 31
  ;; Stage type: batch (state=0)
  ;; Input:  zlib-compressed bytes via input pipe
  ;; Output: decompressed raw bytes via output pipe
  ;; Uses $zlib_member_scan + $deflate_inflate_raw from codec/compress.wat

  (func $process_zlib_decode (export "process_zlib_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $len_slot i32) (local $read i32) (local $status i32)
    (local $record i32) (local $deflate_off i32) (local $deflate_len i32)
    (local $out_cap i32)
    (local.set $len_slot (i32.sub (i32.add (local.get $scratch) (local.get $scap)) (i32.const 4)))
    (drop (call $pipe_read_ptr (local.get $input) (local.get $len_slot)))
    (local.set $read (i32.load (local.get $len_slot)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (drop (call $pipe_read (local.get $input) (i32.const 0x3000) (local.get $read)))
    (local.set $record (i32.add (i32.const 0x3000) (local.get $read)))
    (local.set $status (call $zlib_member_scan (i32.const 0x3000) (local.get $read) (local.get $record)))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $deflate_off (i32.load (local.get $record)))
    (local.set $deflate_len (i32.load offset=4 (local.get $record)))
    (local.set $out_cap (i32.sub (local.get $scap) (i32.const 128)))
    (local.set $status
      (call $deflate_inflate_raw
        (i32.add (i32.const 0x3000) (local.get $deflate_off)) (local.get $deflate_len)
        (local.get $scratch) (local.get $out_cap)
        (local.get $out_cap)
        (i32.add (local.get $scratch) (local.get $out_cap))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $read (i32.load (i32.add (i32.add (local.get $scratch) (local.get $out_cap)) (i32.const 4))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $read)))
    local.get $read)
