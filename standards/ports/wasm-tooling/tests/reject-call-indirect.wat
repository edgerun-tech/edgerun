(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (memory 1)

  (type $entry_type (func (result i32)))
  (table 1 funcref)
  (elem (i32.const 0) $target)

  (func $target (result i32)
    i32.const 1)

  (func (export "entry") (result i32)
    i32.const 0
    call_indirect (type $entry_type))
)
