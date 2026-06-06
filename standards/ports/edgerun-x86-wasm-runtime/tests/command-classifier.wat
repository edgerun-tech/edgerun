(module
  (func (export "classify") (param $command i32) (result i32)
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
  (memory (;0;) 17)
  (data (;0;) (i32.const 1048576) "\01\00\00\00\01\00\00\00\01\00\00\00\01\00\00\00\fe\ff\ff\ff\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\00\01\00\00\00\01\00\00\00")
)
