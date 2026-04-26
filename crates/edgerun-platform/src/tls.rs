//! Thread-Local Storage area - per-CPU data

#![allow(unsafe_op_in_unsafe_fn)]

/// TLS area - typically 128 bytes
#[repr(C)]
pub struct TlsArea {
    pub data: [u8; 128],
}

impl TlsArea {
    pub const INIT: Self = Self { data: [0; 128] };
    
    pub unsafe fn set_per_cpu(&mut self, ptr: usize) {
        self.data[..8].copy_from_slice(&ptr.to_le_bytes());
    }
    
    pub fn per_cpu(&self) -> usize {
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.data[..8]);
        usize::from_le_bytes(bytes)
    }
}

/// Initialize TLS for a new thread/CPU
pub unsafe fn tls_init(_cpu_id: usize, per_cpu_ptr: usize) {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!("mov fs:0, rax", in("rax") per_cpu_ptr as u64);
    
    #[cfg(target_arch = "aarch64")]
    core::arch::asm!("msr TPIDR_EL0, rax", in("rax") per_cpu_ptr as u64);
    
    #[cfg(target_arch = "riscv64")]
    core::arch::asm!("csrw tp, rax", in("rax") per_cpu_ptr as u64);
    
    #[cfg(target_arch = "xtensa")]
    core::arch::asm!("wsr 0, eax", in("eax") per_cpu_ptr as u32);
}