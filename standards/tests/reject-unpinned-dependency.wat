(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,storage.intent,render.emit;dependencies=edgerun.ui")

  (memory 1)

  (func (export "entry") (result i32)
    i32.const 0)
)
