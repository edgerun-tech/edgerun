(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (import "edgerun.clock" "now" (func $clock_now (result i32)))

  (memory 1)

  (func (export "entry") (result i32)
    call $clock_now)
)
