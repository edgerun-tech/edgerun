//! Seccomp-BPF filtering for OCI containers.
//!
//! Supports both spec-driven seccomp rules from the OCI config
//! and a built-in fallback allow-list when no spec rules are provided.
//! Uses raw seccomp syscalls (no libseccomp dependency).

#![allow(dead_code)]

use std::io;
use std::os::raw::c_void;

use crate::json::{OciLinuxSeccomp, OciSeccompAction};
use crate::syscalls::{do_seccomp, SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC};

// ===========================================================================
// Architecture mapping
// ===========================================================================

#[cfg(target_arch = "x86_64")]
pub const AUDIT_ARCH_X86_64: u32 = 0xc000003e;
#[cfg(target_arch = "aarch64")]
pub const AUDIT_ARCH_AARCH64: u32 = 0xc00000b7;

#[cfg(target_arch = "x86_64")]
const CURRENT_ARCH: u32 = AUDIT_ARCH_X86_64;
#[cfg(target_arch = "aarch64")]
const CURRENT_ARCH: u32 = AUDIT_ARCH_AARCH64;

// ===========================================================================
// Syscall number mapping
// ===========================================================================

/// Resolve a syscall name to its number on the current architecture.
fn syscall_nr(name: &str) -> Option<u32> {
    #[cfg(target_arch = "x86_64")]
    return match name {
        "read" => Some(0), "write" => Some(1), "open" => Some(2), "close" => Some(3),
        "stat" => Some(4), "fstat" => Some(5), "lstat" => Some(6), "poll" => Some(7),
        "lseek" => Some(8), "mmap" => Some(9), "mprotect" => Some(10), "munmap" => Some(11),
        "brk" => Some(12), "rt_sigaction" => Some(13), "rt_sigprocmask" => Some(14),
        "rt_sigreturn" => Some(15), "ioctl" => Some(16), "pread64" => Some(17),
        "pwrite64" => Some(18), "readv" => Some(19), "writev" => Some(20),
        "access" => Some(21), "pipe" => Some(22), "select" => Some(23),
        "sched_yield" => Some(24), "mremap" => Some(25), "msync" => Some(26),
        "mincore" => Some(27), "madvise" => Some(28), "shmget" => Some(29),
        "shmat" => Some(30), "shmctl" => Some(31), "dup" => Some(32),
        "dup2" => Some(33), "pause" => Some(34), "nanosleep" => Some(35),
        "getitimer" => Some(36), "alarm" => Some(37), "setitimer" => Some(38),
        "getpid" => Some(39), "sendfile" => Some(40), "socket" => Some(41),
        "connect" => Some(42), "accept" => Some(43), "sendto" => Some(44),
        "recvfrom" => Some(45), "sendmsg" => Some(46), "recvmsg" => Some(47),
        "shutdown" => Some(48), "bind" => Some(49), "listen" => Some(50),
        "getsockname" => Some(51), "getpeername" => Some(52), "socketpair" => Some(53),
        "setsockopt" => Some(54), "getsockopt" => Some(55), "clone" => Some(56),
        "fork" => Some(57), "vfork" => Some(58), "execve" => Some(59),
        "exit" => Some(60), "wait4" => Some(61), "kill" => Some(62),
        "uname" => Some(63), "semget" => Some(64), "semop" => Some(65),
        "semctl" => Some(66), "shmdt" => Some(68),
        "msgget" => Some(69), "msgsnd" => Some(70), "msgrcv" => Some(71),
        "fcntl" => Some(72), "flock" => Some(73),
        "fsync" => Some(74), "fdatasync" => Some(75), "truncate" => Some(76),
        "ftruncate" => Some(77), "getdents" => Some(78), "getcwd" => Some(79),
        "chdir" => Some(80), "fchdir" => Some(81), "rename" => Some(82),
        "mkdir" => Some(83), "rmdir" => Some(84), "creat" => Some(85),
        "link" => Some(86), "unlink" => Some(87), "symlink" => Some(88),
        "readlink" => Some(89), "chmod" => Some(90), "fchmod" => Some(91),
        "chown" => Some(92), "fchown" => Some(93), "lchown" => Some(94),
        "umask" => Some(95), "gettimeofday" => Some(96), "getrlimit" => Some(97),
        "getrusage" => Some(98), "sysinfo" => Some(99), "times" => Some(100),
        "getuid" => Some(102), "syslog" => Some(103), "getgid" => Some(104),
        "setuid" => Some(105), "setgid" => Some(106), "geteuid" => Some(107),
        "getegid" => Some(108), "setpgid" => Some(109), "getppid" => Some(110),
        "getpgrp" => Some(111), "setsid" => Some(112), "setreuid" => Some(113),
        "setregid" => Some(114), "getgroups" => Some(115), "setgroups" => Some(116),
        "setresuid" => Some(117), "getresuid" => Some(118), "setresgid" => Some(119),
        "getresgid" => Some(120), "getpgid" => Some(121), "setfsuid" => Some(122),
        "setfsgid" => Some(123), "getsid" => Some(124), "capget" => Some(125),
        "capset" => Some(126), "rt_sigpending" => Some(127),
        "rt_sigtimedwait" => Some(128), "rt_sigqueueinfo" => Some(129),
        "rt_sigsuspend" => Some(130), "sigaltstack" => Some(131),
        "utime" => Some(132), "mknod" => Some(133), "uselib" => Some(134),
        "personality" => Some(135), "ustat" => Some(136), "statfs" => Some(137),
        "fstatfs" => Some(138), "sysfs" => Some(139), "getpriority" => Some(140),
        "setpriority" => Some(141), "sched_setparam" => Some(142),
        "sched_getparam" => Some(143), "sched_setscheduler" => Some(144),
        "sched_getscheduler" => Some(145), "sched_get_priority_max" => Some(146),
        "sched_get_priority_min" => Some(147), "sched_rr_get_interval" => Some(148),
        "mlock" => Some(149), "munlock" => Some(150), "mlockall" => Some(151),
        "munlockall" => Some(152), "vhangup" => Some(153), "modify_ldt" => Some(154),
        "pivot_root" => Some(155), "_sysctl" => Some(156), "prctl" => Some(157),
        "arch_prctl" => Some(158), "adjtimex" => Some(159), "setrlimit" => Some(160),
        "chroot" => Some(161), "sync" => Some(162), "acct" => Some(163),
        "settimeofday" => Some(164), "mount" => Some(165), "umount2" => Some(166),
        "swapon" => Some(167), "swapoff" => Some(168), "reboot" => Some(169),
        "sethostname" => Some(170), "setdomainname" => Some(171),
        "iopl" => Some(172), "ioperm" => Some(173), "create_module" => Some(174),
        "init_module" => Some(175), "delete_module" => Some(176),
        "get_kernel_syms" => Some(177), "query_module" => Some(178),
        "quotactl" => Some(179), "nfsservctl" => Some(180),
        "getpmsg" => Some(181), "putpmsg" => Some(182), "afs_syscall" => Some(183),
        "tuxcall" => Some(184), "security" => Some(185), "gettid" => Some(186),
        "readahead" => Some(187), "setxattr" => Some(188), "lsetxattr" => Some(189),
        "fsetxattr" => Some(190), "getxattr" => Some(191), "lgetxattr" => Some(192),
        "fgetxattr" => Some(193), "listxattr" => Some(194), "llistxattr" => Some(195),
        "flistxattr" => Some(196), "removexattr" => Some(197), "lremovexattr" => Some(198),
        "fremovexattr" => Some(199), "tkill" => Some(200),
        "time" => Some(201), "futex" => Some(202), "sched_setaffinity" => Some(203),
        "sched_getaffinity" => Some(204), "set_thread_area" => Some(205),
        "io_setup" => Some(206), "io_destroy" => Some(207), "io_getevents" => Some(208),
        "io_submit" => Some(209), "io_cancel" => Some(210),
        "get_thread_area" => Some(211), "epoll_create" => Some(213),
        "set_tid_address" => Some(218), "restart_syscall" => Some(219),
        "semtimedop" => Some(220), "fadvise64" => Some(221),
        "clock_gettime" => Some(228), "clock_getres" => Some(229),
        "clock_nanosleep" => Some(230), "exit_group" => Some(231),
        "epoll_wait" => Some(232), "epoll_ctl" => Some(233),
        "tgkill" => Some(234), "utimes" => Some(235),
        "mbind" => Some(237), "set_mempolicy" => Some(238),
        "get_mempolicy" => Some(239), "mq_open" => Some(240),
        "mq_unlink" => Some(241), "mq_timedsend" => Some(242),
        "mq_timedreceive" => Some(243), "mq_notify" => Some(244),
        "mq_getsetattr" => Some(245), "kexec_load" => Some(246),
        "waitid" => Some(247), "add_key" => Some(248), "request_key" => Some(249),
        "keyctl" => Some(250), "ioprio_set" => Some(251), "ioprio_get" => Some(252),
        "inotify_init" => Some(253), "inotify_add_watch" => Some(254),
        "inotify_rm_watch" => Some(255), "migrate_pages" => Some(256),
        "openat" => Some(257), "mkdirat" => Some(258), "mknodat" => Some(259),
        "fchownat" => Some(260), "futimesat" => Some(261), "newfstatat" => Some(262),
        "unlinkat" => Some(263), "renameat" => Some(264), "linkat" => Some(265),
        "symlinkat" => Some(266), "readlinkat" => Some(267), "fchmodat" => Some(268),
        "faccessat" => Some(269), "pselect6" => Some(270), "ppoll" => Some(271),
        "unshare" => Some(272), "set_robust_list" => Some(273),
        "get_robust_list" => Some(274), "splice" => Some(275),
        "tee" => Some(276), "sync_file_range" => Some(277), "vmsplice" => Some(278),
        "move_pages" => Some(279), "utimensat" => Some(280), "epoll_pwait" => Some(281),
        "signalfd" => Some(282), "timerfd_create" => Some(283), "eventfd" => Some(284),
        "fallocate" => Some(285), "timerfd_settime" => Some(286),
        "timerfd_gettime" => Some(287), "accept4" => Some(288),
        "signalfd4" => Some(289), "eventfd2" => Some(290), "epoll_create1" => Some(291),
        "dup3" => Some(292), "pipe2" => Some(293), "inotify_init1" => Some(294),
        "preadv" => Some(295), "pwritev" => Some(296), "rt_tgsigqueueinfo" => Some(297),
        "perf_event_open" => Some(298), "recvmmsg" => Some(299),
        "fanotify_init" => Some(300), "fanotify_mark" => Some(301),
        "prlimit64" => Some(302), "name_to_handle_at" => Some(303),
        "open_by_handle_at" => Some(304), "clock_adjtime" => Some(305),
        "syncfs" => Some(306), "sendmmsg" => Some(307), "setns" => Some(308),
        "getcpu" => Some(309), "process_vm_readv" => Some(310),
        "process_vm_writev" => Some(311), "kcmp" => Some(312),
        "finit_module" => Some(313), "sched_setattr" => Some(314),
        "sched_getattr" => Some(315), "renameat2" => Some(316),
        "seccomp" => Some(317), "getrandom" => Some(318),
        "memfd_create" => Some(319), "kexec_file_load" => Some(320),
        "bpf" => Some(321), "execveat" => Some(322), "userfaultfd" => Some(323),
        "membarrier" => Some(324), "mlock2" => Some(325), "copy_file_range" => Some(326),
        "preadv2" => Some(327), "pwritev2" => Some(328), "pkey_mprotect" => Some(329),
        "pkey_alloc" => Some(330), "pkey_free" => Some(331), "statx" => Some(332),
        "io_pgetevents" => Some(333), "rseq" => Some(334),
        "pidfd_send_signal" => Some(424), "io_uring_setup" => Some(425),
        "io_uring_enter" => Some(426), "io_uring_register" => Some(427),
        "open_tree" => Some(428), "move_mount" => Some(429),
        "fsopen" => Some(430), "fsconfig" => Some(431), "fsmount" => Some(432),
        "fspick" => Some(433), "pidfd_open" => Some(434),
        "clone3" => Some(435), "close_range" => Some(436),
        "openat2" => Some(437), "pidfd_getfd" => Some(438),
        "faccessat2" => Some(439), "process_madvise" => Some(440),
        "epoll_pwait2" => Some(441), "mount_setattr" => Some(442),
        "quotactl_fd" => Some(443), "landlock_create_ruleset" => Some(444),
        "landlock_add_rule" => Some(445), "landlock_restrict_self" => Some(446),
        "memfd_secret" => Some(447), "process_mrelease" => Some(448),
        "futex_waitv" => Some(449), "set_mempolicy_home_node" => Some(450),
        "fchmodat2" => Some(452), "map_shadow_stack" => Some(453),
        _ => None,
    };

    #[cfg(target_arch = "aarch64")]
    return match name {
        "io_setup" => Some(0), "io_destroy" => Some(1), "io_submit" => Some(2),
        "io_cancel" => Some(3), "io_getevents" => Some(4), "setxattr" => Some(5),
        "lsetxattr" => Some(6), "fsetxattr" => Some(7), "getxattr" => Some(8),
        "lgetxattr" => Some(9), "fgetxattr" => Some(10), "listxattr" => Some(11),
        "llistxattr" => Some(12), "flistxattr" => Some(13), "removexattr" => Some(14),
        "lremovexattr" => Some(15), "fremovexattr" => Some(16), "getcwd" => Some(17),
        "lookup_dcookie" => Some(18), "eventfd2" => Some(19), "epoll_create1" => Some(20),
        "epoll_ctl" => Some(21), "epoll_pwait" => Some(22), "dup" => Some(23),
        "dup3" => Some(24), "fcntl" => Some(25), "inotify_init1" => Some(26),
        "inotify_add_watch" => Some(27), "inotify_rm_watch" => Some(28),
        "ioctl" => Some(29), "ioprio_set" => Some(30), "ioprio_get" => Some(31),
        "flock" => Some(32), "mknodat" => Some(33), "mkdirat" => Some(34),
        "unlinkat" => Some(35), "symlinkat" => Some(36), "linkat" => Some(37),
        "renameat" => Some(38), "umount2" => Some(39), "mount" => Some(40),
        "pivot_root" => Some(41), "nfsservctl" => Some(42), "statfs" => Some(43),
        "fstatfs" => Some(44), "truncate" => Some(45), "ftruncate" => Some(46),
        "fallocate" => Some(47), "faccessat" => Some(48), "chdir" => Some(49),
        "fchdir" => Some(50), "chroot" => Some(51), "fchmod" => Some(52),
        "fchmodat" => Some(53), "fchownat" => Some(54), "fchown" => Some(55),
        "openat" => Some(56), "close" => Some(57), "vhangup" => Some(58),
        "openat2" => Some(59), "pipe2" => Some(59), "quotactl" => Some(60),
        "getdents64" => Some(61), "lseek" => Some(62), "read" => Some(63),
        "write" => Some(64), "readv" => Some(65), "writev" => Some(66),
        "pread64" => Some(67), "pwrite64" => Some(68), "preadv" => Some(69),
        "pwritev" => Some(70), "sendfile" => Some(71), "pselect6" => Some(72),
        "ppoll" => Some(73), "signalfd4" => Some(74), "vmsplice" => Some(75),
        "splice" => Some(76), "tee" => Some(77), "readlinkat" => Some(78),
        "newfstatat" => Some(79), "fstat" => Some(80), "sync" => Some(81),
        "fsync" => Some(82), "fdatasync" => Some(83), "sync_file_range" => Some(84),
        "timerfd_create" => Some(85), "timerfd_settime" => Some(86),
        "timerfd_gettime" => Some(87), "utimensat" => Some(88), "acct" => Some(89),
        "capget" => Some(90), "capset" => Some(91), "personality" => Some(92),
        "exit" => Some(93), "exit_group" => Some(94), "waitid" => Some(95),
        "set_tid_address" => Some(96), "unshare" => Some(97),
        "futex" => Some(98), "set_robust_list" => Some(99),
        "get_robust_list" => Some(100), "nanosleep" => Some(101),
        "getitimer" => Some(102), "setitimer" => Some(103),
        "kexec_load" => Some(104), "init_module" => Some(105),
        "delete_module" => Some(106), "timer_create" => Some(107),
        "timer_gettime" => Some(108), "timer_getoverrun" => Some(109),
        "timer_settime" => Some(110), "timer_delete" => Some(111),
        "clock_settime" => Some(112), "clock_gettime" => Some(113),
        "clock_getres" => Some(114), "clock_nanosleep" => Some(115),
        "syslog" => Some(116), "ptrace" => Some(117),
        "sched_setparam" => Some(118), "sched_setscheduler" => Some(119),
        "sched_getscheduler" => Some(120), "sched_getparam" => Some(121),
        "sched_setaffinity" => Some(122), "sched_getaffinity" => Some(123),
        "sched_yield" => Some(124), "sched_get_priority_max" => Some(125),
        "sched_get_priority_min" => Some(126), "sched_rr_get_interval" => Some(127),
        "restart_syscall" => Some(128), "kill" => Some(129), "tkill" => Some(130),
        "tgkill" => Some(131), "sigaltstack" => Some(132),
        "rt_sigsuspend" => Some(133), "rt_sigaction" => Some(134),
        "rt_sigprocmask" => Some(135), "rt_sigpending" => Some(136),
        "rt_sigtimedwait" => Some(137), "rt_sigqueueinfo" => Some(138),
        "rt_sigreturn" => Some(139), "setpriority" => Some(141),
        "getpriority" => Some(140), "reboot" => Some(142),
        "setregid" => Some(143), "setgid" => Some(144), "setreuid" => Some(145),
        "setuid" => Some(146), "setresuid" => Some(147), "getresuid" => Some(148),
        "setresgid" => Some(149), "getresgid" => Some(150),
        "setfsuid" => Some(151), "setfsgid" => Some(152), "times" => Some(153),
        "setpgid" => Some(154), "getpgid" => Some(155), "getsid" => Some(156),
        "setsid" => Some(157), "getgroups" => Some(158), "setgroups" => Some(159),
        "uname" => Some(160), "sethostname" => Some(161),
        "setdomainname" => Some(162), "getrlimit" => Some(163),
        "setrlimit" => Some(164), "getrusage" => Some(165), "umask" => Some(166),
        "prctl" => Some(167), "getcpu" => Some(168), "gettimeofday" => Some(169),
        "settimeofday" => Some(170), "adjtimex" => Some(171), "getpid" => Some(172),
        "getppid" => Some(173), "getuid" => Some(174), "geteuid" => Some(175),
        "getgid" => Some(176), "getegid" => Some(177), "gettid" => Some(178),
        "sysinfo" => Some(179), "mq_open" => Some(180), "mq_unlink" => Some(181),
        "mq_timedsend" => Some(182), "mq_timedreceive" => Some(183),
        "mq_notify" => Some(184), "mq_getsetattr" => Some(185), "msgget" => Some(186),
        "msgctl" => Some(187), "msgrcv" => Some(188), "msgsnd" => Some(189),
        "semget" => Some(190), "semctl" => Some(191), "semtimedop" => Some(192),
        "semop" => Some(193), "shmget" => Some(194), "shmctl" => Some(195),
        "shmat" => Some(196), "shmdt" => Some(197), "socket" => Some(198),
        "socketpair" => Some(199), "bind" => Some(200), "listen" => Some(201),
        "accept" => Some(202), "connect" => Some(203), "getsockname" => Some(204),
        "getpeername" => Some(205), "sendto" => Some(206), "recvfrom" => Some(207),
        "setsockopt" => Some(208), "getsockopt" => Some(209), "shutdown" => Some(210),
        "sendmsg" => Some(211), "recvmsg" => Some(212), "readahead" => Some(213),
        "brk" => Some(214), "munmap" => Some(215), "mremap" => Some(216),
        "add_key" => Some(217), "request_key" => Some(218), "keyctl" => Some(219),
        "clone" => Some(220), "execve" => Some(221), "mmap" => Some(222),
        "fadvise64" => Some(223), "swapon" => Some(224), "swapoff" => Some(225),
        "mprotect" => Some(226), "msync" => Some(227), "mlock" => Some(228),
        "munlock" => Some(229), "mlockall" => Some(230), "munlockall" => Some(231),
        "mincore" => Some(232), "madvise" => Some(233), "remap_file_pages" => Some(234),
        "mbind" => Some(235), "get_mempolicy" => Some(236),
        "set_mempolicy" => Some(237), "migrate_pages" => Some(238),
        "move_pages" => Some(239), "rt_tgsigqueueinfo" => Some(240),
        "perf_event_open" => Some(241), "accept4" => Some(242),
        "recvmmsg" => Some(243), "wait4" => Some(260), "prlimit64" => Some(261),
        "fanotify_init" => Some(262), "fanotify_mark" => Some(263),
        "name_to_handle_at" => Some(264), "open_by_handle_at" => Some(265),
        "clock_adjtime" => Some(266), "syncfs" => Some(267), "setns" => Some(268),
        "sendmmsg" => Some(269), "process_vm_readv" => Some(270),
        "process_vm_writev" => Some(271), "kcmp" => Some(272),
        "finit_module" => Some(273), "sched_setattr" => Some(274),
        "sched_getattr" => Some(275), "renameat2" => Some(276),
        "seccomp" => Some(277), "getrandom" => Some(278),
        "memfd_create" => Some(279), "bpf" => Some(280),
        "execveat" => Some(281), "userfaultfd" => Some(282),
        "membarrier" => Some(283), "mlock2" => Some(284), "copy_file_range" => Some(285),
        "preadv2" => Some(286), "pwritev2" => Some(287),
        "pkey_mprotect" => Some(288), "pkey_alloc" => Some(289),
        "pkey_free" => Some(290), "statx" => Some(291), "io_pgetevents" => Some(292),
        "rseq" => Some(293), "kexec_file_load" => Some(294),
        "pidfd_send_signal" => Some(424), "io_uring_setup" => Some(425),
        "io_uring_enter" => Some(426), "io_uring_register" => Some(427),
        "open_tree" => Some(428), "move_mount" => Some(429),
        "fsopen" => Some(430), "fsconfig" => Some(431), "fsmount" => Some(432),
        "fspick" => Some(433), "pidfd_open" => Some(434),
        "clone3" => Some(435), "close_range" => Some(436),
        "faccessat2" => Some(439), "process_madvise" => Some(440),
        "epoll_pwait2" => Some(441), "mount_setattr" => Some(442),
        "quotactl_fd" => Some(443), "landlock_create_ruleset" => Some(444),
        "landlock_add_rule" => Some(445), "landlock_restrict_self" => Some(446),
        "memfd_secret" => Some(447), "process_mrelease" => Some(448),
        "futex_waitv" => Some(449), "set_mempolicy_home_node" => Some(450),
        "fchmodat2" => Some(452), "map_shadow_stack" => Some(453),
        _ => None,
    };
}

// ===========================================================================
// BPF instruction builder
// ===========================================================================

pub fn bpf_insn(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    let mut buf = [0u8; 8];
    buf[0..2].copy_from_slice(&code.to_le_bytes());
    buf[2] = jt;
    buf[3] = jf;
    buf[4..8].copy_from_slice(&k.to_le_bytes());
    buf
}

// ===========================================================================
// BPF program generation
// ===========================================================================

/// Generate a seccomp-BPF program from OCI spec seccomp rules.
///
/// The generated program:
/// 1. Validates architecture
/// 2. Checks each syscall against the rule list
/// 3. Evaluates argument filters (64-bit comparisons)
/// 4. Applies the matching action (allow, errno, kill, etc.)
/// 5. Falls through to default_action if no rule matches
pub fn build_seccomp_prog(spec: &OciLinuxSeccomp) -> Vec<u8> {
    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 1. Load audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));

    // 2. Check architectures
    let archs = spec.architectures.as_deref().unwrap_or(&[]);
    let skip_past_arch_check: usize = if archs.is_empty() {
        1 // skip RET_KILL if not matching
    } else {
        archs.len() + 1
    };

    if archs.is_empty() {
        insns.push(bpf_insn_j(0x15, skip_past_arch_check.min(255) as u8, 0, CURRENT_ARCH));
    } else {
        for arch in archs {
            let arch_nr = arch_to_bpf(arch);
            insns.push(bpf_insn_j(0x15, skip_past_arch_check.min(255) as u8, 0, arch_nr));
        }
    }
    insns.push(bpf_insn(0x06, 0, 0, 0x00000000)); // SECCOMP_RET_KILL_THREAD

    // 3. Load syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 4. Build syscall rules with argument filters
    let entries = spec.syscalls.as_deref().unwrap_or(&[]);
    let default_action = spec.default_action.as_ref()
        .unwrap_or(&OciSeccompAction::Kill);
    let default_ret = action_to_bpf(default_action, spec.default_errno_ret);

    for (i, entry) in entries.iter().enumerate() {
        let Some(names) = entry.names.as_ref() else { continue };
        let action = entry.action.as_ref().unwrap_or(default_action);
        let ret_val = action_to_bpf(action, entry.errno_ret);
        let args = entry.args.as_deref().unwrap_or(&[]);

        // Count how many instructions this syscall entry will generate
        // Per syscall name: 1 (JEQ) + arg_check_count + 1 (RET)
        let arg_check_insns = args.len() * 2; // 2 insns per arg (low + high 32-bit)
        let per_syscall_insns = 1 + arg_check_insns + 1; // JEQ + args + RET

        for name in names {
            let Some(nr) = syscall_nr(name) else { continue };

            // Count remaining instructions after this JEQ
            let remaining = {
                let mut count = 0;
                // For this syscall: arg checks + RET
                count += arg_check_insns + 1;
                // For remaining syscalls in this entry
                let remaining_in_entry = names.len() - names.iter()
                    .position(|n| n == name).unwrap_or(0) - 1;
                count += remaining_in_entry * per_syscall_insns;
                // For remaining entries
                for entry in entries.iter().skip(i + 1) {
                    if let Some(e) = entry.names.as_ref() {
                        count += e.len();
                    }
                }
                // Default RET
                count += 1;
                count
            };

            // JEQ: if nr matches, fall through to arg checks (jt=0)
            //       if not, skip past this syscall's block (jf=remaining)
            insns.push(bpf_insn(0x15, 0, remaining.min(255) as u8, nr));

            // Generate arg filter checks
            for (arg_idx, arg) in args.iter().enumerate() {
                let remaining_args = args.len() - arg_idx - 1;
                // For EQ/GE/GT/LE/LT: mismatch → skip past remaining args + RET to next entry/default
                let skip_to_default = (remaining_args * 2 + 1).min(255) as u8;
                // For NE: mismatch means values differ → fall through to RET (skip remaining checks only)
                let skip_to_ret = (remaining_args * 2).min(255) as u8;

                // Load low 32 bits of arg (offset = 16 + index*8)
                let arg_offset = 16 + arg.index * 8;
                insns.push(bpf_insn(0x20, 0, 0, arg_offset));
                let lo_val = arg.value as u32;

                // BPF comparison operators:
                //   0x25 = BPF_JGT (unsigned): jt if A > K, jf if A <= K
                //   0x30 = BPF_JGE (unsigned): jt if A >= K, jf if A < K
                //   0x15 = BPF_JEQ: jt if A == K, jf if A != K
                //
                // For each operator we want: "skip when comparison is FALSE"
                //   EQ: A != K → skip  → JEQ, jt=0, jf=skip  ✓
                //   NE: A == K → skip  → JEQ, jt=skip, jf=0  ✓
                //   LT: A < K true → continue. A >= K → skip  → JGE, jt=skip, jf=0
                //   LE: A <= K true → continue. A > K → skip  → JGT, jt=skip, jf=0
                //   GE: A >= K true → continue. A < K → skip  → JGE, jt=0, jf=skip
                //   GT: A > K true → continue. A <= K → skip  → JGT, jt=0, jf=skip

                match arg.op.as_str() {
                    // EQ: A == K → match → continue (jt=0). A != K → no match → skip (jf=skip)
                    "SCMP_CMP_EQ" => {
                        insns.push(bpf_insn(0x15, 0, skip_to_default, lo_val));
                    }
                    // NE: A != K → match → jump to RET (jf=skip_to_ret). A == K → continue to hi (jt=0)
                    "SCMP_CMP_NE" => {
                        insns.push(bpf_insn(0x15, 0, skip_to_ret, lo_val));
                    }
                    // LT: A < K. If A >= K → skip. JGE: if A >= K → jt=skip.
                    "SCMP_CMP_LT" => {
                        insns.push(bpf_insn(0x30, skip_to_default, 0, lo_val));
                    }
                    // LE: A <= K. If A > K → skip. JGT: if A > K → jt=skip.
                    "SCMP_CMP_LE" => {
                        insns.push(bpf_insn(0x25, skip_to_default, 0, lo_val));
                    }
                    // GE: A >= K. If A >= K → continue (jt=0). If A < K → skip (jf=skip).
                    "SCMP_CMP_GE" => {
                        insns.push(bpf_insn(0x30, 0, skip_to_default, lo_val));
                    }
                    // GT: A > K. If A > K → continue (jt=0). If A <= K → skip (jf=skip).
                    "SCMP_CMP_GT" => {
                        insns.push(bpf_insn(0x25, 0, skip_to_default, lo_val));
                    }
                    // MASKED_EQ: (A & mask) == valueTwo
                    //   Step 1: AND low 32 bits with mask, check == expected low
                    //   Step 2: AND high 32 bits with mask, check == expected high
                    //   Both halves must match to pass.
                    "SCMP_CMP_MASKED_EQ" => {
                        // AND low 32 bits with mask, then check == expected low
                        insns.push(bpf_insn(0x50, 0, 0, arg.value as u32));
                        // JEQ expected_lo → continue to hi check. Mismatch → skip past hi check + RET + remaining args
                        let skip_total = (remaining_args * 2 + 2 + 1).min(255) as u8; // remaining + hi_load + hi_jeq + ret
                        insns.push(bpf_insn(0x15, 0, skip_total, arg.value_two as u32));
                    }
                    _ => {
                        insns.push(bpf_insn(0x15, 0, skip_to_default, lo_val));
                    }
                }

                // Load high 32 bits of arg (offset = 16 + index*8 + 4)
                let arg_offset_hi = 16 + arg.index * 8 + 4;
                let hi_val = (arg.value >> 32) as u32;
                insns.push(bpf_insn(0x20, 0, 0, arg_offset_hi));

                match arg.op.as_str() {
                    // EQ: both match → fall through to RET (jt=0). hi mismatch → skip (jf=1)
                    "SCMP_CMP_EQ" => {
                        insns.push(bpf_insn(0x15, 0, 1, hi_val));
                    }
                    // NE: both match (full equality) → skip past RET (jt=1). hi differs → match → RET (jf=0)
                    "SCMP_CMP_NE" => {
                        insns.push(bpf_insn(0x15, 1, 0, hi_val));
                    }
                    // LT: A < K. If hi > K_hi → A > K → skip. If hi < K_hi → A < K → continue.
                    "SCMP_CMP_LT" => {
                        insns.push(bpf_insn(0x25, 1, 0, hi_val));
                    }
                    // LE: A <= K. If hi > K_hi → skip.
                    "SCMP_CMP_LE" => {
                        insns.push(bpf_insn(0x25, 1, 0, hi_val));
                    }
                    // GE: A >= K. If hi < K_hi → skip.
                    "SCMP_CMP_GE" => {
                        insns.push(bpf_insn(0x30, 0, 1, hi_val));
                    }
                    // GT: A > K. If hi > K_hi → continue. If hi <= K_hi → skip.
                    //   (lo already matched exactly, so hi==K_hi means A==K, not >)
                    //   JGT: if A > K → jt=0 (continue). if A <= K → jf=1 (skip RET).
                    "SCMP_CMP_GT" => {
                        insns.push(bpf_insn(0x25, 0, 1, hi_val));
                    }
                    // MASKED_EQ hi: AND high 32 bits with mask, check == expected high
                    "SCMP_CMP_MASKED_EQ" => {
                        let mask_hi = (arg.value >> 32) as u32;
                        let expected_hi = (arg.value_two >> 32) as u32;
                        insns.push(bpf_insn(0x50, 0, 0, mask_hi));  // A = A & mask_hi
                        // Skip past RET on mismatch
                        insns.push(bpf_insn(0x15, 0, 1, expected_hi));
                    }
                    _ => {
                        insns.push(bpf_insn(0x15, 0, 1, hi_val));
                    }
                }
            }

            // All args matched — apply action
            insns.push(bpf_insn(0x06, 0, 0, ret_val));
        }
    }

    // 5. Default action (fallthrough)
    insns.push(bpf_insn(0x06, 0, 0, default_ret));

    // Build sock_fprog: { len: u16, filter: *sock_filter }
    let mut insn_bytes = [0u8; 256 * 8];
    let src = unsafe { std::slice::from_raw_parts(insns.as_ptr() as *const u8, insns.len() * 8) };
    insn_bytes[..src.len()].copy_from_slice(src);

    let prog_len = insns.len() as u16;
    let prog_ptr = insn_bytes.as_ptr();

    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&prog_len.to_le_bytes());
    prog.resize(8, 0);
    prog.extend_from_slice(&(prog_ptr as u64).to_le_bytes());
    prog
}

/// Generate the built-in fallback allow-list when no spec seccomp rules are present.
pub fn seccomp_bpf_prog() -> Vec<u8> {
    let mut insns: Vec<[u8; 8]> = Vec::new();

    // 0: LOAD audit_arch
    insns.push(bpf_insn(0x20, 0, 0, 4));
    // 1: JEQ expected_arch ? continue : kill
    let skip_to_deny = ALLOWED.len() + 2;
    insns.push(bpf_insn_j(0x15, skip_to_deny.min(255) as u8, 0, CURRENT_ARCH));
    // 2: LOAD syscall_nr
    insns.push(bpf_insn(0x20, 0, 0, 0));

    // 3..N: Check each allowed syscall
    let n = ALLOWED.len();
    for (i, &nr) in ALLOWED.iter().enumerate() {
        let skip = (n - i).min(255) as u8;
        insns.push(bpf_insn_j(0x15, skip, 0, nr));
    }

    // RET ERRNO(EPERM) — default deny
    insns.push(bpf_insn(0x06, 0, 0, 0x00050001));
    // RET ALLOW
    insns.push(bpf_insn(0x06, 0, 0, 0x7fff0000));

    let mut insn_bytes = [0u8; 256 * 8];
    let src = unsafe { std::slice::from_raw_parts(insns.as_ptr() as *const u8, insns.len() * 8) };
    insn_bytes[..src.len()].copy_from_slice(src);

    let prog_len = insns.len() as u16;
    let prog_ptr = insn_bytes.as_ptr();

    let mut prog = Vec::with_capacity(16);
    prog.extend_from_slice(&prog_len.to_le_bytes());
    prog.resize(8, 0);
    prog.extend_from_slice(&(prog_ptr as u64).to_le_bytes());
    prog
}

// ===========================================================================
// Syscall number constants (x86_64)
// ===========================================================================

#[cfg(target_arch = "x86_64")]
mod nr {
    pub const READ: u32 = 0;
    pub const WRITE: u32 = 1;
    pub const CLOSE: u32 = 3;
    pub const FSTAT: u32 = 5;
    pub const MMAP: u32 = 9;
    pub const MPROTECT: u32 = 10;
    pub const MUNMAP: u32 = 11;
    pub const BRK: u32 = 12;
    pub const RT_SIGACTION: u32 = 13;
    pub const RT_SIGPROCMASK: u32 = 14;
    pub const RT_SIGRETURN: u32 = 15;
    pub const IOCTL: u32 = 16;
    pub const PREAD64: u32 = 17;
    pub const PWRITE64: u32 = 18;
    pub const READV: u32 = 19;
    pub const WRITEV: u32 = 20;
    pub const ACCESS: u32 = 21;
    pub const POLL: u32 = 22;
    pub const SELECT: u32 = 23;
    pub const SCHED_YIELD: u32 = 24;
    pub const MREMAP: u32 = 25;
    pub const MINCORE: u32 = 27;
    pub const MADVISE: u32 = 28;
    pub const DUP: u32 = 32;
    pub const DUP2: u32 = 33;
    pub const NANOSLEEP: u32 = 35;
    pub const GETPID: u32 = 39;
    pub const SOCKET: u32 = 41;
    pub const CONNECT: u32 = 42;
    pub const ACCEPT: u32 = 43;
    pub const SENDTO: u32 = 44;
    pub const RECVFROM: u32 = 45;
    pub const SENDMSG: u32 = 46;
    pub const RECVMSG: u32 = 47;
    pub const SHUTDOWN: u32 = 48;
    pub const BIND: u32 = 49;
    pub const LISTEN: u32 = 50;
    pub const GETSOCKNAME: u32 = 51;
    pub const GETPEERNAME: u32 = 52;
    pub const SOCKETPAIR: u32 = 53;
    pub const SETSOCKOPT: u32 = 54;
    pub const GETSOCKOPT: u32 = 55;
    pub const CLONE: u32 = 56;
    pub const FORK: u32 = 57;
    pub const VFORK: u32 = 58;
    pub const EXECVE: u32 = 59;
    pub const EXIT: u32 = 60;
    pub const WAIT4: u32 = 61;
    pub const KILL: u32 = 62;
    pub const UNAME: u32 = 63;
    pub const FCNTL: u32 = 72;
    pub const FLOCK: u32 = 73;
    pub const FSYNC: u32 = 74;
    pub const GETCWD: u32 = 79;
    pub const CHDIR: u32 = 80;
    pub const FCHDIR: u32 = 81;
    pub const RENAME: u32 = 82;
    pub const MKDIR: u32 = 83;
    pub const RMDIR: u32 = 84;
    pub const UNLINK: u32 = 86;
    pub const SYMLINK: u32 = 87;
    pub const READLINK: u32 = 88;
    pub const CHMOD: u32 = 90;
    pub const FCHMOD: u32 = 91;
    pub const CHOWN: u32 = 92;
    pub const FCHOWN: u32 = 93;
    pub const GETRLIMIT: u32 = 97;
    pub const GETRUSAGE: u32 = 98;
    pub const GETTIMEOFDAY: u32 = 96;
    pub const SYSINFO: u32 = 99;
    pub const TIMES: u32 = 100;
    pub const GETUID: u32 = 102;
    pub const SYSLOG: u32 = 103;
    pub const GETGID: u32 = 104;
    pub const SETUID: u32 = 105;
    pub const SETGID: u32 = 106;
    pub const GETEUID: u32 = 107;
    pub const GETEGID: u32 = 108;
    pub const SETPGID: u32 = 109;
    pub const GETPPID: u32 = 110;
    pub const GETPGRP: u32 = 111;
    pub const SETSID: u32 = 112;
    pub const SETRESUID: u32 = 117;
    pub const GETRESUID: u32 = 118;
    pub const SETRESGID: u32 = 119;
    pub const GETRESGID: u32 = 120;
    pub const GETPGID: u32 = 121;
    pub const SETFSUID: u32 = 122;
    pub const SETFSGID: u32 = 123;
    pub const GETGROUPS: u32 = 115;
    pub const SETGROUPS: u32 = 116;
    pub const CAPGET: u32 = 125;
    pub const CAPSET: u32 = 126;
    pub const RT_SIGPENDING: u32 = 127;
    pub const RT_SIGTIMEDWAIT: u32 = 128;
    pub const RT_SIGQUEUEINFO: u32 = 129;
    pub const RT_SIGSUSPEND: u32 = 130;
    pub const SIGALTSTACK: u32 = 131;
    pub const TKILL: u32 = 200;
    pub const FUTEX: u32 = 202;
    pub const SCHED_SETPARAM: u32 = 142;
    pub const SCHED_GETPARAM: u32 = 143;
    pub const SCHED_SETSCHEDULER: u32 = 144;
    pub const SCHED_GETSCHEDULER: u32 = 145;
    pub const SCHED_GET_PRIORITY_MAX: u32 = 146;
    pub const SCHED_GET_PRIORITY_MIN: u32 = 147;
    pub const SCHED_RR_GET_INTERVAL: u32 = 148;
    pub const SCHED_SETAFFINITY: u32 = 203;
    pub const SCHED_GETAFFINITY: u32 = 204;
    pub const GETSID: u32 = 124;
    pub const PRCTL: u32 = 157;
    pub const ARCH_PRCTL: u32 = 158;
    pub const SETRLIMIT: u32 = 160;
    pub const MKNOD: u32 = 133;
    pub const PIVOT_ROOT: u32 = 155;
    pub const MOUNT: u32 = 165;
    pub const UMOUNT2: u32 = 166;
    pub const SETNS: u32 = 308;
    pub const UNSHARE: u32 = 272;
    pub const CLOCK_GETTIME: u32 = 228;
    pub const CLOCK_GETRES: u32 = 229;
    pub const CLOCK_NANOSLEEP: u32 = 230;
    pub const EXIT_GROUP: u32 = 231;
    pub const EPOLL_WAIT: u32 = 232;
    pub const EPOLL_CTL: u32 = 233;
    pub const TGKILL: u32 = 234;
    pub const OPENAT: u32 = 257;
    pub const MKDIRAT: u32 = 258;
    pub const MKNODAT: u32 = 259;
    pub const FCHOWNAT: u32 = 260;
    pub const NEWFSTATAT: u32 = 262;
    pub const UNLINKAT: u32 = 263;
    pub const RENAMEAT: u32 = 264;
    pub const LINKAT: u32 = 265;
    pub const SYMLINKAT: u32 = 266;
    pub const READLINKAT: u32 = 267;
    pub const FCHMODAT: u32 = 268;
    pub const FACCESSAT: u32 = 269;
    pub const PSELECT6: u32 = 270;
    pub const PPOLL: u32 = 271;
    pub const SET_ROBUST_LIST: u32 = 273;
    pub const GET_ROBUST_LIST: u32 = 274;
    pub const SIGNALFD: u32 = 282;
    pub const TIMERFD_CREATE: u32 = 283;
    pub const TIMERFD_SETTIME: u32 = 286;
    pub const TIMERFD_GETTIME: u32 = 287;
    pub const EVENTFD: u32 = 284;
    pub const EPOLL_PWAIT: u32 = 281;
    pub const ACCEPT4: u32 = 288;
    pub const SIGNALFD4: u32 = 289;
    pub const EPOLL_CREATE1: u32 = 291;
    pub const DUP3: u32 = 292;
    pub const RECVMMSG: u32 = 299;
    pub const PRLIMIT64: u32 = 302;
    pub const GETRANDOM: u32 = 318;
    pub const SECCOMP: u32 = 317;
    pub const GETDENTS64: u32 = 217;
    pub const SET_TID_ADDRESS: u32 = 218;
    pub const RSEQ: u32 = 334;
    pub const GETCPU: u32 = 309;
    pub const PIDFD_SEND_SIGNAL: u32 = 424;
    pub const PIDFD_OPEN: u32 = 434;
    pub const CLOSE_RANGE: u32 = 436;
    pub const CLONE3: u32 = 435;
    pub const FACCESSAT2: u32 = 439;
    pub const FALLOCATE: u32 = 285;
}

// ===========================================================================
// Syscall number constants (aarch64)
// ===========================================================================

#[cfg(target_arch = "aarch64")]
mod nr {
    pub const READ: u32 = 63;
    pub const WRITE: u32 = 64;
    pub const CLOSE: u32 = 57;
    pub const FSTAT: u32 = 80;
    pub const MMAP: u32 = 222;
    pub const MPROTECT: u32 = 226;
    pub const MUNMAP: u32 = 215;
    pub const BRK: u32 = 214;
    pub const RT_SIGACTION: u32 = 134;
    pub const RT_SIGPROCMASK: u32 = 135;
    pub const RT_SIGRETURN: u32 = 139;
    pub const IOCTL: u32 = 29;
    pub const PREAD64: u32 = 67;
    pub const PWRITE64: u32 = 68;
    pub const READV: u32 = 65;
    pub const WRITEV: u32 = 66;
    pub const ACCESS: u32 = 48;
    pub const POLL: u32 = 72;
    pub const SELECT: u32 = 73;
    pub const SCHED_YIELD: u32 = 124;
    pub const MREMAP: u32 = 216;
    pub const MINCORE: u32 = 232;
    pub const MADVISE: u32 = 233;
    pub const DUP: u32 = 23;
    pub const DUP2: u32 = 24;
    pub const NANOSLEEP: u32 = 101;
    pub const GETPID: u32 = 172;
    pub const SOCKET: u32 = 198;
    pub const CONNECT: u32 = 203;
    pub const ACCEPT: u32 = 202;
    pub const SENDTO: u32 = 206;
    pub const RECVFROM: u32 = 207;
    pub const SENDMSG: u32 = 211;
    pub const RECVMSG: u32 = 212;
    pub const SHUTDOWN: u32 = 210;
    pub const BIND: u32 = 200;
    pub const LISTEN: u32 = 201;
    pub const GETSOCKNAME: u32 = 204;
    pub const GETPEERNAME: u32 = 205;
    pub const SOCKETPAIR: u32 = 199;
    pub const SETSOCKOPT: u32 = 208;
    pub const GETSOCKOPT: u32 = 209;
    pub const CLONE: u32 = 220;
    pub const EXECVE: u32 = 221;
    pub const EXIT: u32 = 93;
    pub const WAIT4: u32 = 260;
    pub const KILL: u32 = 129;
    pub const UNAME: u32 = 160;
    pub const FCNTL: u32 = 25;
    pub const FLOCK: u32 = 32;
    pub const FSYNC: u32 = 82;
    pub const GETCWD: u32 = 17;
    pub const CHDIR: u32 = 49;
    pub const FCHDIR: u32 = 50;
    pub const RENAME: u32 = 38;
    pub const MKDIR: u32 = 34;
    pub const RMDIR: u32 = 35;
    pub const UNLINK: u32 = 35;
    pub const SYMLINK: u32 = 36;
    pub const READLINK: u32 = 78;
    pub const CHMOD: u32 = 52;
    pub const FCHMOD: u32 = 53;
    pub const CHOWN: u32 = 55;
    pub const FCHOWN: u32 = 55;
    pub const GETRLIMIT: u32 = 163;
    pub const GETRUSAGE: u32 = 165;
    pub const GETTIMEOFDAY: u32 = 169;
    pub const SYSINFO: u32 = 179;
    pub const TIMES: u32 = 153;
    pub const GETUID: u32 = 174;
    pub const SYSLOG: u32 = 116;
    pub const GETGID: u32 = 176;
    pub const SETUID: u32 = 146;
    pub const SETGID: u32 = 144;
    pub const GETEUID: u32 = 175;
    pub const GETEGID: u32 = 177;
    pub const SETPGID: u32 = 154;
    pub const GETPPID: u32 = 173;
    pub const SETSID: u32 = 157;
    pub const SETRESUID: u32 = 147;
    pub const GETRESUID: u32 = 148;
    pub const SETRESGID: u32 = 149;
    pub const GETRESGID: u32 = 150;
    pub const GETPGID: u32 = 155;
    pub const SETFSUID: u32 = 151;
    pub const SETFSGID: u32 = 152;
    pub const GETGROUPS: u32 = 158;
    pub const SETGROUPS: u32 = 159;
    pub const CAPGET: u32 = 90;
    pub const CAPSET: u32 = 91;
    pub const RT_SIGPENDING: u32 = 136;
    pub const RT_SIGTIMEDWAIT: u32 = 137;
    pub const RT_SIGQUEUEINFO: u32 = 138;
    pub const RT_SIGSUSPEND: u32 = 133;
    pub const SIGALTSTACK: u32 = 132;
    pub const TKILL: u32 = 130;
    pub const FUTEX: u32 = 98;
    pub const SCHED_SETPARAM: u32 = 118;
    pub const SCHED_GETPARAM: u32 = 121;
    pub const SCHED_SETSCHEDULER: u32 = 119;
    pub const SCHED_GETSCHEDULER: u32 = 120;
    pub const SCHED_GET_PRIORITY_MAX: u32 = 125;
    pub const SCHED_GET_PRIORITY_MIN: u32 = 126;
    pub const SCHED_RR_GET_INTERVAL: u32 = 127;
    pub const SCHED_SETAFFINITY: u32 = 122;
    pub const SCHED_GETAFFINITY: u32 = 123;
    pub const GETSID: u32 = 156;
    pub const PRCTL: u32 = 167;
    pub const SETRLIMIT: u32 = 164;
    pub const MKNOD: u32 = 33;
    pub const PIVOT_ROOT: u32 = 41;
    pub const MOUNT: u32 = 40;
    pub const UMOUNT2: u32 = 39;
    pub const SETNS: u32 = 268;
    pub const UNSHARE: u32 = 97;
    pub const CLOCK_GETTIME: u32 = 113;
    pub const CLOCK_GETRES: u32 = 114;
    pub const CLOCK_NANOSLEEP: u32 = 115;
    pub const EXIT_GROUP: u32 = 94;
    pub const EPOLL_WAIT: u32 = 21;
    pub const EPOLL_CTL: u32 = 21;
    pub const TGKILL: u32 = 131;
    pub const OPENAT: u32 = 56;
    pub const MKDIRAT: u32 = 34;
    pub const MKNODAT: u32 = 33;
    pub const FCHOWNAT: u32 = 54;
    pub const NEWFSTATAT: u32 = 79;
    pub const UNLINKAT: u32 = 35;
    pub const RENAMEAT: u32 = 38;
    pub const LINKAT: u32 = 37;
    pub const SYMLINKAT: u32 = 36;
    pub const READLINKAT: u32 = 78;
    pub const FCHMODAT: u32 = 53;
    pub const FACCESSAT: u32 = 48;
    pub const PSELECT6: u32 = 72;
    pub const PPOLL: u32 = 73;
    pub const SET_ROBUST_LIST: u32 = 99;
    pub const GET_ROBUST_LIST: u32 = 100;
    pub const SIGNALFD: u32 = 74;
    pub const TIMERFD_CREATE: u32 = 85;
    pub const TIMERFD_SETTIME: u32 = 86;
    pub const TIMERFD_GETTIME: u32 = 87;
    pub const EVENTFD: u32 = 19;
    pub const EPOLL_PWAIT: u32 = 22;
    pub const ACCEPT4: u32 = 242;
    pub const SIGNALFD4: u32 = 74;
    pub const EPOLL_CREATE1: u32 = 20;
    pub const DUP3: u32 = 24;
    pub const RECVMMSG: u32 = 243;
    pub const PRLIMIT64: u32 = 261;
    pub const GETRANDOM: u32 = 278;
    pub const SECCOMP: u32 = 277;
    pub const GETDENTS64: u32 = 61;
    pub const SET_TID_ADDRESS: u32 = 96;
    pub const RSEQ: u32 = 293;
    pub const GETCPU: u32 = 168;
    pub const PIDFD_SEND_SIGNAL: u32 = 424;
    pub const PIDFD_OPEN: u32 = 434;
    pub const CLOSE_RANGE: u32 = 436;
    pub const CLONE3: u32 = 435;
    pub const FACCESSAT2: u32 = 439;
    pub const FALLOCATE: u32 = 47;
}

// ===========================================================================
// Built-in allow-list for fallback when no spec seccomp rules provided
// ===========================================================================

#[cfg(target_arch = "x86_64")]
const ALLOWED: &[u32] = &[
    // I/O fundamentals
    nr::READ, nr::WRITE, nr::CLOSE,
    nr::PREAD64, nr::PWRITE64, nr::READV, nr::WRITEV,
    nr::GETDENTS64, nr::FCNTL, nr::FLOCK,

    // File operations
    nr::OPENAT, nr::MKDIRAT, nr::MKNODAT, nr::FCHOWNAT, nr::NEWFSTATAT,
    nr::UNLINKAT, nr::RENAMEAT, nr::LINKAT, nr::SYMLINKAT,
    nr::READLINKAT, nr::FCHMODAT, nr::FACCESSAT, nr::FACCESSAT2,
    nr::FSYNC,
    nr::RENAME, nr::MKDIR, nr::RMDIR, nr::UNLINK, nr::SYMLINK, nr::READLINK,
    nr::CHMOD, nr::FCHMOD, nr::CHOWN, nr::FCHOWN, nr::MKNOD,

    // Memory management
    nr::MMAP, nr::MPROTECT, nr::MUNMAP, nr::BRK,
    nr::MREMAP, nr::MINCORE, nr::MADVISE, nr::FALLOCATE,

    // Signal handling
    nr::RT_SIGACTION, nr::RT_SIGPROCMASK, nr::RT_SIGRETURN,
    nr::KILL, nr::RT_SIGPENDING, nr::RT_SIGTIMEDWAIT,
    nr::RT_SIGQUEUEINFO, nr::RT_SIGSUSPEND,
    nr::SIGALTSTACK, nr::TGKILL, nr::TKILL,

    // Process management
    nr::GETPID, nr::CLONE, nr::CLONE3, nr::FORK, nr::VFORK,
    nr::EXECVE, nr::EXIT, nr::EXIT_GROUP, nr::WAIT4,
    nr::SETPGID, nr::GETPPID, nr::GETPGRP, nr::SETSID,
    nr::GETPGID, nr::GETSID,

    // Thread / TLS setup (glibc)
    nr::SET_TID_ADDRESS, nr::SET_ROBUST_LIST, nr::GET_ROBUST_LIST,
    nr::GETTIMEOFDAY, nr::CLOCK_GETTIME, nr::CLOCK_GETRES,
    nr::CLOCK_NANOSLEEP, nr::NANOSLEEP,
    nr::FUTEX, nr::ARCH_PRCTL, nr::RSEQ,
    nr::SYSINFO, nr::TIMES,

    // Random / entropy
    nr::GETRANDOM,

    // User/identity
    nr::GETUID, nr::GETGID, nr::SETUID, nr::SETGID,
    nr::GETEUID, nr::GETEGID,
    nr::GETGROUPS, nr::SETGROUPS,
    nr::SETRESUID, nr::GETRESUID, nr::SETRESGID, nr::GETRESGID,
    nr::SETFSUID, nr::SETFSGID,

    // Capabilities / security
    nr::CAPGET, nr::CAPSET, nr::PRCTL, nr::SECCOMP,

    // Resource limits
    nr::GETRLIMIT, nr::SETRLIMIT, nr::PRLIMIT64, nr::GETRUSAGE, nr::SYSLOG,

    // Network
    nr::SOCKET, nr::CONNECT, nr::ACCEPT, nr::ACCEPT4,
    nr::SENDTO, nr::RECVFROM, nr::SENDMSG, nr::RECVMSG, nr::RECVMMSG,
    nr::SHUTDOWN, nr::BIND, nr::LISTEN,
    nr::GETSOCKNAME, nr::GETPEERNAME, nr::SOCKETPAIR,
    nr::SETSOCKOPT, nr::GETSOCKOPT,

    // Event / epoll / io multiplexing
    nr::POLL, nr::SELECT, nr::PSELECT6, nr::PPOLL,
    nr::EPOLL_WAIT, nr::EPOLL_CTL, nr::EPOLL_PWAIT, nr::EPOLL_CREATE1,
    nr::SIGNALFD, nr::SIGNALFD4,
    nr::TIMERFD_CREATE, nr::TIMERFD_SETTIME, nr::TIMERFD_GETTIME,
    nr::EVENTFD,

    // Scheduling
    nr::SCHED_YIELD,
    nr::SCHED_SETPARAM, nr::SCHED_GETPARAM,
    nr::SCHED_SETSCHEDULER, nr::SCHED_GETSCHEDULER,
    nr::SCHED_GET_PRIORITY_MAX, nr::SCHED_GET_PRIORITY_MIN,
    nr::SCHED_RR_GET_INTERVAL,
    nr::SCHED_SETAFFINITY, nr::SCHED_GETAFFINITY,

    // Misc
    nr::UNAME, nr::IOCTL, nr::ACCESS,
    nr::DUP, nr::DUP2, nr::DUP3,
    nr::CHDIR, nr::FCHDIR, nr::GETCWD,
    nr::GETCPU,
    nr::PIDFD_SEND_SIGNAL, nr::PIDFD_OPEN, nr::CLOSE_RANGE,
];

#[cfg(target_arch = "aarch64")]
const ALLOWED: &[u32] = &[
    // I/O fundamentals
    nr::READ, nr::WRITE, nr::CLOSE,
    nr::PREAD64, nr::PWRITE64, nr::READV, nr::WRITEV,
    nr::GETDENTS64, nr::FCNTL, nr::FLOCK,

    // File operations
    nr::OPENAT, nr::MKDIRAT, nr::MKNODAT, nr::FCHOWNAT, nr::NEWFSTATAT,
    nr::UNLINKAT, nr::RENAMEAT, nr::LINKAT, nr::SYMLINKAT,
    nr::READLINKAT, nr::FCHMODAT, nr::FACCESSAT, nr::FACCESSAT2,
    nr::FSYNC,
    nr::FCHMOD, nr::FCHOWN, nr::GETCWD,

    // Memory management
    nr::MMAP, nr::MPROTECT, nr::MUNMAP, nr::BRK,
    nr::MREMAP, nr::MINCORE, nr::MADVISE, nr::FALLOCATE,

    // Signal handling
    nr::RT_SIGACTION, nr::RT_SIGPROCMASK, nr::RT_SIGRETURN,
    nr::KILL, nr::RT_SIGPENDING, nr::RT_SIGTIMEDWAIT,
    nr::RT_SIGQUEUEINFO, nr::RT_SIGSUSPEND,
    nr::SIGALTSTACK, nr::TGKILL, nr::TKILL,

    // Process management
    nr::GETPID, nr::CLONE, nr::CLONE3,
    nr::EXECVE, nr::EXIT, nr::EXIT_GROUP, nr::WAIT4,
    nr::SETPGID, nr::GETPPID, nr::SETSID,
    nr::GETPGID, nr::GETSID,

    // Thread / TLS setup (glibc)
    nr::SET_TID_ADDRESS, nr::SET_ROBUST_LIST, nr::GET_ROBUST_LIST,
    nr::GETTIMEOFDAY, nr::CLOCK_GETTIME, nr::CLOCK_GETRES,
    nr::CLOCK_NANOSLEEP, nr::NANOSLEEP,
    nr::FUTEX, nr::RSEQ,
    nr::SYSINFO, nr::TIMES,

    // Random / entropy
    nr::GETRANDOM,

    // User/identity
    nr::GETUID, nr::GETGID, nr::SETUID, nr::SETGID,
    nr::GETEUID, nr::GETEGID,
    nr::GETGROUPS, nr::SETGROUPS,
    nr::SETRESUID, nr::GETRESUID, nr::SETRESGID, nr::GETRESGID,
    nr::SETFSUID, nr::SETFSGID,

    // Capabilities / security
    nr::CAPGET, nr::CAPSET, nr::PRCTL, nr::SECCOMP,

    // Resource limits
    nr::GETRLIMIT, nr::SETRLIMIT, nr::PRLIMIT64, nr::GETRUSAGE, nr::SYSLOG,

    // Network
    nr::SOCKET, nr::CONNECT, nr::ACCEPT, nr::ACCEPT4,
    nr::SENDTO, nr::RECVFROM, nr::SENDMSG, nr::RECVMSG, nr::RECVMMSG,
    nr::SHUTDOWN, nr::BIND, nr::LISTEN,
    nr::GETSOCKNAME, nr::GETPEERNAME, nr::SOCKETPAIR,
    nr::SETSOCKOPT, nr::GETSOCKOPT,

    // Event / epoll / io multiplexing
    nr::PSELECT6, nr::PPOLL,
    nr::EPOLL_CTL, nr::EPOLL_PWAIT, nr::EPOLL_CREATE1,
    nr::SIGNALFD4,
    nr::TIMERFD_CREATE, nr::TIMERFD_SETTIME, nr::TIMERFD_GETTIME,
    nr::EVENTFD,

    // Scheduling
    nr::SCHED_YIELD,
    nr::SCHED_SETPARAM, nr::SCHED_GETPARAM,
    nr::SCHED_SETSCHEDULER, nr::SCHED_GETSCHEDULER,
    nr::SCHED_GET_PRIORITY_MAX, nr::SCHED_GET_PRIORITY_MIN,
    nr::SCHED_RR_GET_INTERVAL,
    nr::SCHED_SETAFFINITY, nr::SCHED_GETAFFINITY,

    // Misc
    nr::UNAME, nr::IOCTL, nr::ACCESS,
    nr::DUP, nr::DUP3,
    nr::CHDIR, nr::FCHDIR,
    nr::GETCPU,
    nr::PIDFD_SEND_SIGNAL, nr::PIDFD_OPEN, nr::CLOSE_RANGE,
];

fn bpf_insn_j(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    bpf_insn(code, jt, jf, k)
}

// ===========================================================================
// Action conversion
// ===========================================================================

fn action_to_bpf(action: &OciSeccompAction, errno_ret: Option<u32>) -> u32 {
    match action {
        OciSeccompAction::Allow => 0x7fff0000,        // SECCOMP_RET_ALLOW
        OciSeccompAction::Kill => 0x00000000,         // SECCOMP_RET_KILL_THREAD
        OciSeccompAction::KillProcess => 0x80000000,  // SECCOMP_RET_KILL_PROCESS
        OciSeccompAction::KillThread => 0x00000000,   // same as Kill
        OciSeccompAction::Trap => 0x00030000,         // SECCOMP_RET_TRAP
        OciSeccompAction::Errno => 0x00050000 | (errno_ret.unwrap_or(1) & 0x0000ffff), // SECCOMP_RET_ERRNO
        OciSeccompAction::Trace => 0x7ff00000,        // SECCOMP_RET_TRACE
        OciSeccompAction::Log => 0x7ffe0000,          // SECCOMP_RET_LOG
        OciSeccompAction::Notify => 0x7fc00000,       // SECCOMP_RET_NOTIFY
    }
}

fn arch_to_bpf(arch: &str) -> u32 {
    match arch {
        "SCMP_ARCH_X86_64" => 0xc000003e,
        "SCMP_ARCH_X86" => 0x40000003,
        "SCMP_ARCH_X32" => 0x4000003e,
        "SCMP_ARCH_AARCH64" => 0xc00000b7,
        "SCMP_ARCH_ARM" => 0x40000028,
        _ => CURRENT_ARCH,
    }
}

// ===========================================================================
// Apply seccomp
// ===========================================================================

/// Apply seccomp filtering.
///
/// If spec seccomp rules are provided, those are used.
/// Otherwise, the built-in allow-list is applied.
/// Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
pub fn apply_seccomp() -> io::Result<()> {
    let prog = seccomp_bpf_prog();
    let ret = unsafe { do_seccomp(
        SECCOMP_SET_MODE_FILTER,
        SECCOMP_FILTER_FLAG_TSYNC,
        prog.as_ptr() as *const c_void,
    ) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

/// Apply seccomp filtering from OCI spec rules.
///
/// If `spec` is None or has no syscalls, falls back to the built-in allow-list.
/// Requires prctl(PR_SET_NO_NEW_PRIVS, 1) first.
/// Note: Does NOT use TSYNC flag — the container child is single-threaded at this
/// point (just forked). TSYNC requires CAP_SYS_ADMIN even with no_new_privs.
pub fn apply_seccomp_from_spec(spec: Option<&OciLinuxSeccomp>) -> io::Result<()> {
    let has_rules = spec.as_ref()
        .and_then(|s| s.syscalls.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let prog = if has_rules {
        build_seccomp_prog(spec.unwrap())
    } else {
        seccomp_bpf_prog()
    };

    let ret = unsafe { do_seccomp(
        SECCOMP_SET_MODE_FILTER,
        0, // no flags — single-threaded child, TSYNC not needed
        prog.as_ptr() as *const c_void,
    ) };
    if ret == 0 { Ok(()) } else { Err(io::Error::last_os_error()) }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::OciSeccompSyscallEntry;

    #[test]
    fn seccomp_bpf_prog_is_non_empty() {
        let prog = seccomp_bpf_prog();
        assert!(!prog.is_empty());
    }

    #[test]
    fn seccomp_bpf_prog_has_valid_structure() {
        let prog = seccomp_bpf_prog();
        assert!(prog.len() >= 16);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len > 50);
    }

    #[test]
    fn seccomp_bpf_prog_contains_allow_and_deny() {
        let prog = seccomp_bpf_prog();
        assert!(prog.len() >= 16, "sock_fprog should be at least 16 bytes");
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len > 50, "should have many BPF instructions, got {}", len);

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0, "filter pointer should be non-null");

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_allow = false;
            let mut found_deny = false;
            for insn in insns {
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if k == 0x7fff0000 { found_allow = true; }
                if k == 0x00050001 { found_deny = true; }
            }
            assert!(found_allow, "should contain RET_ALLOW (0x7fff0000)");
            assert!(found_deny, "should contain RET_ERRNO(EPERM) (0x00050001)");
        }
    }

    #[test]
    fn bpf_insn_produces_8_bytes() {
        let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(insn.len(), 8);
    }

    #[test]
    fn bpf_insn_ret_allow_encoding() {
        let insn = bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(&insn[0..2], &[0x06, 0x00]);
        assert_eq!(&insn[4..8], &[0x00, 0x00, 0xff, 0x7f]);
    }

    #[test]
    fn build_seccomp_prog_with_spec_rules() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Kill),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["getcwd".into(), "chmod".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: None,
                },
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Errno),
                    errno_ret: Some(13), // EACCES
                    args: None,
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        assert!(prog.len() >= 16);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len > 5, "should have BPF instructions, got {}", len);
    }

    #[test]
    fn build_seccomp_prog_empty_spec_uses_fallback() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Kill),
            default_errno_ret: None,
            architectures: None,
            listener_path: None,
            listener_metadata: None,
            syscalls: None,
        };
        let prog = build_seccomp_prog(&spec);
        // Should still generate a valid program even with empty rules
        assert!(prog.len() >= 16);
    }

    #[test]
    fn build_seccomp_prog_with_arg_filters() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Kill),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Errno),
                    errno_ret: Some(13),
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 1,
                        value: 0o100000,
                        value_two: 0,
                        op: "SCMP_CMP_EQ".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        assert!(prog.len() >= 16);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8, "should have many BPF instructions, got {}", len);
    }

    #[test]
    fn build_seccomp_prog_with_ne_arg_filter() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            default_errno_ret: None,
            architectures: None,
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["ioctl".into()]),
                    action: Some(OciSeccompAction::Kill),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 1,
                        value: 0x5401,
                        value_two: 0,
                        op: "SCMP_CMP_NE".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 5, "should have BPF instructions, got {}", len);
    }

    #[test]
    fn action_to_bpf_values() {
        assert_eq!(action_to_bpf(&OciSeccompAction::Allow, None), 0x7fff0000);
        assert_eq!(action_to_bpf(&OciSeccompAction::Kill, None), 0x00000000);
        assert_eq!(action_to_bpf(&OciSeccompAction::Errno, Some(1)), 0x00050001);
        assert_eq!(action_to_bpf(&OciSeccompAction::Errno, Some(13)), 0x0005000d);
    }

    #[test]
    fn syscall_nr_known_for_common_calls() {
        // On x86_64
        #[cfg(target_arch = "x86_64")]
        {
            assert_eq!(syscall_nr("read"), Some(0));
            assert_eq!(syscall_nr("write"), Some(1));
            assert_eq!(syscall_nr("exit"), Some(60));
            assert_eq!(syscall_nr("getpid"), Some(39));
        }
        // On aarch64
        #[cfg(target_arch = "aarch64")]
        {
            assert_eq!(syscall_nr("read"), Some(63));
            assert_eq!(syscall_nr("write"), Some(64));
            assert_eq!(syscall_nr("exit"), Some(93));
            assert_eq!(syscall_nr("getpid"), Some(172));
        }
    }

    /// Verify that LT uses JGE (0x30) with jt=skip, jf=0
    #[test]
    fn bpf_lt_uses_jge() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Kill),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_LT".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8, "should have multiple instructions");

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0);

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_jge_for_lt = false;
            let mut found_jgt_for_hi = false;
            for insn in insns {
                let code = u16::from_le_bytes([insn[0], insn[1]]);
                let jt = insn[2];
                let jf = insn[3];
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                // JGE with jt=skip, jf=0 for lo check (LT: skip when >=)
                if code == 0x30 && jt > 0 && jf == 0 && k == 0x100 {
                    found_jge_for_lt = true;
                }
                // JGT with jt=1, jf=0 for hi check
                if code == 0x25 && jt == 1 && jf == 0 && k == 0 {
                    found_jgt_for_hi = true;
                }
            }
            assert!(found_jge_for_lt, "LT should use JGE (0x30) with jt=skip, jf=0 for lo");
            assert!(found_jgt_for_hi, "LT should use JGT (0x25) with jt=1, jf=0 for hi");
        }
    }

    /// Verify that GT uses JGT (0x25) with jt=0, jf=skip for hi check
    #[test]
    fn bpf_gt_uses_jgt() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_GT".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8);

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0);

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_jgt_lo = false;
            let mut found_jgt_hi = false;
            for insn in insns {
                let code = u16::from_le_bytes([insn[0], insn[1]]);
                let jt = insn[2];
                let jf = insn[3];
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if code == 0x25 && jt == 0 && jf > 0 && k == 0x100 {
                    found_jgt_lo = true;
                }
                if code == 0x25 && jt == 0 && jf == 1 && k == 0 {
                    found_jgt_hi = true;
                }
            }
            assert!(found_jgt_lo, "GT should use JGT (0x25) with jt=0, jf=skip for lo");
            assert!(found_jgt_hi, "GT should use JGT (0x25) with jt=0, jf=1 for hi");
        }
    }

    /// Verify GE uses JGE (0x30)
    #[test]
    fn bpf_ge_uses_jge() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_GE".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8);

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0);

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_jge_lo = false;
            let mut found_jge_hi = false;
            for insn in insns {
                let code = u16::from_le_bytes([insn[0], insn[1]]);
                let jt = insn[2];
                let jf = insn[3];
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if code == 0x30 && jt == 0 && jf > 0 && k == 0x100 {
                    found_jge_lo = true;
                }
                if code == 0x30 && jt == 0 && jf == 1 && k == 0 {
                    found_jge_hi = true;
                }
            }
            assert!(found_jge_lo, "GE should use JGE (0x30) with jt=0, jf=skip for lo");
            assert!(found_jge_hi, "GE should use JGE (0x30) with jt=0, jf=1 for hi");
        }
    }

    /// Verify LE uses JGT (0x25)
    #[test]
    fn bpf_le_uses_jgt() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 0, value: 0x100, value_two: 0, op: "SCMP_CMP_LE".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8);

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0);

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_jgt_lo = false;
            let mut found_jgt_hi = false;
            for insn in insns {
                let code = u16::from_le_bytes([insn[0], insn[1]]);
                let jt = insn[2];
                let jf = insn[3];
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if code == 0x25 && jt > 0 && jf == 0 && k == 0x100 {
                    found_jgt_lo = true;
                }
                if code == 0x25 && jt == 1 && jf == 0 && k == 0 {
                    found_jgt_hi = true;
                }
            }
            assert!(found_jgt_lo, "LE should use JGT (0x25) with jt=skip, jf=0 for lo");
            assert!(found_jgt_hi, "LE should use JGT (0x25) with jt=1, jf=0 for hi");
        }
    }

    /// Verify MASKED_EQ uses AND (0x50) + JEQ (0x15) for both halves
    #[test]
    fn bpf_masked_eq_uses_and_then_jeq() {
        let spec = OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            default_errno_ret: None,
            architectures: Some(vec!["SCMP_ARCH_X86_64".into()]),
            listener_path: None,
            listener_metadata: None,
            syscalls: Some(vec![
                OciSeccompSyscallEntry {
                    names: Some(vec!["openat".into()]),
                    action: Some(OciSeccompAction::Allow),
                    errno_ret: None,
                    args: Some(vec![crate::json::OciSeccompArg {
                        index: 0, value: 0xFF, value_two: 0x42, op: "SCMP_CMP_MASKED_EQ".into(),
                    }]),
                },
            ]),
        };
        let prog = build_seccomp_prog(&spec);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len >= 8);

        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0);

        unsafe {
            let insns = std::slice::from_raw_parts(ptr as *const [u8; 8], len);
            let mut found_and_lo = false;
            let mut found_jeq_lo = false;
            for insn in insns {
                let code = u16::from_le_bytes([insn[0], insn[1]]);
                let k = u32::from_le_bytes([insn[4], insn[5], insn[6], insn[7]]);
                if code == 0x50 && k == 0xFF { found_and_lo = true; }
                if code == 0x15 && k == 0x42 { found_jeq_lo = true; }
            }
            assert!(found_and_lo, "MASKED_EQ should use AND (0x50) with mask");
            assert!(found_jeq_lo, "MASKED_EQ should use JEQ (0x15) with expected value");
        }
    }
}
