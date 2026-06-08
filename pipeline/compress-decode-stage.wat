;; Gzip/Zlib Decode Pipeline Stage — slots 29, 31
  ;; Input:  gzip or zlib-compressed bytes via input pipe
  ;; Output: decompressed raw bytes via output pipe

  (func $compress_decode_impl (param $input i32) (param $output i32)
    (param $scratch i32) (param $scap i32) (param $is_gzip i32) (result i32)
    (local $status i32) (local $record i32)
    (local $deflate_off i32) (local $deflate_len i32) (local $out_cap i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $record (i32.add (global.get $SCRATCH_BUF) (local.get $read)))
    (if (local.get $is_gzip)
      (then (local.set $status (call $gzip_member_scan (global.get $SCRATCH_BUF) (local.get $read) (local.get $record))))
      (else (local.set $status (call $zlib_member_scan (global.get $SCRATCH_BUF) (local.get $read) (local.get $record)))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $deflate_off (i32.load (local.get $record)))
    (local.set $deflate_len (i32.load offset=4 (local.get $record)))
    (local.set $out_cap (i32.sub (local.get $scap) (i32.const 128)))
    (local.set $status
      (call $deflate_inflate_raw
        (i32.add (global.get $SCRATCH_BUF) (local.get $deflate_off)) (local.get $deflate_len)
        (local.get $scratch) (local.get $out_cap)
        (local.get $out_cap)
        (i32.add (local.get $scratch) (local.get $out_cap))))
    (if (local.get $status) (then (return (i32.sub (i32.const 0) (local.get $status)))))
    (local.set $read (i32.load (i32.add (i32.add (local.get $scratch) (local.get $out_cap)) (i32.const 4))))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (local.get $read)))
    local.get $read)

  (func $compress_decode (export "process_gzip_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (return (call $compress_decode_impl
      (local.get $input) (local.get $output)
      (local.get $scratch) (local.get $scap) (i32.const 1))))

  (func $compress_decode_zlib (export "process_zlib_decode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (return (call $compress_decode_impl
      (local.get $input) (local.get $output)
      (local.get $scratch) (local.get $scap) (i32.const 0))))
