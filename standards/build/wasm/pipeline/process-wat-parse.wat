(module
  (import "edgerun-core" "memory" (memory 1))
  (import "pipe-core" "pipe_read" (func $pipe_read (param i32 i32 i32) (result i32)))
  (import "pipe-core" "pipe_write" (func $pipe_write (param i32 i32 i32) (result i32)))
  (import "wat-parse-core" "wat_parse_module" (func $wat_parse_module (param i32 i32) (result i32)))

  (func (export "proto_standard_id") (result i32) i32.const 300300)

  (func (export "process_wat_parse")
    (param $input i32) (param $output i32) (param $cfg i32) (param $clen i32)
    (param $scratch i32) (param $scap i32) (param $state i32) (result i32)
    (local $wat_len i32) (local $err i32)

    (local.set $wat_len (call $pipe_read (local.get $input) (local.get $scratch) (local.get $scap)))
    (if (i32.le_s (local.get $wat_len) (i32.const 0))
      (then (return (local.get $wat_len))))

    (local.set $err (call $wat_parse_module (local.get $scratch) (local.get $wat_len)))
    (if (local.get $err)
      (then (return (i32.sub (i32.const 0) (local.get $err)))))

    (i32.store (local.get $scratch) (i32.const 0))
    (drop (call $pipe_write (local.get $output) (local.get $scratch) (i32.const 4)))
    (i32.const 4))
)
