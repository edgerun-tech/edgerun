(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (memory 1)

  (func $self (result i32)
    call $self)

  (export "entry" (func $self))
)
