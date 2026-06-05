(module
  (memory (export "memory") 1)

  (func (export "proto_abi_version") (result i32)
    i32.const 2)

  (func (export "proto_standard_id") (result i32)
    i32.const 300074)

  (func $load (param $ptr i32) (param $i i32) (result i32)
    local.get $ptr
    local.get $i
    i32.add
    i32.load8_u)

  (func $matches3 (param $b i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    local.get $b
    local.get $n1
    i32.eq
    local.get $b
    local.get $n2
    i32.eq
    i32.or
    local.get $b
    local.get $n3
    i32.eq
    i32.or)

  (func (export "memchr") (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr") (param $ptr i32) (param $len i32) (param $needle i32) (result i32)
    (local $i i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load
        local.get $needle
        i32.eq
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func (export "memchr2")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $b i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr2")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (result i32)
    (local $i i32)
    (local $b i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load
        local.set $b
        local.get $b
        local.get $n1
        i32.eq
        local.get $b
        local.get $n2
        i32.eq
        i32.or
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func (export "memchr3")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $len
        i32.ge_u
        br_if $done
        local.get $ptr
        local.get $i
        call $load
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memrchr3")
    (param $ptr i32) (param $len i32) (param $n1 i32) (param $n2 i32) (param $n3 i32) (result i32)
    (local $i i32)
    local.get $len
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $ptr
        local.get $i
        call $load
        local.get $n1
        local.get $n2
        local.get $n3
        call $matches3
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)

  (func $needle_at
    (param $hay_ptr i32) (param $needle_ptr i32) (param $pos i32) (param $needle_len i32)
    (result i32)
    (local $j i32)
    i32.const 0
    local.set $j
    (block $yes
      (loop $scan
        local.get $j
        local.get $needle_len
        i32.ge_u
        br_if $yes
        local.get $hay_ptr
        local.get $pos
        i32.add
        local.get $j
        i32.add
        i32.load8_u
        local.get $needle_ptr
        local.get $j
        i32.add
        i32.load8_u
        i32.ne
        if
          i32.const 0
          return
        end
        local.get $j
        i32.const 1
        i32.add
        local.set $j
        br $scan))
    i32.const 1)

  (func (export "memmem_find")
    (param $hay_ptr i32) (param $hay_len i32) (param $needle_ptr i32) (param $needle_len i32)
    (result i32)
    (local $i i32)
    (local $last i32)
    local.get $needle_len
    i32.eqz
    if
      i32.const 0
      return
    end
    local.get $needle_len
    local.get $hay_len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $hay_len
    local.get $needle_len
    i32.sub
    local.set $last
    i32.const 0
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        local.get $last
        i32.gt_u
        br_if $done
        local.get $hay_ptr
        local.get $needle_ptr
        local.get $i
        local.get $needle_len
        call $needle_at
        if
          local.get $i
          return
        end
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $scan))
    i32.const -1)

  (func (export "memmem_rfind")
    (param $hay_ptr i32) (param $hay_len i32) (param $needle_ptr i32) (param $needle_len i32)
    (result i32)
    (local $i i32)
    local.get $needle_len
    i32.eqz
    if
      local.get $hay_len
      return
    end
    local.get $needle_len
    local.get $hay_len
    i32.gt_u
    if
      i32.const -1
      return
    end
    local.get $hay_len
    local.get $needle_len
    i32.sub
    i32.const 1
    i32.add
    local.set $i
    (block $done
      (loop $scan
        local.get $i
        i32.eqz
        br_if $done
        local.get $i
        i32.const 1
        i32.sub
        local.set $i
        local.get $hay_ptr
        local.get $needle_ptr
        local.get $i
        local.get $needle_len
        call $needle_at
        if
          local.get $i
          return
        end
        br $scan))
    i32.const -1)
)
