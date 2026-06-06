(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (memory 1)

  (func $start)
  (start $start)

  (func (export "entry") (result i32)
    i32.const 1)
)
