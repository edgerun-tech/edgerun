(module
  (import "edgerun-core" "memory" (memory 1))
(func (export "proto_standard_id") (result i32)
    i32.const 300084)

  ;; Status values: 0 ok, 1 unsupported, 2 short, 3 invalid.
  ;; Header out record, 32 bytes:
  ;;   u32 type, u32 machine, u64 entry, u64 phoff, u32 phentsize, u32 phnum.
  ;; Program header out record, 48 bytes:
  ;;   u32 kind, u32 flags, u64 offset, u64 vaddr, u64 filesz, u64 memsz, u64 align.

  (func $u16 (param $ptr i32) (result i32)
    local.get $ptr
    i32.load16_u align=1)

  (func $u32 (param $ptr i32) (result i32)
    local.get $ptr
    i32.load align=1)

  (func $u64 (param $ptr i32) (result i64)
    local.get $ptr
    i64.load align=1)

  (func $add_overflows_u64 (param $a i64) (param $b i64) (result i32)
    local.get $a
    local.get $b
    i64.add
    local.get $a
    i64.lt_u)

  (func $valid_header (param $ptr i32) (param $len i32) (result i32)
    local.get $len
    i32.const 64
    i32.lt_u
    if
      i32.const 2
      return
    end

    local.get $ptr
    i32.load8_u
    i32.const 0x7f
    i32.ne
    local.get $ptr
    i32.const 1
    i32.add
    i32.load8_u
    i32.const 0x45
    i32.ne
    i32.or
    local.get $ptr
    i32.const 2
    i32.add
    i32.load8_u
    i32.const 0x4c
    i32.ne
    i32.or
    local.get $ptr
    i32.const 3
    i32.add
    i32.load8_u
    i32.const 0x46
    i32.ne
    i32.or
    if
      i32.const 3
      return
    end

    local.get $ptr
    i32.const 4
    i32.add
    i32.load8_u
    i32.const 2
    i32.ne
    local.get $ptr
    i32.const 5
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $ptr
    i32.const 6
    i32.add
    i32.load8_u
    i32.const 1
    i32.ne
    i32.or
    local.get $ptr
    i32.const 54
    i32.add
    call $u16
    i32.const 56
    i32.ne
    i32.or
    if
      i32.const 1
      return
    end

    i32.const 0)

  (func (export "elf64_header_parse")
    (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $status i32)
    local.get $ptr
    local.get $len
    call $valid_header
    local.tee $status
    if
      local.get $status
      return
    end

    local.get $out
    local.get $ptr
    i32.const 16
    i32.add
    call $u16
    i32.store align=1
    local.get $out
    i32.const 4
    i32.add
    local.get $ptr
    i32.const 18
    i32.add
    call $u16
    i32.store align=1
    local.get $out
    i32.const 8
    i32.add
    local.get $ptr
    i32.const 24
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 16
    i32.add
    local.get $ptr
    i32.const 32
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 24
    i32.add
    local.get $ptr
    i32.const 54
    i32.add
    call $u16
    i32.store align=1
    local.get $out
    i32.const 28
    i32.add
    local.get $ptr
    i32.const 56
    i32.add
    call $u16
    i32.store align=1

    i32.const 0)

  (func (export "elf64_program_header_parse")
    (param $ptr i32) (param $len i32) (param $index i32) (param $out i32) (result i32)
    (local $status i32)
    (local $phoff64 i64)
    (local $offset64 i64)
    (local $offset i32)
    (local $phnum i32)

    local.get $ptr
    local.get $len
    call $valid_header
    local.tee $status
    if
      local.get $status
      return
    end

    local.get $ptr
    i32.const 56
    i32.add
    call $u16
    local.set $phnum
    local.get $index
    local.get $phnum
    i32.ge_u
    if
      i32.const 3
      return
    end

    local.get $ptr
    i32.const 32
    i32.add
    call $u64
    local.set $phoff64
    local.get $index
    i64.extend_i32_u
    i64.const 56
    i64.mul
    local.get $phoff64
    i64.add
    local.tee $offset64
    local.get $phoff64
    i64.lt_u
    if
      i32.const 2
      return
    end
    local.get $offset64
    i64.const 56
    call $add_overflows_u64
    if
      i32.const 2
      return
    end
    local.get $offset64
    i64.const 56
    i64.add
    local.get $len
    i64.extend_i32_u
    i64.gt_u
    if
      i32.const 2
      return
    end

    local.get $offset64
    i32.wrap_i64
    local.set $offset
    local.get $out
    local.get $ptr
    local.get $offset
    i32.add
    call $u32
    i32.store align=1
    local.get $out
    i32.const 4
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 4
    i32.add
    call $u32
    i32.store align=1
    local.get $out
    i32.const 8
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 8
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 16
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 16
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 24
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 32
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 32
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 40
    i32.add
    call $u64
    i64.store align=1
    local.get $out
    i32.const 40
    i32.add
    local.get $ptr
    local.get $offset
    i32.add
    i32.const 48
    i32.add
    call $u64
    i64.store align=1

    i32.const 0)

  (func (export "elf64_flags_permissions") (param $flags i32) (result i32)
    local.get $flags
    i32.const 4
    i32.and
    i32.const 0
    i32.ne
    local.get $flags
    i32.const 2
    i32.and
    i32.const 0
    i32.ne
    i32.const 1
    i32.shl
    i32.or
    local.get $flags
    i32.const 1
    i32.and
    i32.const 0
    i32.ne
    i32.const 2
    i32.shl
    i32.or)

  (func (export "elf64_entry_executable")
    (param $ptr i32) (param $len i32) (result i32)
    (local $status i32)
    (local $entry i64)
    (local $phoff64 i64)
    (local $offset64 i64)
    (local $offset i32)
    (local $phnum i32)
    (local $index i32)
    (local $base i32)
    (local $kind i32)
    (local $flags i32)
    (local $vaddr i64)
    (local $memsz i64)
    (local $end i64)

    local.get $ptr
    local.get $len
    call $valid_header
    local.tee $status
    if
      i32.const 0
      return
    end

    local.get $ptr
    i32.const 24
    i32.add
    call $u64
    local.set $entry
    local.get $ptr
    i32.const 32
    i32.add
    call $u64
    local.set $phoff64
    local.get $ptr
    i32.const 56
    i32.add
    call $u16
    local.set $phnum

    block $done
      loop $again
        local.get $index
        local.get $phnum
        i32.ge_u
        br_if $done

        local.get $index
        i64.extend_i32_u
        i64.const 56
        i64.mul
        local.get $phoff64
        i64.add
        local.tee $offset64
        local.get $phoff64
        i64.lt_u
        if
          i32.const 0
          return
        end
        local.get $offset64
        i64.const 56
        call $add_overflows_u64
        if
          i32.const 0
          return
        end
        local.get $offset64
        i64.const 56
        i64.add
        local.get $len
        i64.extend_i32_u
        i64.gt_u
        if
          i32.const 0
          return
        end

        local.get $ptr
        local.get $offset64
        i32.wrap_i64
        i32.add
        local.tee $base
        call $u32
        local.set $kind
        local.get $base
        i32.const 4
        i32.add
        call $u32
        local.set $flags

        local.get $kind
        i32.const 1
        i32.eq
        local.get $flags
        i32.const 1
        i32.and
        i32.const 0
        i32.ne
        i32.and
        if
          local.get $base
          i32.const 16
          i32.add
          call $u64
          local.set $vaddr
          local.get $base
          i32.const 40
          i32.add
          call $u64
          local.set $memsz
          local.get $vaddr
          local.get $memsz
          i64.add
          local.tee $end
          local.get $vaddr
          i64.lt_u
          if
            i32.const 0
            return
          end
          local.get $entry
          local.get $vaddr
          i64.ge_u
          local.get $entry
          local.get $end
          i64.lt_u
          i32.and
          if
            i32.const 1
            return
          end
        end

        local.get $index
        i32.const 1
        i32.add
        local.set $index
        br $again
      end
    end

    i32.const 0)
)
