(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (memory 1)

  (global $hidden_state (mut i32) (i32.const 0))

  (func (export "entry") (result i32)
    global.get $hidden_state)
)
