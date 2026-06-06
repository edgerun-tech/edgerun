(module
  (import "edgerun-core" "memory" (memory 1))
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

  (func (export "proto_standard_id") (result i32)
    i32.const 300093)

  (func $m143b (param $ptr i32) (param $off i32) (result i32)
    (i32.load8_u (i32.add (local.get $ptr) (local.get $off))))

  (func $m143eq_lit (param $ptr i32) (param $len i32) (param $lit i32) (param $lit_len i32) (result i32)
    (local $i i32)
    (if (i32.ne (local.get $len) (local.get $lit_len)) (then (return (i32.const 0))))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.ne
              (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))
              (i32.load8_u (i32.add (local.get $lit) (local.get $i))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    i32.const 1)

  (func (export "oci_status_valid") (param $status i32) (result i32)
    (i32.le_u (local.get $status) (i32.const 4)))

  (func (export "oci_status_terminal") (param $status i32) (result i32)
    (i32.or
      (i32.eq (local.get $status) (i32.const 3))
      (i32.eq (local.get $status) (i32.const 4))))

  (func (export "oci_status_code") (param $ptr i32) (param $len i32) (result i32)
    (if (call $m143eq_lit (local.get $ptr) (local.get $len) (i32.const 1024) (i32.const 8))
      (then (return (i32.const 0))))
    (if (call $m143eq_lit (local.get $ptr) (local.get $len) (i32.const 1032) (i32.const 7))
      (then (return (i32.const 1))))
    (if (call $m143eq_lit (local.get $ptr) (local.get $len) (i32.const 1040) (i32.const 7))
      (then (return (i32.const 2))))
    (if (call $m143eq_lit (local.get $ptr) (local.get $len) (i32.const 1048) (i32.const 7))
      (then (return (i32.const 3))))
    (if (call $m143eq_lit (local.get $ptr) (local.get $len) (i32.const 1056) (i32.const 7))
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
    (if (i32.or (i32.or (i32.eq (local.get $nr) (i32.const 272)) (i32.eq (local.get $nr) (i32.const 308)))
                (i32.or (i32.eq (local.get $nr) (i32.const 170)) (i32.eq (local.get $nr) (i32.const 171))))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (i32.const 165)) (i32.eq (local.get $nr) (i32.const 155)))
            (i32.or (i32.eq (local.get $nr) (i32.const 166)) (i32.eq (local.get $nr) (i32.const 428))))
          (i32.or (i32.eq (local.get $nr) (i32.const 429)) (i32.eq (local.get $nr) (i32.const 442))))
      (then (return (i32.const 2))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (i32.const 317)) (i32.eq (local.get $nr) (i32.const 126)))
            (i32.or (i32.eq (local.get $nr) (i32.const 157)) (i32.eq (local.get $nr) (i32.const 105))))
          (i32.or (i32.eq (local.get $nr) (i32.const 106)) (i32.eq (local.get $nr) (i32.const 116))))
      (then (return (i32.const 3))))
    (if (i32.or (i32.or (i32.eq (local.get $nr) (i32.const 302)) (i32.eq (local.get $nr) (i32.const 314)))
                (i32.or (i32.eq (local.get $nr) (i32.const 251)) (i32.eq (local.get $nr) (i32.const 95))))
      (then (return (i32.const 4))))
    (if (i32.or
          (i32.or (i32.eq (local.get $nr) (i32.const 62)) (i32.eq (local.get $nr) (i32.const 61)))
          (i32.or (i32.eq (local.get $nr) (i32.const 59)) (i32.eq (local.get $nr) (i32.const 56))))
      (then (return (i32.const 5))))
    (if (i32.eq (local.get $nr) (i32.const 321)) (then (return (i32.const 6))))
    i32.const 0)

  (func $aarch64_syscall_class (param $nr i32) (result i32)
    (if (i32.or (i32.or (i32.eq (local.get $nr) (i32.const 97)) (i32.eq (local.get $nr) (i32.const 268)))
                (i32.or (i32.eq (local.get $nr) (i32.const 161)) (i32.eq (local.get $nr) (i32.const 162))))
      (then (return (i32.const 1))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (i32.const 40)) (i32.eq (local.get $nr) (i32.const 41)))
            (i32.or (i32.eq (local.get $nr) (i32.const 39)) (i32.eq (local.get $nr) (i32.const 428))))
          (i32.or (i32.eq (local.get $nr) (i32.const 429)) (i32.eq (local.get $nr) (i32.const 442))))
      (then (return (i32.const 2))))
    (if (i32.or
          (i32.or
            (i32.or (i32.eq (local.get $nr) (i32.const 277)) (i32.eq (local.get $nr) (i32.const 94)))
            (i32.or (i32.eq (local.get $nr) (i32.const 167)) (i32.eq (local.get $nr) (i32.const 146))))
          (i32.or (i32.eq (local.get $nr) (i32.const 144)) (i32.eq (local.get $nr) (i32.const 159))))
      (then (return (i32.const 3))))
    (if (i32.or (i32.or (i32.eq (local.get $nr) (i32.const 267)) (i32.eq (local.get $nr) (i32.const 274)))
                (i32.or (i32.eq (local.get $nr) (i32.const 31)) (i32.eq (local.get $nr) (i32.const 166))))
      (then (return (i32.const 4))))
    (if (i32.or
          (i32.or (i32.eq (local.get $nr) (i32.const 129)) (i32.eq (local.get $nr) (i32.const 260)))
          (i32.or (i32.eq (local.get $nr) (i32.const 221)) (i32.eq (local.get $nr) (i32.const 220))))
      (then (return (i32.const 5))))
    (if (i32.eq (local.get $nr) (i32.const 280)) (then (return (i32.const 6))))
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

  (data (i32.const 1024) "creating")
  (data (i32.const 1032) "created")
  (data (i32.const 1040) "running")
  (data (i32.const 1048) "stopped")
  (data (i32.const 1056) "deleted")
)
