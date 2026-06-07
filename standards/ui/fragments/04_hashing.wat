(module


  (func $fnv1a_byte (export "fnv1a_byte") (param $hash i32) (param $byte i32) (result i32)
    local.get $hash
    local.get $byte
    i32.xor
    i32.const 16777619
    i32.mul)

  (func $fnv1a_range (export "fnv1a_range") (param $hash i32) (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    block $done
      loop $loop
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $hash
        local.get $ptr
        local.get $i
        i32.add
        i32.load8_u
        call $fnv1a_byte
        local.set $hash
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end
    end
    local.get $hash)

)
