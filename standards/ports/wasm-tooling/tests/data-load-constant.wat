(module
  (type (;0;) (func (result i32)))
  (func (export "load_first") (type 0) (result i32)
    i32.const 0
    i32.load offset=1048576)
  (func (export "load_second") (type 0) (result i32)
    i32.const 4
    i32.load offset=1048576)
  (func (export "load_third") (type 0) (result i32)
    i32.const 8
    i32.load offset=1048576)
  (memory (;0;) 17)
  (data (;0;) (i32.const 1048576) "\01\00\00\00\fe\ff\ff\ff\00\00\00\00")
)
