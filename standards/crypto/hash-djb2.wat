(module
  (import "edgerun-core" "memory" (memory 1))
  ;; DJB2 hash — Dan Bernstein's djb2 string hash, modified with initial hash = 0.
  ;; This is the variant used by OSRS cache lookups.
  ;; Exports: djb2_hash(input_ptr, input_len) -> i32 hash
  (func (export "proto_standard_id") (result i32) i32.const 300521)

  (func (export "djb2_hash") (param $ptr i32) (param $len i32) (result i32)
    (local $hash i32) (local $i i32) (local $b i32)
    i32.const 0 local.set $hash
    i32.const 0 local.set $i
    block $done
    loop $loop
      local.get $i local.get $len i32.ge_u br_if $done
      local.get $ptr local.get $i i32.add i32.load8_u local.set $b
      local.get $b local.get $hash i32.const 5 i32.shl local.get $hash i32.sub i32.add local.set $hash
      local.get $i i32.const 1 i32.add local.set $i
      br $loop
    end
    end
    local.get $hash
  )
)