//! Shared Linux name catalogs used by validation and host setup.

pub(crate) const LINUX_CAPABILITIES: &[(&str, u32)] = &[
    ("CAP_CHOWN", 0),
    ("CAP_DAC_OVERRIDE", 1),
    ("CAP_DAC_READ_SEARCH", 2),
    ("CAP_FOWNER", 3),
    ("CAP_FSETID", 4),
    ("CAP_KILL", 5),
    ("CAP_SETGID", 6),
    ("CAP_SETUID", 7),
    ("CAP_SETPCAP", 8),
    ("CAP_LINUX_IMMUTABLE", 9),
    ("CAP_NET_BIND_SERVICE", 10),
    ("CAP_NET_BROADCAST", 11),
    ("CAP_NET_ADMIN", 12),
    ("CAP_NET_RAW", 13),
    ("CAP_IPC_LOCK", 14),
    ("CAP_IPC_OWNER", 15),
    ("CAP_SYS_MODULE", 16),
    ("CAP_SYS_RAWIO", 17),
    ("CAP_SYS_CHROOT", 18),
    ("CAP_SYS_PTRACE", 19),
    ("CAP_SYS_PACCT", 20),
    ("CAP_SYS_ADMIN", 21),
    ("CAP_SYS_BOOT", 22),
    ("CAP_SYS_NICE", 23),
    ("CAP_SYS_RESOURCE", 24),
    ("CAP_SYS_TIME", 25),
    ("CAP_SYS_TTY_CONFIG", 26),
    ("CAP_MKNOD", 27),
    ("CAP_LEASE", 28),
    ("CAP_AUDIT_WRITE", 29),
    ("CAP_AUDIT_CONTROL", 30),
    ("CAP_SETFCAP", 31),
    ("CAP_MAC_OVERRIDE", 32),
    ("CAP_MAC_ADMIN", 33),
    ("CAP_SYSLOG", 34),
    ("CAP_WAKE_ALARM", 35),
    ("CAP_BLOCK_SUSPEND", 36),
    ("CAP_AUDIT_READ", 37),
    ("CAP_PERFMON", 38),
    ("CAP_BPF", 39),
    ("CAP_CHECKPOINT_RESTORE", 40),
];

pub(crate) const LINUX_RLIMITS: &[(&str, u32)] = &[
    ("RLIMIT_CPU", 0),
    ("RLIMIT_FSIZE", 1),
    ("RLIMIT_DATA", 2),
    ("RLIMIT_STACK", 3),
    ("RLIMIT_CORE", 4),
    ("RLIMIT_RSS", 5),
    ("RLIMIT_NPROC", 6),
    ("RLIMIT_NOFILE", 7),
    ("RLIMIT_MEMLOCK", 8),
    ("RLIMIT_AS", 9),
    ("RLIMIT_LOCKS", 10),
    ("RLIMIT_SIGPENDING", 11),
    ("RLIMIT_MSGQUEUE", 12),
    ("RLIMIT_NICE", 13),
    ("RLIMIT_RTPRIO", 14),
    ("RLIMIT_RTTIME", 15),
];

pub(crate) const SECCOMP_ARCHES: &[(&str, u32)] = &[
    ("SCMP_ARCH_X86", 0x40000003),
    ("SCMP_ARCH_X86_64", 0xc000003e),
    ("SCMP_ARCH_X32", 0x4000003e),
    ("SCMP_ARCH_ARM", 0x40000028),
    ("SCMP_ARCH_AARCH64", 0xc00000b7),
    ("SCMP_ARCH_MIPS", 0x00000008),
    ("SCMP_ARCH_MIPS64", 0x80000008),
    ("SCMP_ARCH_MIPS64N32", 0xa0000008),
    ("SCMP_ARCH_MIPSEL", 0x40000008),
    ("SCMP_ARCH_MIPSEL64", 0xc0000008),
    ("SCMP_ARCH_MIPSEL64N32", 0xe0000008),
    ("SCMP_ARCH_PPC", 0x00000014),
    ("SCMP_ARCH_PPC64", 0x80000015),
    ("SCMP_ARCH_PPC64LE", 0xc0000015),
    ("SCMP_ARCH_S390", 0x00000016),
    ("SCMP_ARCH_S390X", 0x80000016),
    ("SCMP_ARCH_RISCV64", 0xc00000f3),
];

pub(crate) const LINUX_NAMESPACES: &[(&str, Option<i32>)] = &[
    ("mount", Some(0x00020000)),
    ("cgroup", Some(0x02000000)),
    ("uts", Some(0x04000000)),
    ("ipc", Some(0x08000000)),
    ("user", Some(0x10000000)),
    ("pid", Some(0x20000000)),
    ("network", Some(0x40000000)),
    ("time", None),
];

pub(crate) fn capability_number(name: &str) -> Option<u32> {
    named_linux_value(name, "CAP_", LINUX_CAPABILITIES)
}

pub(crate) fn is_oci_capability_name(name: &str) -> bool {
    has_exact_name(name, LINUX_CAPABILITIES)
}

pub(crate) fn rlimit_number(name: &str) -> Option<u32> {
    named_linux_value(name, "RLIMIT_", LINUX_RLIMITS)
}

pub(crate) fn is_oci_rlimit_name(name: &str) -> bool {
    has_exact_name(name, LINUX_RLIMITS)
}

pub(crate) fn seccomp_arch_bpf(name: &str) -> Option<u32> {
    named_linux_value(name, "SCMP_ARCH_", SECCOMP_ARCHES)
}

pub(crate) fn is_seccomp_arch_name(name: &str) -> bool {
    has_exact_name(name, SECCOMP_ARCHES)
}

pub(crate) fn namespace_flag(name: &str) -> Option<i32> {
    LINUX_NAMESPACES
        .iter()
        .find_map(|(known_name, flag)| (*known_name == name).then_some(*flag))
        .flatten()
}

pub(crate) fn is_oci_namespace_name(name: &str) -> bool {
    LINUX_NAMESPACES
        .iter()
        .any(|(known_name, _)| *known_name == name)
}

fn named_linux_value(name: &str, prefix: &str, values: &[(&str, u32)]) -> Option<u32> {
    values.iter().find_map(|(known_name, value)| {
        if *known_name == name || known_name.strip_prefix(prefix) == Some(name) {
            Some(*value)
        } else {
            None
        }
    })
}

fn has_exact_name(name: &str, values: &[(&str, u32)]) -> bool {
    values.iter().any(|(known_name, _)| *known_name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_number_accepts_full_and_kernel_names() {
        assert_eq!(capability_number("CAP_NET_ADMIN"), Some(12));
        assert_eq!(capability_number("NET_ADMIN"), Some(12));
    }

    #[test]
    fn rlimit_number_accepts_full_and_kernel_names() {
        assert_eq!(rlimit_number("RLIMIT_NOFILE"), Some(7));
        assert_eq!(rlimit_number("NOFILE"), Some(7));
    }

    #[test]
    fn oci_names_require_oci_prefixes() {
        assert!(is_oci_capability_name("CAP_NET_ADMIN"));
        assert!(!is_oci_capability_name("NET_ADMIN"));
        assert!(is_oci_rlimit_name("RLIMIT_NOFILE"));
        assert!(!is_oci_rlimit_name("NOFILE"));
    }

    #[test]
    fn seccomp_arches_include_validated_names() {
        assert_eq!(seccomp_arch_bpf("SCMP_ARCH_X86_64"), Some(0xc000003e));
        assert_eq!(seccomp_arch_bpf("X86_64"), Some(0xc000003e));
        assert!(is_seccomp_arch_name("SCMP_ARCH_RISCV64"));
        assert!(!is_seccomp_arch_name("RISCV64"));
    }

    #[test]
    fn namespace_catalog_tracks_known_names_and_clone_flags() {
        assert_eq!(namespace_flag("pid"), Some(0x20000000));
        assert_eq!(namespace_flag("time"), None);
        assert!(is_oci_namespace_name("time"));
        assert!(!is_oci_namespace_name("mnt"));
    }
}
