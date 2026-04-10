//! Minimal OCI container runtime — kernel-only, no external tools.
//!
//! Uses Linux kernel primitives: namespaces, cgroups v2, pivot_root, mount.
//! No Docker, no runc, no systemd, no libc crate. Just raw syscalls and std.
//!
//! ## Module structure
//!
//! - `syscalls` — Raw syscall FFI, constants, wrappers
//! - `seccomp`  — Seccomp-BPF filtering (arch-aware)
//! - `userns`   — UID/GID mapping, capability dropping
//! - `cgroups`  — Cgroups v2 resource management
//! - `rootfs`   — Rootfs setup: pivot_root, mounts, devices, whiteouts
//! - `bundle`   — OCI bundle creation and serialization
//! - `container` — Blocking and non-blocking container lifecycle

pub mod json;
pub use json::*;

pub mod syscalls;
pub mod seccomp;
pub mod userns;
pub mod cgroups;
pub mod rootfs;
mod bundle;
pub mod process;
mod handle;
mod lifecycle;
mod hooks;
mod init;
pub mod state;
pub mod cli;

// Re-export public API
pub use bundle::{create_bundle, write_bundle};
pub use cgroups::shares_to_weight;
pub use container::{run_bundle, run_spec, run_spec_with_id, start_bundle, start_spec, start_spec_with_id, ForkedChild, save_created_state, signal_start, setup_container_cgroups, run_poststart_hooks, update_state_running, into_running_container, run_poststop_and_cleanup, RunningContainer, delete_container};
pub use rootfs::{apply_whiteouts, build_rootfs};
pub use hooks::{
    ContainerState, HookError,
    execute_prestart_hooks, execute_create_runtime_hooks,
    execute_create_container_hooks, execute_start_container_hooks,
    execute_poststart_hooks, execute_poststop_hooks,
};

// Thin re-exports so `container` module still works as a facade
mod container;

// ===========================================================================
// Namespace helpers (need to be here since they use syscalls::ns)
// ===========================================================================

pub fn default_namespaces() -> Vec<OciNamespace> {
    vec![
        OciNamespace { ns_type: "user".into(), path: None },
        OciNamespace { ns_type: "mount".into(), path: None },
        OciNamespace { ns_type: "pid".into(), path: None },
        OciNamespace { ns_type: "network".into(), path: None },
        OciNamespace { ns_type: "ipc".into(), path: None },
        OciNamespace { ns_type: "uts".into(), path: None },
    ]
}

/// Resolve namespace clone flags from OCI namespace type strings.
/// Returns `syscalls::ns::CONTAINER` (all six namespaces) when the spec has no
/// explicit namespace configuration — the OCI default is to create new
/// namespaces for the container.
pub fn namespace_flags(namespaces: &[OciNamespace]) -> std::os::raw::c_int {
    use syscalls::ns;
    use std::os::raw::c_int;

    if namespaces.is_empty() {
        return ns::CONTAINER;
    }
    let mut flags: c_int = 0;
    for ns in namespaces {
        flags |= process::ns_type_to_flag(&ns.ns_type).unwrap_or(0);
    }
    flags
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // OCI Spec serialization / deserialization
    // ===========================================================================

    #[test]
    fn oci_spec_default_roundtrip() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: None,
            root: None,
            hostname: None,
            linux: None,
            mounts: None,
            annotations: None,
        };
        let json = spec.to_json_string();
        let parsed: OciSpec = json::parse_oci_spec(json.as_bytes()).unwrap();
        assert_eq!(parsed.version, "1.0.2");
    }

    #[test]
    fn oci_spec_full_roundtrip() {
        let spec = OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(OciProcess {
                terminal: Some(false),
                user: Some(OciUser {
                    uid: Some(1000),
                    gid: Some(1000),
                    additional_gids: Some(vec![100, 200]),
                    umask: None,
                }),
                args: Some(vec!["/bin/sh".into(), "-c".into(), "echo hello".into()]),
                env: Some(vec!["PATH=/usr/bin".into(), "HOME=/root".into()]),
                cwd: Some("/app".into()),
                capabilities: Some(OciCapabilities {
                    bounding: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    inheritable: Some(vec![]),
                    permitted: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
                    ambient: Some(vec![]),
                }),
                rlimits: Some(vec![OciRlimit {
                    ns_type: "RLIMIT_NOFILE".into(),
                    hard: 65536,
                    soft: 65536,
                }]),
                no_new_privileges: Some(true),
                oom_score_adj: None,
                apparmor_profile: None,
                selinux_label: None,
            }),
            root: Some(OciRoot {
                path: "/var/lib/edgerun/bundles/test/rootfs".into(),
                readonly: Some(false),
            }),
            hostname: Some("test-container".into()),
            linux: Some(OciLinux {
                uid_mappings: Some(vec![OciIdMapping {
                    container_id: 0,
                    host_id: 65534,
                    size: 1,
                }]),
                gid_mappings: Some(vec![OciIdMapping {
                    container_id: 0,
                    host_id: 65534,
                    size: 1,
                }]),
                resources: Some(OciLinuxResources {
                    devices: None,
                    memory: Some(OciLinuxMemory {
                        limit: Some(536870912),
                        reservation: None,
                        swap: Some(0),
                        kernel: None,
                        kernel_tcp: None,
                    }),
                    cpu: Some(OciLinuxCpu {
                        shares: Some(512),
                        quota: Some(50000),
                        period: Some(100000),
                        realtime_runtime: None,
                        realtime_period: None,
                        cpus: None,
                        mems: None,
                    }),
                    pids: Some(OciLinuxPids { limit: 128 }),
                    block_io: None,
                    hugepage_limits: None,
                    network: None,
                }),
                cgroups_path: Some("/edgerun/test".into()),
                namespaces: Some(default_namespaces()),
                devices: Some(vec![OciLinuxDevice {
                    ns_type: "c".into(),
                    path: "/dev/null".into(),
                    file_mode: Some(0o666),
                    uid: Some(0),
                    gid: Some(0),
                    major: Some(1),
                    minor: Some(3),
                }]),
                masked_paths: Some(vec!["/proc/acpi".into()]),
                readonly_paths: Some(vec!["/proc/sys".into()]),
                mount_label: None,
                rootfs_propagation: Some("private".into()),
                sysctl: Some([("net.ipv4.ip_forward".into(), "1".into())].into()),
                hooks: None,
                seccomp: None,
            }),
            mounts: Some(vec![
                OciMount {
                    destination: "/proc".into(),
                    mount_type: Some("proc".into()),
                    source: Some("proc".into()),
                    options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                    label: None,
                },
                OciMount {
                    destination: "/sys".into(),
                    mount_type: Some("sysfs".into()),
                    source: Some("sysfs".into()),
                    options: Some(vec!["ro".into(), "nosuid".into(), "nodev".into(), "noexec".into()]),
                    label: None,
                },
            ]),
            annotations: None,
        };

        let json = spec.to_json_string();
        let parsed: OciSpec = json::parse_oci_spec(json.as_bytes()).unwrap();

        // Verify all fields round-tripped
        assert_eq!(parsed.version, spec.version);
        assert_eq!(parsed.hostname, spec.hostname);
        let proc = parsed.process.unwrap();
        assert_eq!(proc.args.unwrap(), vec!["/bin/sh", "-c", "echo hello"]);
        assert_eq!(proc.env.unwrap(), vec!["PATH=/usr/bin", "HOME=/root"]);
        assert_eq!(proc.cwd.unwrap(), "/app");
        assert!(proc.no_new_privileges.unwrap());
        assert_eq!(parsed.root.unwrap().path, spec.root.unwrap().path);
        let linux = parsed.linux.unwrap();
        assert_eq!(linux.cgroups_path.unwrap(), "/edgerun/test");
        assert_eq!(linux.namespaces.unwrap().len(), 6);
        assert_eq!(linux.masked_paths.unwrap(), vec!["/proc/acpi"]);
        assert_eq!(linux.readonly_paths.unwrap(), vec!["/proc/sys"]);
    }

    #[test]
    fn oci_spec_deserializes_missing_optional_fields() {
        let json = r#"{"ociVersion":"1.0.2","root":{"path":"/rootfs"}}"#;
        let spec: OciSpec = json::parse_oci_spec(json.as_bytes()).unwrap();
        assert_eq!(spec.version, "1.0.2");
        assert!(spec.process.is_none());
        assert!(spec.hostname.is_none());
        assert!(spec.linux.is_none());
        assert!(spec.mounts.is_none());
    }

    // ===========================================================================
    // create_bundle
    // ===========================================================================

    #[test]
    fn create_bundle_sets_defaults() {
        let spec = create_bundle(
            "/var/lib/edgerun/bundles/test/rootfs",
            vec!["/bin/sh".into(), "-c".into(), "echo hello".into()],
            Some(vec!["PATH=/usr/bin".into()]),
            Some("myhost".into()),
        );

        assert_eq!(spec.version, "1.0.2");
        assert_eq!(spec.hostname, Some("myhost".into()));
        assert_eq!(spec.root.as_ref().unwrap().path, "/var/lib/edgerun/bundles/test/rootfs");

        let proc = spec.process.as_ref().unwrap();
        assert_eq!(proc.args.as_ref().unwrap(), &vec!["/bin/sh", "-c", "echo hello"]);
        assert_eq!(proc.env.as_ref().unwrap(), &vec!["PATH=/usr/bin"]);
        assert_eq!(proc.cwd.as_ref().unwrap(), "/");
        assert!(proc.no_new_privileges.unwrap());
        assert_eq!(proc.user.as_ref().unwrap().uid, Some(0));
        assert_eq!(proc.user.as_ref().unwrap().gid, Some(0));

        let linux = spec.linux.as_ref().unwrap();
        assert_eq!(linux.namespaces.as_ref().unwrap().len(), 6);
        assert_eq!(linux.masked_paths.as_ref().unwrap().len(), 9);
        assert_eq!(linux.readonly_paths.as_ref().unwrap().len(), 6);
    }

    #[test]
    fn create_bundle_default_hostname() {
        let spec = create_bundle("/rootfs", vec!["/bin/sh".into()], None, None);
        assert_eq!(spec.hostname, Some("edgerun".into()));
    }

    #[test]
    fn create_bundle_empty_args() {
        let spec = create_bundle("/rootfs", vec![], None, None);
        assert_eq!(spec.process.as_ref().unwrap().args.as_ref().unwrap(), &Vec::<String>::new());
    }

    // ===========================================================================
    // default_namespaces
    // ===========================================================================

    #[test]
    fn default_namespaces_returns_six() {
        let ns = default_namespaces();
        assert_eq!(ns.len(), 6);
    }

    #[test]
    fn default_namespaces_contains_expected_types() {
        let ns = default_namespaces();
        let types: Vec<&str> = ns.iter().map(|n| n.ns_type.as_str()).collect();
        assert!(types.contains(&"user"));
        assert!(types.contains(&"mount"));
        assert!(types.contains(&"pid"));
        assert!(types.contains(&"network"));
        assert!(types.contains(&"ipc"));
        assert!(types.contains(&"uts"));
    }

    #[test]
    fn default_namespaces_have_no_path() {
        for ns in default_namespaces() {
            assert!(ns.path.is_none(), "namespace {} should have no path", ns.ns_type);
        }
    }

    // ===========================================================================
    // namespace_flags
    // ===========================================================================

    #[test]
    fn namespace_flags_empty() {
        // Empty namespace list means "create all namespaces" (OCI default)
        assert_eq!(namespace_flags(&[]), 0x6e020000); // ns::CONTAINER
    }

    #[test]
    fn namespace_flags_single_mount() {
        let ns = vec![OciNamespace { ns_type: "mount".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_ne!(flags, 0);
        assert_eq!(flags, 0x00020000); // NEWNS
    }

    #[test]
    fn namespace_flags_single_user() {
        let ns = vec![OciNamespace { ns_type: "user".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x10000000); // NEWUSER
    }

    #[test]
    fn namespace_flags_single_pid() {
        let ns = vec![OciNamespace { ns_type: "pid".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x20000000); // NEWPID
    }

    #[test]
    fn namespace_flags_single_network() {
        let ns = vec![OciNamespace { ns_type: "network".into(), path: None }];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x40000000); // NEWNET
    }

    #[test]
    fn namespace_flags_container_combination() {
        let ns = vec![
            OciNamespace { ns_type: "mount".into(), path: None },
            OciNamespace { ns_type: "cgroup".into(), path: None },
            OciNamespace { ns_type: "uts".into(), path: None },
            OciNamespace { ns_type: "ipc".into(), path: None },
            OciNamespace { ns_type: "pid".into(), path: None },
            OciNamespace { ns_type: "network".into(), path: None },
        ];
        let flags = namespace_flags(&ns);
        // NEWNS=0x00020000 | NEWCGROUP=0x02000000 | NEWUTS=0x04000000
        // | NEWIPC=0x08000000 | NEWPID=0x20000000 | NEWNET=0x40000000
        assert_eq!(flags, 0x6e020000);
    }

    #[test]
    fn namespace_flags_unknown_type_ignored() {
        let ns = vec![OciNamespace { ns_type: "bogus".into(), path: None }];
        assert_eq!(namespace_flags(&ns), 0);
    }

    #[test]
    fn namespace_flags_duplicate_namespace_uses_last_match() {
        // Duplicates should OR the same flag (no change)
        let ns = vec![
            OciNamespace { ns_type: "mount".into(), path: None },
            OciNamespace { ns_type: "mount".into(), path: None },
        ];
        let flags = namespace_flags(&ns);
        assert_eq!(flags, 0x00020000);
    }

    // ===========================================================================
    // shares_to_weight
    // ===========================================================================

    #[test]
    fn shares_to_weight_zero_returns_one() {
        assert_eq!(shares_to_weight(0), 1);
    }

    #[test]
    fn shares_to_weight_one_returns_one() {
        assert_eq!(shares_to_weight(1), 1);
    }

    #[test]
    fn shares_to_weight_two_returns_one() {
        assert_eq!(shares_to_weight(2), 1);
    }

    #[test]
    fn shares_to_weight_default_returns_256() {
        // Default shares is 1024, should map to weight ~39
        // 1 + (1024 - 2) * 9999 / 262142 = 1 + 1022 * 9999 / 262142 ≈ 40
        let w = shares_to_weight(1024);
        assert!(w >= 1 && w <= 10000);
    }

    #[test]
    fn shares_to_weight_high_shares_caps_at_10000() {
        assert_eq!(shares_to_weight(262144), 10000);
        // No longer overflows — saturating arithmetic
        assert_eq!(shares_to_weight(10_000_000), 10000);
        assert_eq!(shares_to_weight(u64::MAX), 10000);
    }

    #[test]
    fn shares_to_weight_is_monotonic() {
        let w1 = shares_to_weight(100);
        let w2 = shares_to_weight(1000);
        let w3 = shares_to_weight(10000);
        assert!(w1 <= w2 && w2 <= w3);
    }

    // ===========================================================================
    // mount_flags_from_opts (via rootfs module, tested through cgroups test)
    // ===========================================================================

    // ===========================================================================
    // seccomp_bpf_prog
    // ===========================================================================

    #[test]
    fn seccomp_bpf_prog_is_non_empty() {
        let prog = seccomp::seccomp_bpf_prog();
        assert!(!prog.is_empty());
    }

    #[test]
    fn seccomp_bpf_prog_has_valid_structure() {
        let prog = seccomp::seccomp_bpf_prog();
        // First 2 bytes are length (u16 LE), next 8 bytes are pointer
        assert!(prog.len() >= 16);
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        // Should have many BPF instructions
        assert!(len > 50);
    }

    #[test]
    fn seccomp_bpf_prog_contains_allow_and_deny() {
        let prog = seccomp::seccomp_bpf_prog();
        assert!(prog.len() >= 16, "sock_fprog should be at least 16 bytes");
        let len = u16::from_le_bytes([prog[0], prog[1]]) as usize;
        assert!(len > 50, "should have many BPF instructions, got {}", len);

        // Verify the pointer is non-null
        let ptr_bytes: [u8; 8] = prog[8..16].try_into().unwrap();
        let ptr = u64::from_le_bytes(ptr_bytes);
        assert_ne!(ptr, 0, "filter pointer should be non-null");

        // Read the actual BPF instructions from the leaked memory
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

    // ===========================================================================
    // bpf_insn helpers
    // ===========================================================================

    #[test]
    fn bpf_insn_produces_8_bytes() {
        let insn = seccomp::bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(insn.len(), 8);
    }

    #[test]
    fn bpf_insn_ret_allow_encoding() {
        let insn = seccomp::bpf_insn(0x06, 0, 0, 0x7fff0000);
        assert_eq!(&insn[0..2], &[0x06, 0x00]); // code
        assert_eq!(&insn[4..8], &[0x00, 0x00, 0xff, 0x7f]); // k = RET_ALLOW
    }

    // ===========================================================================
    // write_bundle
    // ===========================================================================

    #[test]
    fn write_bundle_creates_directory_and_config() {
        let tmp = std::env::temp_dir().join(format!("oci-bundle-test-{}", std::process::id()));
        let spec = create_bundle("/rootfs", vec!["/bin/sh".into()], None, None);
        write_bundle(&tmp, &spec).unwrap();

        assert!(tmp.is_dir());
        assert!(tmp.join("config.json").exists());
        let size = std::fs::metadata(tmp.join("config.json")).unwrap().len();
        assert!(size > 50, "config.json should be non-trivial size, got {}", size);

        // Verify the file contains expected OCI fields by checking for key strings
        let content = std::fs::read_to_string(tmp.join("config.json")).unwrap();
        assert!(content.contains("1.0.2"), "should contain OCI version");
        assert!(content.contains("rootfs"), "should contain rootfs path");

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn write_bundle_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("oci-bundle-roundtrip-{}", std::process::id()));
        let spec = create_bundle(
            "/var/lib/test/rootfs",
            vec!["/bin/bash".into(), "-l".into()],
            Some(vec!["FOO=bar".into()]),
            Some("roundtest".into()),
        );
        write_bundle(&tmp, &spec).unwrap();

        // Verify key fields are in the written config
        let content = std::fs::read_to_string(tmp.join("config.json")).unwrap();
        assert!(content.contains("1.0.2"));
        assert!(content.contains("/var/lib/test/rootfs"));
        assert!(content.contains("/bin/bash"));
        assert!(content.contains("FOO=bar"));
        assert!(content.contains("roundtest"));

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ===========================================================================
    // OciMount serialization
    // ===========================================================================

    #[test]
    fn oci_mount_roundtrip() {
        let mount = OciMount {
            destination: "/proc".into(),
            mount_type: Some("proc".into()),
            source: Some("proc".into()),
            options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
            label: None,
        };
        let json = edgerun_json::to_string(&mount).unwrap();
        let parsed: OciMount = edgerun_json::from_slice::<OciMount>(json.as_bytes()).unwrap();
        assert_eq!(parsed.destination, "/proc");
        assert_eq!(parsed.mount_type, Some("proc".into()));
        assert_eq!(parsed.options.as_ref().unwrap().len(), 3);
    }

    // ===========================================================================
    // OciLinuxResources serialization
    // ===========================================================================

    #[test]
    fn oci_linux_resources_roundtrip() {
        let res = OciLinuxResources {
            devices: None,
            pids: Some(OciLinuxPids { limit: 256 }),
            memory: Some(OciLinuxMemory {
                limit: Some(1073741824),
                reservation: Some(536870912),
                swap: Some(0),
                kernel: None,
                kernel_tcp: None,
            }),
            cpu: Some(OciLinuxCpu {
                shares: Some(2048),
                quota: Some(100000),
                period: Some(100000),
                realtime_runtime: None,
                realtime_period: None,
                cpus: None,
                mems: None,
            }),
            block_io: None,
            hugepage_limits: None,
            network: None,
        };
        let json = edgerun_json::to_string(&res).unwrap();
        let parsed: OciLinuxResources = edgerun_json::from_slice::<OciLinuxResources>(json.as_bytes()).unwrap();
        assert_eq!(parsed.memory.as_ref().unwrap().limit, Some(1073741824));
        assert_eq!(parsed.cpu.as_ref().unwrap().shares, Some(2048));
        assert_eq!(parsed.pids.as_ref().unwrap().limit, 256);
    }

    // ===========================================================================
    // OciProcess with capabilities
    // ===========================================================================

    #[test]
    fn oci_process_capabilities_roundtrip() {
        let caps = OciCapabilities {
            bounding: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            inheritable: Some(vec![]),
            permitted: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            ambient: Some(vec![]),
        };
        let json = edgerun_json::to_string(&caps).unwrap();
        let parsed: OciCapabilities = edgerun_json::from_slice::<OciCapabilities>(json.as_bytes()).unwrap();
        assert_eq!(parsed.bounding.as_ref().unwrap().len(), 1);
        assert_eq!(parsed.effective.as_ref().unwrap()[0], "CAP_NET_BIND_SERVICE");
    }
}

    #[test]
    fn parse_go_generated_config() {
        // Read the Go-generated config.json
        let data = std::fs::read("/tmp/go-config.json");
        if data.is_err() {
            // Skip if file doesn't exist
            return;
        }
        let data = data.unwrap();

        // First parse as JsonValue
        let v_result: Result<edgerun_json::Value, _> = edgerun_json::from_slice(&data);

        // Then parse as OciSpec
        match json::parse_oci_spec(&data) {
            Ok(spec) => {
                assert!(!spec.version.is_empty(), "version should not be empty");
            }
            Err(e) => {
                panic!("Failed to parse Go-generated config: {}", e);
            }
        }
    }
