(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,storage.intent,render.emit;dependencies=edgerun.ui:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef;postinstall=run")

  (memory 1)

  (func (export "entry") (result i32)
    i32.const 0)
)
