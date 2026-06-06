(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,render.emit")

  (memory 1)

  (func (export "er_init") (param i32) (result i32)
    i32.const 0)

  (func (export "er_handle_message") (param i32 i32 i32) (result i32)
    i32.const 96)

  (func (export "er_handle_action") (param i32 i32 i32) (result i32)
    i32.const 128)

  (func (export "er_render") (param i32) (result i32)
    i32.const 320)
)
