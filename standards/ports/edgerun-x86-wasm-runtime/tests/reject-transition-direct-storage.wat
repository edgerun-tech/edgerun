(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,storage.intent,render.emit")

  (memory 1)

  (func (export "er_init") (param i32) (result i32)
    i32.const 0)

  (func (export "er_handle_message") (param i32 i32 i32) (result i32)
    i32.const 96
    i32.const 0x5452414e
    i32.store
    i32.const 100
    i32.const 1
    i32.store
    i32.const 104
    i32.const 16
    i32.store
    i32.const 108
    i32.const 1
    i32.store
    i32.const 112
    i32.const 0
    i32.store
    i32.const 116
    i32.const 1
    i32.store
    i32.const 96)

  (func (export "er_handle_action") (param i32 i32 i32) (result i32)
    i32.const 0)

  (func (export "er_render") (param i32) (result i32)
    i32.const 0)
)
