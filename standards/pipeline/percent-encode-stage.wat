;; Percent Encode Pipeline Stage — slot 35
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: URL percent-encoded bytes via output pipe
  ;; Config: i32 mode (0=default component, 1=form-urlencoded, 2=fragment)
  ;; Calls $percent_encode_component from codec/string-url.wat

  (func $process_percent_encode (export "process_percent_encode")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)

    (local $result i64) (local $status i32) (local $out_len i32)
    (local $mode i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $mode (i32.const 0))
    (if (i32.ge_s (local.get $clen) (i32.const 4))
      (then (local.set $mode (i32.load (local.get $cfg)))))
    (local.set $result (call $percent_encode_component (i32.const 0x3000) (local.get $read) (local.get $scratch) (local.get $scap) (local.get $mode)))
    (return (call $stage_write_result (local.get $output) (local.get $result) (local.get $scratch)))
  )
