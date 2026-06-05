#!/usr/bin/env node

const assert = require("assert");
const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "../..");
const wat = path.join(root, "standards/build/wasm/device-primitives/platform-core.wat");
const wasm = path.join(os.tmpdir(), `platform-core-${process.pid}.wasm`);

execFileSync("wat2wasm", [wat, "-o", wasm], { stdio: "pipe" });

(async () => {
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasm), {});
  const e = instance.exports;

  assert.strictEqual(e.platform_core_abi_version(), 1);
  assert.strictEqual(e.platform_max_cpus(), 64);
  assert.strictEqual(e.platform_timer_freq(), 1000000n);
  assert.strictEqual(e.platform_irq_reschedule(), 0xfee0);
  assert.strictEqual(e.cpu_id_index(257), 1);
  assert.strictEqual(e.cpu_count_after_smp_init(1), 1);
  assert.strictEqual(e.cpu_count_after_smp_init(4), 4);
  assert.strictEqual(e.timer_ticks_to_us(42n), 42n);
  assert.strictEqual(e.timer_us_to_ticks(42n), 42n);
  assert.strictEqual(e.monotime_elapsed(100n, 40n), 60n);
  assert.strictEqual(e.align_up(0x200001, 4096), 0x201000);
  assert.strictEqual(e.allocator_heap_min_start(0), 0x200000);
  assert.strictEqual(e.allocator_heap_min_start(1), 0);
  assert.strictEqual(e.allocator_heap_end(0), 0x1000000);
  assert.strictEqual(e.allocator_alloc_result(8, 8, 0x200000, 0x200100), 0);
  assert.strictEqual(e.allocator_alloc_result(0, 8, 0x200000, 0x200100), 1);
  assert.strictEqual(e.allocator_alloc_result(0x200, 8, 0x200000, 0x200100), 2);

  assert.strictEqual(e.tls_area_size(), 128);
  assert.strictEqual(e.tls_per_cpu_roundtrip(0x1234), 0x1234);
  assert.strictEqual(e.irq_disable_result(1, 0x202), 0x202);
  assert.strictEqual(e.irq_disable_result(0, 0x202), 0);
  assert.strictEqual(e.ipi_target_code(0xff, 0xfee0), 0xfffee0);
  assert.strictEqual(e.waker_ipi_vector(), 0xfee0);

  assert.strictEqual(e.x86_selector_code(1), 0x08);
  assert.strictEqual(e.x86_selector_code(5), 0x2b);
  assert.strictEqual(e.x86_syscall_rflags_mask(), 0x47700n);
  assert.strictEqual(e.x86_default_user_rflags(), 0x202n);
  assert.strictEqual(e.x86_ipi_icr(2, 0xfee0), 0x0200fee0 | 0x4000);
  assert.strictEqual(e.x86_rdtsc_join(1, 2), 0x200000001n);

  assert.strictEqual(e.aarch64_gic_irq_reg_offset(1, 64), 0x108);
  assert.strictEqual(e.aarch64_gic_irq_reg_offset(2, 64), 0x188);
  assert.strictEqual(e.aarch64_sgi_value(2, 7), 0x02017000);
  assert.strictEqual(e.aarch64_cpu_id_from_mpidr(0x1200), 0x12);
  assert.strictEqual(e.riscv_msip_addr_offset(3), 12);
  assert.strictEqual(e.riscv_mtimecmp_offset(3), 0x4018);
  assert.strictEqual(e.riscv_interrupt_bit(1), 8n);
  assert.strictEqual(e.riscv_interrupt_bit(2), 128n);
  assert.strictEqual(e.xtensa_interrupt_mask(3), 8);
  assert.strictEqual(e.xtensa_level_clamp(99), 15);
  assert.strictEqual(e.xtensa_timer_next_deadline(5), 10005);
  assert.strictEqual(e.xtensa_usb_clock_bit(), 1024);
  assert.strictEqual(e.xtensa_usb_conf0_default(), 0x4200);

  console.log("platform core smoke passed");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
