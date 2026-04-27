//! Features command implementation — outputs supported features as OCI features JSON.

use crate::prelude::*;
use std::io;

pub fn cmd_features(_opts: &crate::cli::GlobalOpts, _args: &[String]) -> io::Result<()> {
    // Build the features JSON matching OCI runtime spec v1.0.2
    let features = features_json();
    print!("{}", features);
    Ok(())
}

fn features_json() -> String {
    // Build manually to avoid adding a new struct
    let mut f = String::new();
    f.push_str("{\n");
    f.push_str("  \"ociVersionMin\": \"1.0.0\",\n");
    f.push_str("  \"ociVersionMax\": \"1.0.2\",\n");
    f.push_str("  \"hooks\": [\n");
    f.push_str("    \"prestart\",\n");
    f.push_str("    \"createRuntime\",\n");
    f.push_str("    \"createContainer\",\n");
    f.push_str("    \"startContainer\",\n");
    f.push_str("    \"poststart\",\n");
    f.push_str("    \"poststop\"\n");
    f.push_str("  ],\n");
    f.push_str("  \"mountOptions\": [\"ro\", \"rw\", \"nosuid\", \"nodev\", \"noexec\", \"relatime\", \"strictatime\", \"nosymfollow\"],\n");
    f.push_str("  \"linux\": {\n");
    f.push_str("    \"namespaces\": [\n");
    f.push_str("      \"cgroup\",\n");
    f.push_str("      \"ipc\",\n");
    f.push_str("      \"mount\",\n");
    f.push_str("      \"network\",\n");
    f.push_str("      \"pid\",\n");
    f.push_str("      \"user\",\n");
    f.push_str("      \"uts\"\n");
    f.push_str("    ],\n");
    f.push_str("    \"capabilities\": [\n");
    f.push_str("      \"CAP_CHOWN\",\n");
    f.push_str("      \"CAP_DAC_OVERRIDE\",\n");
    f.push_str("      \"CAP_DAC_READ_SEARCH\",\n");
    f.push_str("      \"CAP_FOWNER\",\n");
    f.push_str("      \"CAP_FSETID\",\n");
    f.push_str("      \"CAP_KILL\",\n");
    f.push_str("      \"CAP_SETGID\",\n");
    f.push_str("      \"CAP_SETUID\",\n");
    f.push_str("      \"CAP_SETPCAP\",\n");
    f.push_str("      \"CAP_LINUX_IMMUTABLE\",\n");
    f.push_str("      \"CAP_NET_BIND_SERVICE\",\n");
    f.push_str("      \"CAP_NET_BROADCAST\",\n");
    f.push_str("      \"CAP_NET_ADMIN\",\n");
    f.push_str("      \"CAP_NET_RAW\",\n");
    f.push_str("      \"CAP_IPC_LOCK\",\n");
    f.push_str("      \"CAP_IPC_OWNER\",\n");
    f.push_str("      \"CAP_SYS_MODULE\",\n");
    f.push_str("      \"CAP_SYS_RAWIO\",\n");
    f.push_str("      \"CAP_SYS_CHROOT\",\n");
    f.push_str("      \"CAP_SYS_PTRACE\",\n");
    f.push_str("      \"CAP_SYS_PACCT\",\n");
    f.push_str("      \"CAP_SYS_ADMIN\",\n");
    f.push_str("      \"CAP_SYS_BOOT\",\n");
    f.push_str("      \"CAP_SYS_NICE\",\n");
    f.push_str("      \"CAP_SYS_RESOURCE\",\n");
    f.push_str("      \"CAP_SYS_TIME\",\n");
    f.push_str("      \"CAP_SYS_TTY_CONFIG\",\n");
    f.push_str("      \"CAP_MKNOD\",\n");
    f.push_str("      \"CAP_LEASE\",\n");
    f.push_str("      \"CAP_AUDIT_WRITE\",\n");
    f.push_str("      \"CAP_AUDIT_CONTROL\",\n");
    f.push_str("      \"CAP_SETFCAP\",\n");
    f.push_str("      \"CAP_MAC_OVERRIDE\",\n");
    f.push_str("      \"CAP_MAC_ADMIN\",\n");
    f.push_str("      \"CAP_SYSLOG\",\n");
    f.push_str("      \"CAP_WAKE_ALARM\",\n");
    f.push_str("      \"CAP_BLOCK_SUSPEND\",\n");
    f.push_str("      \"CAP_AUDIT_READ\",\n");
    f.push_str("      \"CAP_PERFMON\",\n");
    f.push_str("      \"CAP_BPF\",\n");
    f.push_str("      \"CAP_CHECKPOINT_RESTORE\"\n");
    f.push_str("    ],\n");
    f.push_str("    \"cgroup\": {\n");
    f.push_str("      \"v1\": false,\n");
    f.push_str("      \"v2\": true,\n");
    f.push_str("      \"systemd\": false,\n");
    f.push_str("      \"rdma\": false\n");
    f.push_str("    },\n");
    f.push_str("    \"seccomp\": {\n");
    f.push_str("      \"enabled\": true,\n");
    f.push_str("      \"actions\": [\"SCMP_ACT_ALLOW\", \"SCMP_ACT_ERRNO\", \"SCMP_ACT_KILL\", \"SCMP_ACT_KILL_PROCESS\", \"SCMP_ACT_KILL_THREAD\", \"SCMP_ACT_TRAP\", \"SCMP_ACT_TRACE\", \"SCMP_ACT_NOTIFY\", \"SCMP_ACT_LOG\"],\n");
    f.push_str("      \"operators\": [\"SCMP_CMP_NE\", \"SCMP_CMP_EQ\", \"SCMP_CMP_LT\", \"SCMP_CMP_LE\", \"SCMP_CMP_GT\", \"SCMP_CMP_GE\", \"SCMP_CMP_MASKED_EQ\"]\n");
    f.push_str("    },\n");
    f.push_str("    \"apparmor\": {\n");
    f.push_str("      \"enabled\": true\n");
    f.push_str("    },\n");
    f.push_str("    \"selinux\": {\n");
    f.push_str("      \"enabled\": true\n");
    f.push_str("    }\n");
    f.push_str("  },\n");
    f.push_str("  \"potentiallyUnsafeConfigAnnotations\": [],\n");
    f.push_str("  \"annotations\": {},\n");
    f.push_str("  \"rootless\": {\n");
    f.push_str("    \"supported\": true,\n");
    f.push_str(&format!("    \"uid\": {}\n", unsafe { libc::getuid() }));
    f.push_str("  }\n");
    f.push('}');
    f
}
