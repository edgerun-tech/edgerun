;; Bare-metal platform semantics plundered from edgerun-platform.

  (import "math" "align_up" (func $align_up_internal (param i32 i32) (result i32)))

  (func (export "platform_core_abi_version") (result i32) i32.const 1)
  (func (export "platform_max_cpus") (result i32) i32.const 64)
  (func (export "platform_timer_freq") (result i64) i64.const 1000000)
  (func (export "platform_irq_timer") (result i32) i32.const 0)

  (func (export "platform_irq_reschedule") (result i32) i32.const 65248)

  (func (export "cpu_id_index") (param $id i32) (result i32)
    (i32.and (local.get $id) (i32.const 255)))

  (func (export "cpu_count_after_smp_init") (param $requested_count i32) (result i32)
    ;; smp_init returns true for <=1 and otherwise stores requested count.
    (if (i32.le_u (local.get $requested_count) (i32.const 1)) (then (return (i32.const 1))))
    (i32.and (local.get $requested_count) (i32.const 255)))

  (func (export "cpu_all_cpus_len") (param $cpu_count i32) (result i32)
    (i32.and (local.get $cpu_count) (i32.const 255)))

  (func (export "timer_ticks_to_us") (param $ticks i64) (result i64)
    ;; timer_freq is 1 MHz, so one tick is one microsecond.
    local.get $ticks)

  (func (export "timer_us_to_ticks") (param $us i64) (result i64)
    local.get $us)

  (func (export "monotime_elapsed") (param $now i64) (param $start i64) (result i64)
    (i64.sub (local.get $now) (local.get $start)))

  (func (export "align_up") (param $value i32) (param $align i32) (result i32)
    (call $align_up_internal (local.get $value) (local.get $align)))

  (func (export "allocator_heap_min_start") (param $is_xtensa i32) (result i32)
    (if (local.get $is_xtensa) (then (return (i32.const 0))))
    i32.const 2097152)

  (func (export "allocator_heap_end") (param $is_xtensa i32) (result i32)
    (if (local.get $is_xtensa) (then (return (i32.const 0x3fcef000))))
    i32.const 16777216)

  (func (export "allocator_alloc_result") (param $size i32) (param $align i32) (param $current i32) (param $heap_end i32) (result i32)
    ;; 0 ok, 1 zero-size/null, 2 out of heap.
    (local $aligned i32)
    (if (i32.eqz (local.get $size)) (then (return (i32.const 1))))
    (local.set $aligned (call $align_up_internal (local.get $current) (local.get $align)))
    (if (i32.gt_u (i32.add (local.get $aligned) (local.get $size)) (local.get $heap_end)) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "tls_area_size") (result i32) i32.const 128)
  (func (export "tls_per_cpu_roundtrip") (param $ptr i32) (result i32)
    local.get $ptr)

  (func (export "irq_disable_result") (param $is_x86_64 i32) (param $flags i32) (result i32)
    ;; x86 returns old flags; other arches return 0.
    (if (local.get $is_x86_64) (then (return (local.get $flags))))
    i32.const 0)

  (func (export "ipi_target_code") (param $target i32) (param $vector i32) (result i32)
    ;; Send IPI vector to target CPU; 0xff means broadcast.
    (i32.or (i32.shl (i32.and (local.get $target) (i32.const 255)) (i32.const 16)) (i32.and (local.get $vector) (i32.const 65535))))

  (func (export "waker_ipi_vector") (result i32) i32.const 65248)

  (func (export "x86_selector_code") (param $selector_kind i32) (result i32)
    ;; 1 kernel code, 2 kernel data, 3 compat user code, 4 user data, 5 user code.
    (if (i32.eq (local.get $selector_kind) (i32.const 1)) (then (return (i32.const 8))))
    (if (i32.eq (local.get $selector_kind) (i32.const 2)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $selector_kind) (i32.const 3)) (then (return (i32.const 27))))
    (if (i32.eq (local.get $selector_kind) (i32.const 4)) (then (return (i32.const 35))))
    (if (i32.eq (local.get $selector_kind) (i32.const 5)) (then (return (i32.const 43))))
    i32.const 0)

  (func (export "x86_syscall_rflags_mask") (result i64) i64.const 292608)
  (func (export "x86_default_user_rflags") (result i64) i64.const 514)
  (func (export "x86_ipi_icr") (param $cpu i32) (param $vector i32) (result i32)
    (i32.or (i32.shl (i32.and (local.get $cpu) (i32.const 255)) (i32.const 24)) (i32.or (i32.and (local.get $vector) (i32.const 65535)) (i32.const 16384))))

  (func (export "x86_rdtsc_join") (param $lo i32) (param $hi i32) (result i64)
    (i64.or (i64.shl (i64.extend_i32_u (local.get $hi)) (i64.const 32)) (i64.extend_i32_u (local.get $lo))))

  (func (export "aarch64_gic_irq_reg_offset") (param $base_kind i32) (param $irq i32) (result i32)
    ;; base_kind 1 enable, 2 disable. Offset adds (irq / 32) * 4.
    (local $base i32)
    (local.set $base (if (result i32) (i32.eq (local.get $base_kind) (i32.const 1)) (then i32.const 256) (else i32.const 384)))
    (i32.add (local.get $base) (i32.mul (i32.div_u (local.get $irq) (i32.const 32)) (i32.const 4))))

  (func (export "aarch64_sgi_value") (param $cpu i32) (param $irq i32) (result i32)
    (i32.or (i32.shl (i32.and (local.get $irq) (i32.const 255)) (i32.const 12)) (i32.or (i32.shl (i32.and (local.get $cpu) (i32.const 255)) (i32.const 24)) (i32.const 65536))))

  (func (export "aarch64_cpu_id_from_mpidr") (param $mpidr_low32 i32) (result i32)
    (i32.and (i32.shr_u (local.get $mpidr_low32) (i32.const 8)) (i32.const 255)))

  (func (export "riscv_msip_addr_offset") (param $hart i32) (result i32)
    (i32.mul (i32.and (local.get $hart) (i32.const 255)) (i32.const 4)))

  (func (export "riscv_mtimecmp_offset") (param $hart i32) (result i32)
    (i32.add (i32.const 16384) (i32.mul (i32.and (local.get $hart) (i32.const 255)) (i32.const 8))))

  (func (export "riscv_interrupt_bit") (param $kind i32) (result i64)
    ;; 1 software interrupt, 2 timer interrupt.
    (if (i32.eq (local.get $kind) (i32.const 1)) (then (return (i64.const 8))))
    (if (i32.eq (local.get $kind) (i32.const 2)) (then (return (i64.const 128))))
    i64.const 0)

  (func (export "xtensa_interrupt_mask") (param $level i32) (result i32)
    (i32.shl (i32.const 1) (i32.and (local.get $level) (i32.const 31))))

  (func (export "xtensa_level_clamp") (param $level i32) (result i32)
    (if (i32.gt_u (local.get $level) (i32.const 15)) (then (return (i32.const 15))))
    local.get $level)

  (func (export "xtensa_timer_next_deadline") (param $ccount i32) (result i32)
    (i32.add (local.get $ccount) (i32.const 10000)))

  (func (export "xtensa_usb_clock_bit") (result i32) i32.const 1024)
  (func (export "xtensa_usb_conf0_default") (result i32) i32.const 16896)
