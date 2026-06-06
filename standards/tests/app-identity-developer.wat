(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=developer;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false;capabilities=transition.emit,storage.intent,route.intent,tls.intent,sealed.intent,child.spawn,dependency.use,render.emit;dependencies=edgerun.ui:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")

  (memory 1)

  (func (export "er_init") (param i32) (result i32)
    i32.const 0)

  (func (export "er_handle_message") (param i32 i32 i32) (result i32)
    i32.const 0)

  (func (export "er_handle_action") (param i32 i32 i32) (result i32)
    i32.const 0)

  (func (export "er_render") (param i32) (result i32)
    i32.const 0)
)
