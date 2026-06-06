(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (import "edgerun.host" "call" (func $host_call (param i32 i32) (result i32)))

  (memory 1)

  (func (export "entry") (result i32)
    i32.const 0
    i32.const 0
    call $host_call)
)
