(module
  (func $classify (param $command i32) (result i32)
    (local $result i32)
    (local.set $result (i32.const -4))
    (block $done
      (local.set $command
        (i32.and
          (i32.add (local.get $command) (i32.const -1))
          (i32.const 255)))
      (br_if $done (i32.gt_u (local.get $command) (i32.const 43)))
      (local.set $result
        (i32.load offset=1048576
          (i32.shl (local.get $command) (i32.const 2)))))
    (local.get $result))

  (func (export "validate_stream") (param $command i32) (param $stream_id i32) (result i32)
    (local $result i32)
    (local $tmp i32)
    (local.set $result (i32.const -1))
    (block $done
      (br_if $done (i32.gt_u (local.get $command) (i32.const 255)))
      (br_if $done (i32.ge_u (local.get $stream_id) (i32.const 65536)))
      (local.set $tmp (i32.const 0))
      (block $outer
        (block $stream_nonzero
          (block $stream_zero
            (br_table $done $outer $outer $outer $stream_zero $stream_nonzero $outer
              (i32.add (call $classify (local.get $command)) (i32.const 4))))
          (local.set $result (i32.const -1))
          (br_if $done (local.get $stream_id))
          (br $outer))
        (local.set $tmp
          (select
            (i32.const 0)
            (i32.const -1)
            (local.get $stream_id))))
      (local.set $result (local.get $tmp)))
    (local.get $result))

  (memory (;0;) 17)
  (data (;0;) (i32.const 1048576) "\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\fe\ff\ff\ff\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\01\00\00\00")
)
