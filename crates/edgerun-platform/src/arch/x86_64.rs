//! x86_64 LAPIC (Local APIC) implementation

#![allow(unsafe_op_in_unsafe_fn)]

use core::sync::atomic::{AtomicPtr, Ordering};

const MSR_EFER: u32 = 0xC000_0080;
const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_SFMASK: u32 = 0xC000_0084;
const EFER_SCE: u64 = 1;
const DEFAULT_SYSCALL_RFLAGS_MASK: u64 = 0x47700;
const DEFAULT_USER_RFLAGS: u64 = 0x202;

pub const KERNEL_CODE_SELECTOR: u16 = 0x08;
pub const KERNEL_DATA_SELECTOR: u16 = 0x10;
pub const USER_COMPAT_CODE_SELECTOR: u16 = 0x1b;
pub const USER_DATA_SELECTOR: u16 = 0x23;
pub const USER_CODE_SELECTOR: u16 = 0x2b;

const FLAT_GDT: [u64; 6] = [
    0,
    0x00af9a000000ffff,
    0x00cf92000000ffff,
    0x00cffa000000ffff,
    0x00cff2000000ffff,
    0x00affa000000ffff,
];

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SyscallFrame {
    pub rax: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub r10: u64,
    pub r8: u64,
    pub r9: u64,
    pub user_rip: u64,
    pub user_rflags: u64,
}

pub type SyscallHandler = extern "C" fn(&mut SyscallFrame);

static SYSCALL_HANDLER: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

core::arch::global_asm!(
    r#"
    .section .text.edgerun_syscall,"ax"
    .global edgerun_x86_64_reload_segments
edgerun_x86_64_reload_segments:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    push 0x08
    lea rax, [rip + 2f]
    push rax
    retfq
2:
    ret

    .global edgerun_x86_64_syscall_entry
edgerun_x86_64_syscall_entry:
    cld
    sub rsp, 72
    mov [rsp + 0], rax
    mov [rsp + 8], rdi
    mov [rsp + 16], rsi
    mov [rsp + 24], rdx
    mov [rsp + 32], r10
    mov [rsp + 40], r8
    mov [rsp + 48], r9
    mov [rsp + 56], rcx
    mov [rsp + 64], r11
    mov rdi, rsp
    call edgerun_x86_64_dispatch_syscall
    mov rax, [rsp + 0]
    mov rcx, [rsp + 56]
    mov r11, [rsp + 64]
    add rsp, 72
    sysretq
"#
);

unsafe extern "C" {
    fn edgerun_x86_64_reload_segments();
    fn edgerun_x86_64_syscall_entry();
}

/// Send IPI to a specific CPU or broadcast (cpu=0xFF)
#[inline]
pub fn send_ipi(cpu: u8, vector: u16) {
    let icr = ((cpu as u32) << 24) | (vector as u32) | (1 << 14);
    unsafe {
        core::arch::asm!(
            "mov dx, 0xFEE00030",
            "mov eax, eax",
            "out dx, eax",
            "2: mov dx, 0xFEE00030",
            "in eax, dx",
            "and eax, 0x1000",
            "jnz 2b",
            in("eax") icr,
            out("dx") _,
        );
    }
}

/// Read LAPIC ID
#[inline]
pub fn id() -> u8 {
    unsafe {
        let id: u32;
        core::arch::asm!(
            "mov eax, 0",
            "mov dx, 0xFEE00030",
            "out dx, eax",
            "mov dx, 0xFEE00034",
            "in eax, dx",
            out("rax") id,
            out("dx") _,
        );
        ((id >> 24) & 0xFF) as u8
    }
}

/// End Of Interrupt
#[inline]
pub fn eoi() {
    unsafe {
        core::arch::asm!("mov dx, 0xFEE000B0", "mov eax, 0", "out dx, eax");
    }
}

/// Enable LAPIC
#[inline]
pub fn enable() {
    unsafe {
        core::arch::asm!("mov dx, 0xFEE000F0", "mov eax, 0x1FF", "out dx, eax");
    }
}

/// Disable LAPIC
#[inline]
pub fn disable() {
    unsafe {
        core::arch::asm!("mov dx, 0xFEE000F0", "mov eax, 0", "out dx, eax");
    }
}

/// Get current TSC value
#[inline]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        core::arch::asm!(
            "rdtsc",
            out("rax") lo,
            out("rdx") hi,
        );
    }
    ((hi as u64) << 32) | (lo as u64)
}

#[inline]
pub unsafe fn read_msr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | low as u64
}

#[inline]
pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags),
        );
    }
}

/// Load Edgerun's flat kernel/user GDT and reload visible segment registers.
///
/// The GDT layout is chosen to satisfy the x86_64 `syscall/sysret` selector
/// arithmetic: `USER_COMPAT_CODE_SELECTOR + 16` is the 64-bit user code selector
/// and `USER_COMPAT_CODE_SELECTOR + 8` is the user data selector.
///
/// # Safety
///
/// The caller must run in long mode on x86_64 and ensure no concurrently running
/// CPU still depends on a different GDT layout.
pub unsafe fn load_flat_gdt() {
    let pointer = DescriptorTablePointer {
        limit: (core::mem::size_of_val(&FLAT_GDT) - 1) as u16,
        base: FLAT_GDT.as_ptr() as u64,
    };
    unsafe {
        core::arch::asm!("lgdt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
        edgerun_x86_64_reload_segments();
    }
}

pub fn set_syscall_handler(handler: SyscallHandler) {
    SYSCALL_HANDLER.store(handler as *mut (), Ordering::Release);
}

pub fn clear_syscall_handler() {
    SYSCALL_HANDLER.store(core::ptr::null_mut(), Ordering::Release);
}

/// Enable the x86_64 `syscall` entry path.
///
/// # Safety
///
/// The caller must provide valid GDT selectors and ensure the loaded program's
/// privilege/stack model is compatible with `syscall/sysret`. This function only
/// programs MSRs and does not create a TSS, user stack switch, or page tables.
pub unsafe fn enable_syscall_entry(kernel_code_selector: u16, user_code_selector: u16) {
    unsafe {
        enable_syscall_entry_with_mask(
            kernel_code_selector,
            user_code_selector,
            DEFAULT_SYSCALL_RFLAGS_MASK,
        );
    }
}

/// Same as [`enable_syscall_entry`], with an explicit RFLAGS mask for `syscall`.
///
/// # Safety
///
/// Same requirements as [`enable_syscall_entry`].
pub unsafe fn enable_syscall_entry_with_mask(
    kernel_code_selector: u16,
    user_code_selector: u16,
    rflags_mask: u64,
) {
    let star = ((user_code_selector as u64) << 48) | ((kernel_code_selector as u64) << 32);
    unsafe {
        write_msr(MSR_STAR, star);
        write_msr(
            MSR_LSTAR,
            edgerun_x86_64_syscall_entry as *const () as usize as u64,
        );
        write_msr(MSR_SFMASK, rflags_mask);
        write_msr(MSR_EFER, read_msr(MSR_EFER) | EFER_SCE);
    }
}

/// Enter 64-bit ring-3 code through `iretq` with interrupts enabled.
///
/// # Safety
///
/// The caller must ensure the flat GDT is loaded, `entry_point` is executable
/// user memory, `stack_pointer` is a valid user stack, and syscall/exception
/// handling is installed before transfer.
pub unsafe fn enter_user64(entry_point: u64, stack_pointer: u64) -> ! {
    unsafe { enter_user64_with_flags(entry_point, stack_pointer, DEFAULT_USER_RFLAGS) }
}

/// Same as [`enter_user64`], with explicit initial RFLAGS.
///
/// # Safety
///
/// Same requirements as [`enter_user64`].
pub unsafe fn enter_user64_with_flags(entry_point: u64, stack_pointer: u64, rflags: u64) -> ! {
    unsafe {
        core::arch::asm!(
            "mov ax, {user_data}",
            "mov ds, ax",
            "mov es, ax",
            "push {user_data}",
            "push {stack}",
            "push {rflags}",
            "push {user_code}",
            "push {entry}",
            "iretq",
            user_data = const USER_DATA_SELECTOR,
            user_code = const USER_CODE_SELECTOR,
            stack = in(reg) stack_pointer,
            rflags = in(reg) rflags,
            entry = in(reg) entry_point,
            options(noreturn)
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn edgerun_x86_64_dispatch_syscall(frame: &mut SyscallFrame) {
    let ptr = SYSCALL_HANDLER.load(Ordering::Acquire);
    if ptr.is_null() {
        frame.rax = u64::MAX;
        return;
    }
    let handler: SyscallHandler = unsafe { core::mem::transmute(ptr) };
    handler(frame);
}
