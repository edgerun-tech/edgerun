;; Linux syscall number globals (placeholder values — real values in edgerun-core)
  (global $LX64_UNSHARE      i32 (i32.const 272))
  (global $LX64_SETNS        i32 (i32.const 308))
  (global $LX64_SETHOSTNAME  i32 (i32.const 161))
  (global $LX64_SETDOMAINNAME i32 (i32.const 162))
  (global $LX64_MOUNT        i32 (i32.const 165))
  (global $LX64_PIVOT_ROOT   i32 (i32.const 155))
  (global $LX64_UMOUNT2      i32 (i32.const 166))
  (global $LX64_OPEN_TREE    i32 (i32.const 428))
  (global $LX64_MOVE_MOUNT   i32 (i32.const 429))
  (global $LX64_MOUNT_SETATTR i32 (i32.const 440))
  (global $LX64_SECCOMP      i32 (i32.const 317))
  (global $LX64_CAPSET       i32 (i32.const 125))
  (global $LX64_PRCTL        i32 (i32.const 157))
  (global $LX64_SETUID       i32 (i32.const 105))
  (global $LX64_SETGID       i32 (i32.const 106))
  (global $LX64_SETGROUPS    i32 (i32.const 116))
  (global $LX64_PRLIMIT64    i32 (i32.const 302))
  (global $LX64_SCHED_SETATTR i32 (i32.const 245))
  (global $LX64_IOPRIO_SET   i32 (i32.const 290))
  (global $LX64_UMASK        i32 (i32.const 95))
  (global $LX64_KILL         i32 (i32.const 62))
  (global $LX64_WAIT4        i32 (i32.const 61))
  (global $LX64_EXECVE       i32 (i32.const 59))
  (global $LX64_CLONE        i32 (i32.const 56))
  (global $LX64_BPF          i32 (i32.const 321))
  (global $LA64_UNSHARE      i32 (i32.const 0))
  (global $LA64_SETNS        i32 (i32.const 0))
  (global $LA64_SETHOSTNAME  i32 (i32.const 0))
  (global $LA64_SETDOMAINNAME i32 (i32.const 0))
  (global $LA64_MOUNT        i32 (i32.const 0))
  (global $LA64_PIVOT_ROOT   i32 (i32.const 0))
  (global $LA64_UMOUNT2      i32 (i32.const 0))
  (global $LA64_OPEN_TREE    i32 (i32.const 0))
  (global $LA64_MOVE_MOUNT   i32 (i32.const 0))
  (global $LA64_MOUNT_SETATTR i32 (i32.const 0))
  (global $LA64_SECCOMP      i32 (i32.const 0))
  (global $LA64_CAPSET       i32 (i32.const 0))
  (global $LA64_PRCTL        i32 (i32.const 0))
  (global $LA64_SETUID       i32 (i32.const 0))
  (global $LA64_SETGID       i32 (i32.const 0))
  (global $LA64_SETGROUPS    i32 (i32.const 0))
  (global $LA64_PRLIMIT64    i32 (i32.const 0))
  (global $LA64_SCHED_SETATTR i32 (i32.const 0))
  (global $LA64_IOPRIO_SET   i32 (i32.const 0))
  (global $LA64_UMASK        i32 (i32.const 0))
  (global $LA64_KILL         i32 (i32.const 0))
  (global $LA64_WAIT4        i32 (i32.const 0))
  (global $LA64_EXECVE       i32 (i32.const 0))
  (global $LA64_CLONE        i32 (i32.const 0))
  (global $LA64_BPF          i32 (i32.const 0))



  (func (export "oci_config_hash_lower") (param $ptr i32) (param $len i32) (result i32)
    (call $fnv1a_lower (local.get $ptr) (local.get $len)))

  (func (export "oci_namespace_kind") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1813408442)) (then (return (i32.const 1)))) ;; mount
    (if (i32.eq (local.get $h) (i32.const 1765866786)) (then (return (i32.const 2)))) ;; pid
    (if (i32.eq (local.get $h) (i32.const 1377339077)) (then (return (i32.const 3)))) ;; network
    (if (i32.eq (local.get $h) (i32.const 2835456301)) (then (return (i32.const 4)))) ;; ipc
    (if (i32.eq (local.get $h) (i32.const 1301541837)) (then (return (i32.const 5)))) ;; uts
    (if (i32.eq (local.get $h) (i32.const 1618501362)) (then (return (i32.const 6)))) ;; user
    (if (i32.eq (local.get $h) (i32.const 3410062693)) (then (return (i32.const 7)))) ;; cgroup
    (if (i32.eq (local.get $h) (i32.const 1564253156)) (then (return (i32.const 8)))) ;; time
    i32.const 0)

  (func (export "oci_namespace_flag") (param $kind i32) (result i32)
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $kind) (i32.const 3)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $kind) (i32.const 4)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $kind) (i32.const 5)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $kind) (i32.const 6)) (then (return (i32.const 32))))
    (if (i32.eq (local.get $kind) (i32.const 7)) (then (return (i32.const 64))))
    (if (i32.eq (local.get $kind) (i32.const 8)) (then (return (i32.const 128))))
    i32.const 0)

  (func (export "oci_default_namespace_mask") (result i32)
    i32.const 31)

  (func (export "oci_pull_policy") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 3069444615)) (then (return (i32.const 1)))) ;; missing
    (if (i32.eq (local.get $h) (i32.const 647213027)) (then (return (i32.const 1)))) ;; if-missing
    (if (i32.eq (local.get $h) (i32.const 1731637220)) (then (return (i32.const 2)))) ;; always
    (if (i32.eq (local.get $h) (i32.const 180965513)) (then (return (i32.const 3)))) ;; never
    i32.const 0)

  (func (export "oci_mount_option_flag") (param $ptr i32) (param $len i32) (result i32)
    (local $h i32)
    (local.set $h (call $fnv1a_lower (local.get $ptr) (local.get $len)))
    (if (i32.eq (local.get $h) (i32.const 1649706254)) (then (return (i32.const 1)))) ;; ro
    (if (i32.eq (local.get $h) (i32.const 1247043398)) (then (return (i32.const 2)))) ;; rw
    (if (i32.eq (local.get $h) (i32.const 3344129838)) (then (return (i32.const 4)))) ;; bind
    (if (i32.eq (local.get $h) (i32.const 7593790)) (then (return (i32.const 12)))) ;; rbind
    (if (i32.eq (local.get $h) (i32.const 3348235767)) (then (return (i32.const 16)))) ;; nosuid
    (if (i32.eq (local.get $h) (i32.const 1313770401)) (then (return (i32.const 32)))) ;; nodev
    (if (i32.eq (local.get $h) (i32.const 80650471)) (then (return (i32.const 64)))) ;; noexec
    (if (i32.eq (local.get $h) (i32.const 3127169598)) (then (return (i32.const 128)))) ;; strictatime
    (if (i32.eq (local.get $h) (i32.const 2767733972)) (then (return (i32.const 256)))) ;; shared
    (if (i32.eq (local.get $h) (i32.const 3162545472)) (then (return (i32.const 512)))) ;; slave
    (if (i32.eq (local.get $h) (i32.const 1657474316)) (then (return (i32.const 1024)))) ;; private
    (if (i32.eq (local.get $h) (i32.const 2373037001)) (then (return (i32.const 2048)))) ;; unbindable
    i32.const 0)

  (func (export "oci_path_absolute") (param $ptr i32) (param $len i32) (result i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47)))

  (func (export "oci_rootfs_relative_path_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $seg_start i32)
    (local $seg_len i32)
    (block $done
      (loop $scan
        (if (i32.ge_u (local.get $i) (local.get $len)) (then (br $done)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47))
          (then
            (local.set $seg_len (i32.sub (local.get $i) (local.get $seg_start)))
            (if
              (i32.and
                (i32.eq (local.get $seg_len) (i32.const 2))
                (i32.and
                  (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46))
                  (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $seg_start)) (i32.const 1))) (i32.const 46))))
              (then (return (i32.const 0))))
            (local.set $seg_start (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (local.set $seg_len (i32.sub (local.get $len) (local.get $seg_start)))
    (if
      (i32.and
        (i32.eq (local.get $seg_len) (i32.const 2))
        (i32.and
          (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $seg_start))) (i32.const 46))
          (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $seg_start)) (i32.const 1))) (i32.const 46))))
      (then (return (i32.const 0))))
    i32.const 1)

  (func (export "oci_container_id_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 255)) (then (return (i32.const 0))))
    (local.set $c (i32.load8_u (local.get $ptr)))
    (if (i32.eqz (call $is_alnum (local.get $c))) (then (return (i32.const 0))))
    (local.set $i (i32.const 1))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (call $is_alnum (local.get $c))
              (i32.or
                (i32.eq (local.get $c) (i32.const 45))
                (i32.or
                  (i32.eq (local.get $c) (i32.const 95))
                  (i32.eq (local.get $c) (i32.const 46))))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func (export "oci_hostname_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 253)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (call $is_alnum (local.get $c))
              (i32.or
                (i32.eq (local.get $c) (i32.const 45))
                (i32.eq (local.get $c) (i32.const 46)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func (export "oci_dns_server_valid") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.gt_u (local.get $len) (i32.const 255)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if
          (i32.eqz
            (i32.or
              (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
              (i32.or
                (i32.and (i32.ge_u (call $to_lower (local.get $c)) (i32.const 97)) (i32.le_u (call $to_lower (local.get $c)) (i32.const 102)))
                (i32.or
                  (i32.eq (local.get $c) (i32.const 46))
                  (i32.eq (local.get $c) (i32.const 58))))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

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

;; Status values: 0 ok, 1 invalid, 2 too large.
  ;; oci_reference_scan writes nine little-endian u32 slots:
  ;; registry_start,registry_len,repo_start,repo_len,tag_start,tag_len,digest_start,digest_len,kind.
  ;; kind: 1 tag, 2 digest, 3 both. Missing spans are written as zero length.


  (func $is_lower_hex (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 102)))))

  (func $is_tag_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
        (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90))))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 95))
          (i32.or (i32.eq (local.get $c) (i32.const 46)) (i32.eq (local.get $c) (i32.const 45)))))))

  (func $is_repo_char (param $c i32) (result i32)
    (i32.or
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 46))
          (i32.or (i32.eq (local.get $c) (i32.const 95)) (i32.eq (local.get $c) (i32.const 45)))))))

  (func $is_registry_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
        (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90))))
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))
        (i32.or
          (i32.eq (local.get $c) (i32.const 46))
          (i32.or
            (i32.eq (local.get $c) (i32.const 45))
            (i32.or
              (i32.eq (local.get $c) (i32.const 58))
              (i32.or (i32.eq (local.get $c) (i32.const 91)) (i32.eq (local.get $c) (i32.const 93)))))))))

  ;; (m142eq_lit removed — use $string_eq from runtime)


  (func $validate_tag (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (i32.const 128)))
      (then (return (i32.const 0))))
    (local.set $c (call $load8_u (local.get $ptr) (local.get $start)))
    (if (i32.eqz
          (i32.or
            (i32.eq (local.get $c) (i32.const 95))
            (i32.or
              (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))
              (i32.or
                (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90)))
                (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122)))))))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eqz (call $is_tag_char (call $load8_u (local.get $ptr) (i32.add (local.get $start) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $validate_registry (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (local.set $c (call $load8_u (local.get $ptr) (local.get $start)))
    (if (i32.eq (local.get $c) (i32.const 46)) (then (return (i32.const 0))))
    (local.set $c (call $load8_u (local.get $ptr) (i32.sub (i32.add (local.get $start) (local.get $len)) (i32.const 1))))
    (if (i32.eq (local.get $c) (i32.const 46)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eqz (call $is_registry_char (call $load8_u (local.get $ptr) (i32.add (local.get $start) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $validate_repo (param $ptr i32) (param $start i32) (param $len i32) (result i32)
    (local $i i32)
    (local $prev_slash i32)
    (local $c i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (if (i32.eq (call $load8_u (local.get $ptr) (local.get $start)) (i32.const 47)) (then (return (i32.const 0))))
    (if (i32.eq (call $load8_u (local.get $ptr) (i32.sub (i32.add (local.get $start) (local.get $len)) (i32.const 1))) (i32.const 47))
      (then (return (i32.const 0))))
    (local.set $prev_slash (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (call $load8_u (local.get $ptr) (i32.add (local.get $start) (local.get $i))))
        (if (i32.eq (local.get $c) (i32.const 47))
          (then
            (if (local.get $prev_slash) (then (return (i32.const 0))))
            (local.set $prev_slash (i32.const 1)))
          (else
            (if (i32.eqz (call $is_repo_char (local.get $c))) (then (return (i32.const 0))))
            (local.set $prev_slash (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $oci_digest_validate (export "oci_digest_validate") (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $c i32)
    (if (i32.ne (local.get $len) (i32.const 71)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (local.get $ptr)) (i32.const 115)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 104)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 2))) (i32.const 97)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 3))) (i32.const 50)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 4))) (i32.const 53)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 5))) (i32.const 54)) (then (return (i32.const 1))))
    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (i32.const 6))) (i32.const 58)) (then (return (i32.const 1))))
    (local.set $i (i32.const 7))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $c (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.eqz (call $is_lower_hex (local.get $c))) (then (return (i32.const 1))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 0)

  (func $store_out
    (param $out i32) (param $rs i32) (param $rl i32) (param $ps i32) (param $pl i32)
    (param $ts i32) (param $tl i32) (param $ds i32) (param $dl i32) (param $kind i32)
    (i32.store (local.get $out) (local.get $rs))
    (i32.store (i32.add (local.get $out) (i32.const 4)) (local.get $rl))
    (i32.store (i32.add (local.get $out) (i32.const 8)) (local.get $ps))
    (i32.store (i32.add (local.get $out) (i32.const 12)) (local.get $pl))
    (i32.store (i32.add (local.get $out) (i32.const 16)) (local.get $ts))
    (i32.store (i32.add (local.get $out) (i32.const 20)) (local.get $tl))
    (i32.store (i32.add (local.get $out) (i32.const 24)) (local.get $ds))
    (i32.store (i32.add (local.get $out) (i32.const 28)) (local.get $dl))
    (i32.store (i32.add (local.get $out) (i32.const 32)) (local.get $kind)))

  (func (export "oci_reference_scan") (param $ptr i32) (param $len i32) (param $out i32) (result i32)
    (local $i i32)
    (local $first_slash i32)
    (local $last_slash i32)
    (local $last_colon i32)
    (local $at i32)
    (local $registry_start i32)
    (local $registry_len i32)
    (local $repo_start i32)
    (local $repo_end i32)
    (local $tag_start i32)
    (local $tag_len i32)
    (local $digest_start i32)
    (local $digest_len i32)
    (local $kind i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $len) (i32.const 512)) (then (return (i32.const 2))))
    (call $store_out (local.get $out) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0) (i32.const 0))
    (local.set $first_slash (i32.const -1))
    (local.set $last_slash (i32.const -1))
    (local.set $last_colon (i32.const -1))
    (local.set $at (i32.const -1))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eq (call $load8_u (local.get $ptr) (local.get $i)) (i32.const 47))
          (then
            (if (i32.eq (local.get $first_slash) (i32.const -1)) (then (local.set $first_slash (local.get $i))))
            (local.set $last_slash (local.get $i))))
        (if (i32.and
              (i32.eq (local.get $at) (i32.const -1))
              (i32.eq (call $load8_u (local.get $ptr) (local.get $i)) (i32.const 58)))
          (then (local.set $last_colon (local.get $i))))
        (if (i32.eq (call $load8_u (local.get $ptr) (local.get $i)) (i32.const 64))
          (then
            (if (i32.ne (local.get $at) (i32.const -1)) (then (return (i32.const 1))))
            (local.set $at (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (if (i32.ne (local.get $at) (i32.const -1))
      (then
        (local.set $digest_start (i32.add (local.get $at) (i32.const 1)))
        (local.set $digest_len (i32.sub (local.get $len) (local.get $digest_start)))
        (if (call $oci_digest_validate (i32.add (local.get $ptr) (local.get $digest_start)) (local.get $digest_len))
          (then (return (i32.const 1))))
        (local.set $repo_end (local.get $at))
        (local.set $kind (i32.const 2)))
      (else
        (local.set $repo_end (local.get $len))))
    (if (i32.and
          (i32.and
            (i32.ne (local.get $last_colon) (i32.const -1))
            (i32.or
              (i32.eq (local.get $last_slash) (i32.const -1))
              (i32.gt_u (local.get $last_colon) (local.get $last_slash))))
          (i32.lt_u (local.get $last_colon) (local.get $repo_end)))
      (then
        (local.set $tag_start (i32.add (local.get $last_colon) (i32.const 1)))
        (local.set $tag_len (i32.sub (local.get $repo_end) (local.get $tag_start)))
        (if (i32.eqz (call $validate_tag (local.get $ptr) (local.get $tag_start) (local.get $tag_len)))
          (then (return (i32.const 1))))
        (local.set $repo_end (local.get $last_colon))
        (local.set $kind (i32.or (local.get $kind) (i32.const 1)))))
    (if (i32.eqz (local.get $kind)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $first_slash) (i32.const -1))
      (then
        (local.set $registry_start (i32.const 0))
        (local.set $registry_len (local.get $first_slash))
        (local.set $repo_start (i32.add (local.get $first_slash) (i32.const 1))))
      (else
        (local.set $repo_start (i32.const 0))))
    (if (i32.eq (local.get $registry_len) (i32.const 0))
      (then (local.set $registry_start (i32.const 0))))
    (local.set $repo_end (local.get $repo_end))
    (if (i32.eqz (call $validate_registry (local.get $ptr) (local.get $registry_start) (local.get $registry_len)))
      (then (return (i32.const 1))))
    (if (i32.eqz (call $validate_repo (local.get $ptr) (local.get $repo_start) (i32.sub (local.get $repo_end) (local.get $repo_start))))
      (then (return (i32.const 1))))
    (call $store_out
      (local.get $out)
      (local.get $registry_start) (local.get $registry_len)
      (local.get $repo_start) (i32.sub (local.get $repo_end) (local.get $repo_start))
      (local.get $tag_start) (local.get $tag_len)
      (local.get $digest_start) (local.get $digest_len)
      (local.get $kind))
    i32.const 0)

  (func (export "oci_media_layer_compression") (param $ptr i32) (param $len i32) (result i32)
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4096) (i32.const 38)) (then (return (i32.const 0))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4160) (i32.const 55)) (then (return (i32.const 0))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4224) (i32.const 44)) (then (return (i32.const 0))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4288) (i32.const 43)) (then (return (i32.const 1))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4352) (i32.const 60)) (then (return (i32.const 1))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4416) (i32.const 49)) (then (return (i32.const 1))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4480) (i32.const 43)) (then (return (i32.const 2))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 4544) (i32.const 60)) (then (return (i32.const 2))))
    i32.const 3)


  ;; Syscall number constants defined in edgerun-core as LINUX_SYS_X64_* / LINUX_SYS_AARCH64_*
  ;; aarch64 syscall classification constants
  ;; ABI status codes: 0 ok/valid, 1 invalid.
  ;; OCI lifecycle status ids:
  ;;   0 creating, 1 created, 2 running, 3 stopped, 4 deleted.
  ;; Synthetic current-state id 255 means no persisted runtime state yet.
  ;; Lifecycle event ids:
  ;;   1 create_begin, 2 create_commit, 3 start, 4 process_exit, 5 delete.
  ;; Syscall operation classes:
  ;;   0 unknown, 1 namespace, 2 mount, 3 security, 4 resource, 5 process, 6 ebpf.
  ;; Architecture ids for syscall-number classification:
  ;;   1 linux x86_64, 2 linux aarch64.


  (func (export "oci_status_valid") (param $status i32) (result i32)
    (i32.le_u (local.get $status) (i32.const 4)))

  (func (export "oci_status_terminal") (param $status i32) (result i32)
    (i32.or
      (i32.eq (local.get $status) (i32.const 3))
      (i32.eq (local.get $status) (i32.const 4))))

  (func (export "oci_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 1024) (i32.const 8))
      (then (return (i32.const 0))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 1032) (i32.const 7))
      (then (return (i32.const 1))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 1040) (i32.const 7))
      (then (return (i32.const 2))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 1048) (i32.const 7))
      (then (return (i32.const 3))))
    (if (call $string_eq (local.get $ptr) (local.get $len) (i32.const 1056) (i32.const 7))
      (then (return (i32.const 4))))
    i32.const -1)

  (func $oci_lifecycle_transition (export "oci_lifecycle_transition") (param $current i32) (param $event i32) (result i32)
    (if (i32.and (i32.eq (local.get $current) (i32.const 255)) (i32.eq (local.get $event) (i32.const 1)))
      (then (return (i32.const 0))))
    (if (i32.and (i32.eq (local.get $current) (i32.const 0)) (i32.eq (local.get $event) (i32.const 2)))
      (then (return (i32.const 1))))
    (if (i32.and (i32.eq (local.get $current) (i32.const 1)) (i32.eq (local.get $event) (i32.const 3)))
      (then (return (i32.const 2))))
    (if (i32.and (i32.eq (local.get $current) (i32.const 2)) (i32.eq (local.get $event) (i32.const 4)))
      (then (return (i32.const 3))))
    (if (i32.and (i32.eq (local.get $current) (i32.const 3)) (i32.eq (local.get $event) (i32.const 5)))
      (then (return (i32.const 4))))
    i32.const -1)

  (func (export "oci_lifecycle_can_transition") (param $current i32) (param $event i32) (result i32)
    (i32.ne (call $oci_lifecycle_transition (local.get $current) (local.get $event)) (i32.const -1)))

  (func (export "oci_syscall_op_class") (param $op i32) (result i32)
    ;; namespace: unshare, setns, sethostname, setdomainname
    (if (i32.and (i32.ge_u (local.get $op) (i32.const 1)) (i32.le_u (local.get $op) (i32.const 3)))
      (then (return (i32.const 1))))
    (if (i32.eq (local.get $op) (i32.const 24)) (then (return (i32.const 1))))
    ;; mount/rootfs: mount, pivot_root, umount2, open_tree, move_mount, mount_setattr
    (if (i32.and (i32.ge_u (local.get $op) (i32.const 4)) (i32.le_u (local.get $op) (i32.const 9)))
      (then (return (i32.const 2))))
    ;; security/identity: seccomp, capset, prctl, setuid, setgid, setgroups
    (if (i32.and (i32.ge_u (local.get $op) (i32.const 10)) (i32.le_u (local.get $op) (i32.const 15)))
      (then (return (i32.const 3))))
    ;; resource policy: prlimit64, sched_setattr, ioprio_set, umask
    (if (i32.and (i32.ge_u (local.get $op) (i32.const 16)) (i32.le_u (local.get $op) (i32.const 19)))
      (then (return (i32.const 4))))
    ;; eBPF device/network cgroup helpers
    (if (i32.eq (local.get $op) (i32.const 20)) (then (return (i32.const 6))))
    ;; process control: kill, wait, exec, fork/clone
    (if (i32.and (i32.ge_u (local.get $op) (i32.const 21)) (i32.le_u (local.get $op) (i32.const 23)))
      (then (return (i32.const 5))))
    (if (i32.eq (local.get $op) (i32.const 25)) (then (return (i32.const 5))))
    i32.const 0)

  (func $x86_syscall_class (param $nr i32) (result i32)
    (if (i32.or (i32.or (i32.eq (local.get $nr) (global.get $LX64_UNSHARE)) (i32.eq (local.get $nr) (global.get $LX64_SETNS)))
                (i32.or (i32.eq (local.get $nr) (global.get $LX64_SETHOSTNAME)) (i32.eq (local.get $nr) (global.get $LX64_SETDOMAINNAME))))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (global.get $LX64_MOUNT)) (i32.eq (local.get $nr) (global.get $LX64_PIVOT_ROOT)))
            (i32.or (i32.eq (local.get $nr) (global.get $LX64_UMOUNT2)) (i32.eq (local.get $nr) (global.get $LX64_OPEN_TREE))))
          (i32.or (i32.eq (local.get $nr) (global.get $LX64_MOVE_MOUNT)) (i32.eq (local.get $nr) (global.get $LX64_MOUNT_SETATTR))))
      (then (return (i32.const 2))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (global.get $LX64_SECCOMP)) (i32.eq (local.get $nr) (global.get $LX64_CAPSET)))
            (i32.or (i32.eq (local.get $nr) (global.get $LX64_PRCTL)) (i32.eq (local.get $nr) (global.get $LX64_SETUID))))
          (i32.or (i32.eq (local.get $nr) (global.get $LX64_SETGID)) (i32.eq (local.get $nr) (global.get $LX64_SETGROUPS))))
      (then (return (i32.const 3))))
    (if (i32.or (i32.or (i32.eq (local.get $nr) (global.get $LX64_PRLIMIT64)) (i32.eq (local.get $nr) (global.get $LX64_SCHED_SETATTR)))
                (i32.or (i32.eq (local.get $nr) (global.get $LX64_IOPRIO_SET)) (i32.eq (local.get $nr) (global.get $LX64_UMASK))))
      (then (return (i32.const 4))))
    (if (i32.or
          (i32.or (i32.eq (local.get $nr) (global.get $LX64_KILL)) (i32.eq (local.get $nr) (global.get $LX64_WAIT4)))
          (i32.or (i32.eq (local.get $nr) (global.get $LX64_EXECVE)) (i32.eq (local.get $nr) (global.get $LX64_CLONE))))
      (then (return (i32.const 5))))
    (if (i32.eq (local.get $nr) (global.get $LX64_BPF)) (then (return (i32.const 6))))
    i32.const 0)

  (func $aarch64_syscall_class (param $nr i32) (result i32)
    (if (i32.or (i32.or (i32.eq (local.get $nr) (global.get $LA64_UNSHARE)) (i32.eq (local.get $nr) (global.get $LA64_SETNS)))
                (i32.or (i32.eq (local.get $nr) (global.get $LA64_SETHOSTNAME)) (i32.eq (local.get $nr) (global.get $LA64_SETDOMAINNAME))))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (global.get $LA64_MOUNT)) (i32.eq (local.get $nr) (global.get $LA64_PIVOT_ROOT)))
            (i32.or (i32.eq (local.get $nr) (global.get $LA64_UMOUNT2)) (i32.eq (local.get $nr) (global.get $LA64_OPEN_TREE))))
          (i32.or (i32.eq (local.get $nr) (global.get $LA64_MOVE_MOUNT)) (i32.eq (local.get $nr) (global.get $LA64_MOUNT_SETATTR))))
      (then (return (i32.const 2))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (global.get $LA64_SECCOMP)) (i32.eq (local.get $nr) (global.get $LA64_CAPSET)))
            (i32.or (i32.eq (local.get $nr) (global.get $LA64_PRCTL)) (i32.eq (local.get $nr) (global.get $LA64_SETUID))))
          (i32.or (i32.eq (local.get $nr) (global.get $LA64_SETGID)) (i32.eq (local.get $nr) (global.get $LA64_SETGROUPS))))
      (then (return (i32.const 3))))
    (if (i32.or (i32.or (i32.eq (local.get $nr) (global.get $LA64_PRLIMIT64)) (i32.eq (local.get $nr) (global.get $LA64_SCHED_SETATTR)))
                (i32.or (i32.eq (local.get $nr) (global.get $LA64_IOPRIO_SET)) (i32.eq (local.get $nr) (global.get $LA64_UMASK))))
      (then (return (i32.const 4))))
    (if (i32.or
          (i32.or (i32.eq (local.get $nr) (global.get $LA64_KILL)) (i32.eq (local.get $nr) (global.get $LA64_WAIT4)))
          (i32.or (i32.eq (local.get $nr) (global.get $LA64_EXECVE)) (i32.eq (local.get $nr) (global.get $LA64_CLONE))))
      (then (return (i32.const 5))))
    (if (i32.eq (local.get $nr) (global.get $LA64_BPF)) (then (return (i32.const 6))))
    i32.const 0)

  (func (export "oci_linux_syscall_class") (param $arch i32) (param $nr i32) (result i32)
    (if (i32.eq (local.get $arch) (i32.const 1))
      (then (return (call $x86_syscall_class (local.get $nr)))))
    (if (i32.eq (local.get $arch) (i32.const 2))
      (then (return (call $aarch64_syscall_class (local.get $nr)))))
    i32.const 0)

  (func (export "oci_pack_exited_wait_status") (param $exit_code i32) (result i32)
    (i32.shl (i32.and (local.get $exit_code) (i32.const 255)) (i32.const 8)))

  (func (export "oci_pack_signaled_wait_status") (param $signal i32) (result i32)
    (i32.and (local.get $signal) (i32.const 127)))

  (func $oci_wait_status_exited (export "oci_wait_status_exited") (param $status i32) (result i32)
    (i32.eqz (i32.and (local.get $status) (i32.const 127))))

  (func $oci_wait_status_signaled (export "oci_wait_status_signaled") (param $status i32) (result i32)
    (i32.ge_u (i32.add (i32.and (local.get $status) (i32.const 127)) (i32.const 1)) (i32.const 2)))

  (func (export "oci_exit_code_from_wait_status") (param $status i32) (result i32)
    (if (call $oci_wait_status_exited (local.get $status))
      (then (return (i32.and (i32.shr_u (local.get $status) (i32.const 8)) (i32.const 255)))))
    (if (call $oci_wait_status_signaled (local.get $status))
      (then (return (i32.add (i32.const 128) (i32.and (local.get $status) (i32.const 127))))))
    i32.const 128)

  (func (export "oci_pack_state_exit") (param $status i32) (param $exit_code i32) (result i64)
    (i64.or
      (i64.extend_i32_u (local.get $status))
      (i64.shl (i64.extend_i32_u (local.get $exit_code)) (i64.const 32))))

  (func (export "oci_packed_state_status") (param $packed i64) (result i32)
    (i32.wrap_i64 (i64.and (local.get $packed) (i64.const 0xffffffff))))

  (func (export "oci_packed_state_exit") (param $packed i64) (result i32)
    (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))


;; Status: 0 ok, 1 unsupported kind, 2 short, 3 invalid checksum/path/octal, 4 zero block.
  ;; Header output is eleven little-endian u32 slots:
  ;; kind,size_lo,size_hi,mode,uid,gid,mtime_lo,mtime_hi,path_len,link_len,whiteout_kind.
  ;; kind: regular=0, hardlink=1, symlink=2, char=3, block=4, dir=5, fifo=6, pax=7,
  ;; global_pax=8, gnu_long_name=9, gnu_long_link=10. whiteout: none=0, entry=1, opaque=2.

  (func $field_len (param $ptr i32) (param $max i32) (result i32)
    (local $i i32)
    (local $end i32)
    (local.set $i (i32.const 0))
    (local.set $end (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $max)))
        (br_if $done (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
        (local.set $end (i32.add (local.get $i) (i32.const 1)))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (block $trim_done
      (loop $trim
        (br_if $trim_done (i32.eqz (local.get $end)))
        (br_if $trim_done
          (i32.ne
            (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $end) (i32.const 1))))
            (i32.const 32)))
        (local.set $end (i32.sub (local.get $end) (i32.const 1)))
        (br $trim)))
    local.get $end)

  (func $is_zero_block (param $ptr i32) (result i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (i32.const 512)))
        (if (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func $parse_octal64 (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $i i32)
    (local $b i32)
    (local $saw i32)
    (local $value i64)
    (local.set $i (i32.const 0))
    (local.set $value (i64.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $b (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
        (if (i32.or (i32.eqz (local.get $b)) (i32.eq (local.get $b) (i32.const 32)))
          (then
            (if (local.get $saw)
              (then (br $done)))))
        (if (i32.and (i32.ge_u (local.get $b) (i32.const 48)) (i32.le_u (local.get $b) (i32.const 55)))
          (then
            (local.set $saw (i32.const 1))
            (local.set $value
              (i64.add
                (i64.shl (local.get $value) (i64.const 3))
                (i64.extend_i32_u (i32.sub (local.get $b) (i32.const 48))))))
          (else
            (if (i32.and (i32.ne (local.get $b) (i32.const 0)) (i32.ne (local.get $b) (i32.const 32)))
              (then (return (i32.const 3))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (i32.store (local.get $out_ptr) (i32.wrap_i64 (local.get $value)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.wrap_i64 (i64.shr_u (local.get $value) (i64.const 32))))
    i32.const 0)

  (func (export "oci_tar_octal_u64") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (call $parse_octal64 (local.get $ptr) (local.get $len) (local.get $out_ptr)))

  (func $is_component_dots (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (if (i32.eq (i32.sub (local.get $end) (local.get $start)) (i32.const 2))
      (then
        (return
          (i32.and
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46))
            (i32.eq
              (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1)))
              (i32.const 46))))))
    i32.const 0)

  (func $path_safe_len (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (local $component_start i32)
    (local $normalized i32)
    (if (i32.eqz (local.get $len))
      (then (return (i32.const 0))))
    (if (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 47))
      (then (return (i32.const 0))))
    (local.set $i (i32.const 0))
    (local.set $component_start (i32.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.gt_u (local.get $i) (local.get $len)))
        (if (i32.or
              (i32.eq (local.get $i) (local.get $len))
              (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $i))) (i32.const 47)))
          (then
            (if (i32.gt_u (local.get $i) (local.get $component_start))
              (then
                (if (call $is_component_dots (local.get $ptr) (local.get $component_start) (local.get $i))
                  (then (return (i32.const 0))))
                (if (i32.ne
                      (i32.sub (local.get $i) (local.get $component_start))
                      (i32.const 1))
                  (then (local.set $normalized (i32.const 1)))
                  (else
                    (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $component_start))) (i32.const 46))
                      (then (local.set $normalized (i32.const 1))))))))
            (local.set $component_start (i32.add (local.get $i) (i32.const 1)))))
        (if (i32.and
              (i32.lt_u (local.get $i) (local.get $len))
              (i32.eqz (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    local.get $normalized)

  (func (export "oci_tar_path_safe") (param $ptr i32) (param $len i32) (result i32)
    (call $path_safe_len (local.get $ptr) (local.get $len)))

  (func $whiteout_kind_name (param $ptr i32) (param $start i32) (param $end i32) (result i32)
    (local $n i32)
    (local $ok i32)
    (local.set $n (i32.sub (local.get $end) (local.get $start)))
    (if (i32.eq (local.get $n) (i32.const 12))
      (then
        (local.set $ok (i32.const 1))
        (if (i32.ne (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1))) (i32.const 119)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))) (i32.const 104)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 3))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 4))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 5))) (i32.const 119)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 6))) (i32.const 104)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 7))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 8))) (i32.const 46)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 9))) (i32.const 111)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 10))) (i32.const 112)) (then (local.set $ok (i32.const 0))))
        (if (i32.ne (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 11))) (i32.const 113)) (then (local.set $ok (i32.const 0))))
        (if (local.get $ok) (then (return (i32.const 2))))))
    (if (i32.and
          (i32.gt_u (local.get $n) (i32.const 4))
          (i32.and
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (local.get $start))) (i32.const 46))
            (i32.and
              (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 1))) (i32.const 119))
              (i32.and
                (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 2))) (i32.const 104))
                (i32.eq (i32.load8_u (i32.add (i32.add (local.get $ptr) (local.get $start)) (i32.const 3))) (i32.const 46))))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func $whiteout_kind (param $name_ptr i32) (param $name_len i32) (result i32)
    (local $i i32)
    (local $last i32)
    (local.set $last (i32.const 0))
    (local.set $i (i32.const 0))
    (block $done2
      (loop $scann
        (br_if $done2 (i32.ge_u (local.get $i) (local.get $name_len)))
        (if (i32.eq (i32.load8_u (i32.add (local.get $name_ptr) (local.get $i))) (i32.const 47))
          (then (local.set $last (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scann)))
    (call $whiteout_kind_name (local.get $name_ptr) (local.get $last) (local.get $name_len)))

  (func $kind_code (param $byte i32) (result i32)
    (if (i32.or (i32.eqz (local.get $byte)) (i32.eq (local.get $byte) (i32.const 48))) (then (return (i32.const 0))))
    (if (i32.eq (local.get $byte) (i32.const 49)) (then (return (i32.const 1))))
    (if (i32.eq (local.get $byte) (i32.const 50)) (then (return (i32.const 2))))
    (if (i32.eq (local.get $byte) (i32.const 51)) (then (return (i32.const 3))))
    (if (i32.eq (local.get $byte) (i32.const 52)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $byte) (i32.const 53)) (then (return (i32.const 5))))
    (if (i32.eq (local.get $byte) (i32.const 54)) (then (return (i32.const 6))))
    (if (i32.eq (local.get $byte) (i32.const 120)) (then (return (i32.const 7))))
    (if (i32.eq (local.get $byte) (i32.const 103)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $byte) (i32.const 76)) (then (return (i32.const 9))))
    (if (i32.eq (local.get $byte) (i32.const 75)) (then (return (i32.const 10))))
    i32.const 255)

  (func $verify_checksum (param $ptr i32) (result i32)
    (local $i i32)
    (local $sum i64)
    (local $expected i64)
    (if (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 148)) (i32.const 8) (i32.const 0))
      (then (return (i32.const 0))))
    (local.set $expected
      (i64.or
        (i64.extend_i32_u (i32.load (i32.const 0)))
        (i64.shl (i64.extend_i32_u (i32.load (i32.const 4))) (i64.const 32))))
    (local.set $i (i32.const 0))
    (local.set $sum (i64.const 0))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (i32.const 512)))
        (local.set $sum
          (i64.add
            (local.get $sum)
            (i64.extend_i32_u
              (select
                (i32.const 32)
                (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
                (i32.and (i32.ge_u (local.get $i) (i32.const 148)) (i32.lt_u (local.get $i) (i32.const 156)))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (i64.eq (local.get $expected) (local.get $sum)))

  (func (export "oci_tar_header_parse") (param $ptr i32) (param $len i32) (param $out_ptr i32) (result i32)
    (local $name_len i32)
    (local $prefix_len i32)
    (local $link_len i32)
    (local $kind i32)
    (local $status i32)
    (if (i32.lt_u (local.get $len) (i32.const 512))
      (then (return (i32.const 2))))
    (if (call $is_zero_block (local.get $ptr))
      (then (return (i32.const 4))))
    (if (i32.eqz (call $verify_checksum (local.get $ptr)))
      (then (return (i32.const 3))))
    (local.set $name_len (call $field_len (local.get $ptr) (i32.const 100)))
    (local.set $prefix_len (call $field_len (i32.add (local.get $ptr) (i32.const 345)) (i32.const 155)))
    (local.set $link_len (call $field_len (i32.add (local.get $ptr) (i32.const 157)) (i32.const 100)))
    (if (i32.eqz (local.get $name_len))
      (then (return (i32.const 3))))
    (if (local.get $prefix_len)
      (then
        (if (i32.eqz (call $path_safe_len (i32.add (local.get $ptr) (i32.const 345)) (local.get $prefix_len)))
          (then (return (i32.const 3))))
        (if (i32.eqz (call $path_safe_len (local.get $ptr) (local.get $name_len)))
          (then (return (i32.const 3)))))
      (else
        (if (i32.eqz (call $path_safe_len (local.get $ptr) (local.get $name_len)))
          (then (return (i32.const 3))))))
    (local.set $kind (call $kind_code (i32.load8_u (i32.add (local.get $ptr) (i32.const 156)))))
    (if (i32.eq (local.get $kind) (i32.const 255))
      (then (return (i32.const 1))))
    (i32.store (local.get $out_ptr) (local.get $kind))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 124)) (i32.const 12) (i32.add (local.get $out_ptr) (i32.const 4))))
    (if (local.get $status) (then (return (local.get $status))))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 100)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 108)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 16)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 116)) (i32.const 8) (i32.const 0)))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 20)) (i32.load (i32.const 0)))
    (local.set $status (call $parse_octal64 (i32.add (local.get $ptr) (i32.const 136)) (i32.const 12) (i32.add (local.get $out_ptr) (i32.const 24))))
    (if (local.get $status) (then (return (local.get $status))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 32))
      (i32.add (i32.add (local.get $name_len) (local.get $prefix_len)) (select (i32.const 1) (i32.const 0) (local.get $prefix_len))))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 36)) (local.get $link_len))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 40))
      (call $whiteout_kind
        (local.get $ptr)
        (local.get $name_len)))
    i32.const 0)

  (data (i32.const 4096) "application/vnd.oci.image.layer.v1.tar")
  (data (i32.const 4160) "application/vnd.oci.image.layer.nondistributable.v1.tar")
  (data (i32.const 4224) "application/vnd.docker.image.rootfs.diff.tar")
  (data (i32.const 4288) "application/vnd.oci.image.layer.v1.tar+gzip")
  (data (i32.const 4352) "application/vnd.oci.image.layer.nondistributable.v1.tar+gzip")
  (data (i32.const 4416) "application/vnd.docker.image.rootfs.diff.tar.gzip")
  (data (i32.const 4480) "application/vnd.oci.image.layer.v1.tar+zstd")
  (data (i32.const 4544) "application/vnd.oci.image.layer.nondistributable.v1.tar+zstd")
  (data (i32.const 1024) "creating")
  (data (i32.const 1032) "created")
  (data (i32.const 1040) "running")
  (data (i32.const 1048) "stopped")
  (data (i32.const 1056) "deleted")
