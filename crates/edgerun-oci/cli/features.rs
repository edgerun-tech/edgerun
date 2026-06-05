//! Features command implementation — outputs supported features as OCI features JSON.

use crate::libc;
use crate::prelude::*;
use std::io;

pub fn cmd_features(_opts: &crate::cli::GlobalOpts, _args: &[String]) -> io::Result<()> {
    // Build the features JSON matching OCI runtime spec v1.0.2
    let features = features_json();
    print!("{}", features);
    Ok(())
}

fn features_json() -> String {
    format!(
        concat!(
            "{{\n",
            "  \"ociVersionMin\":\"1.0.0\",\n",
            "  \"ociVersionMax\":\"1.0.2\",\n",
            "  \"hooks\":[\"prestart\",\"createRuntime\",\"createContainer\",\"startContainer\",\"poststart\",\"poststop\"],\n",
            "  \"mountOptions\":[\"ro\",\"rw\",\"nosuid\",\"nodev\",\"noexec\",\"relatime\",\"strictatime\",\"nosymfollow\"],\n",
            "  \"linux\":{{\n",
            "    \"namespaces\":[\"cgroup\",\"ipc\",\"mount\",\"network\",\"pid\",\"user\",\"uts\"],\n",
            "    \"capabilities\":[\"CAP_CHOWN\",\"CAP_DAC_OVERRIDE\",\"CAP_DAC_READ_SEARCH\",\"CAP_FOWNER\",\"CAP_FSETID\",\"CAP_KILL\",\"CAP_SETGID\",\"CAP_SETUID\",\"CAP_SETPCAP\",\"CAP_LINUX_IMMUTABLE\",\"CAP_NET_BIND_SERVICE\",\"CAP_NET_BROADCAST\",\"CAP_NET_ADMIN\",\"CAP_NET_RAW\",\"CAP_IPC_LOCK\",\"CAP_IPC_OWNER\",\"CAP_SYS_MODULE\",\"CAP_SYS_RAWIO\",\"CAP_SYS_CHROOT\",\"CAP_SYS_PTRACE\",\"CAP_SYS_PACCT\",\"CAP_SYS_ADMIN\",\"CAP_SYS_BOOT\",\"CAP_SYS_NICE\",\"CAP_SYS_RESOURCE\",\"CAP_SYS_TIME\",\"CAP_SYS_TTY_CONFIG\",\"CAP_MKNOD\",\"CAP_LEASE\",\"CAP_AUDIT_WRITE\",\"CAP_AUDIT_CONTROL\",\"CAP_SETFCAP\",\"CAP_MAC_OVERRIDE\",\"CAP_MAC_ADMIN\",\"CAP_SYSLOG\",\"CAP_WAKE_ALARM\",\"CAP_BLOCK_SUSPEND\",\"CAP_AUDIT_READ\",\"CAP_PERFMON\",\"CAP_BPF\",\"CAP_CHECKPOINT_RESTORE\"],\n",
            "    \"cgroup\":{{\"v1\":false,\"v2\":true,\"systemd\":false,\"rdma\":false}},\n",
            "    \"seccomp\":{{\"enabled\":true,\"actions\":[\"SCMP_ACT_ALLOW\",\"SCMP_ACT_ERRNO\",\"SCMP_ACT_KILL\",\"SCMP_ACT_KILL_PROCESS\",\"SCMP_ACT_KILL_THREAD\",\"SCMP_ACT_TRAP\",\"SCMP_ACT_TRACE\",\"SCMP_ACT_NOTIFY\",\"SCMP_ACT_LOG\"],\"operators\":[\"SCMP_CMP_NE\",\"SCMP_CMP_EQ\",\"SCMP_CMP_LT\",\"SCMP_CMP_LE\",\"SCMP_CMP_GT\",\"SCMP_CMP_GE\",\"SCMP_CMP_MASKED_EQ\"]}},\n",
            "    \"apparmor\":{{\"enabled\":true}},\n",
            "    \"selinux\":{{\"enabled\":true}}\n",
            "  }},\n",
            "  \"potentiallyUnsafeConfigAnnotations\":[],\n",
            "  \"annotations\":{{}},\n",
            "  \"rootless\":{{\"supported\":true,\"uid\":{}}}\n",
            "}}"
        ),
        unsafe { libc::getuid() }
    )
}
