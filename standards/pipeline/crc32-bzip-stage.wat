;; CRC-32 (BZIP2) Pipeline Stage — slot 26
  ;; Stage type: batch (state=0)
  ;; Input:  raw bytes via input pipe
  ;; Output: 4-byte CRC-32 (little-endian i32) via output pipe
  ;; Calls $bzip_crc32 from crypto/checksum-crc32-bzip.wat
  (func $process_crc32_bzip (export "process_crc32_bzip")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $crc i32)
    (local $read i32)
    (local.set $read (call $stage_read_input (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.eqz (local.get $read)) (then (return (i32.const 0))))
    (local.set $crc (call $bzip_crc32 (global.get $SCRATCH_BUF) (local.get $read)))
    (i32.store (global.get $SHA256_OUT_BUF) (local.get $crc))
    (drop (call $pipe_write (local.get $output) (global.get $SHA256_OUT_BUF) (i32.const 4)))
    i32.const 4)
