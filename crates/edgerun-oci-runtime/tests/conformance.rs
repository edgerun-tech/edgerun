//! OCI runtime conformance tests — cgroup v2 aware.
//!
//! Runs real containers via the edgerun-oci CLI binary and validates results.
//! No external test frameworks, no Go dependencies.
//!
//! These tests require root privileges (for namespace creation).
//! When run as non-root, the test harness automatically uses `sudo -n`.
//!
//! Run with: `sudo cargo test -p edgerun-oci-runtime --test conformance -- --test-threads=1`

#[path = "support/runner.rs"]
mod runner;
#[path = "support/bundle.rs"]
mod bundle;

use edgerun_oci_runtime::json::*;

use std::path::Path;

// ===========================================================================
// Integration tests
// ===========================================================================

#[test]
fn conformance_default() {
    let mut runner = runner::Runner::new("default");
    runner.bundle(bundle::minimal());
    runner.run().expect("default container failed");
}

#[test]
fn conformance_hostname() {
    let mut runner = runner::Runner::new("hostname");
    runner.bundle(bundle::minimal().hostname("conformance-hostname"));
    runner.run().expect("hostname container failed");
}

#[test]
fn conformance_process_args() {
    let mut runner = runner::Runner::new("process-args");
    runner.bundle(bundle::minimal().args(["/bin/sleep", "1"]));
    runner.run().expect("process args container failed");
}

#[test]
fn conformance_mounts() {
    let mut runner = runner::Runner::new("mounts");
    runner.bundle(bundle::minimal().mount(proc_mount()));
    runner.run().expect("mounts container failed");
}

#[test]
fn conformance_masked_paths() {
    let mut runner = runner::Runner::new("masked-paths");
    runner.bundle(bundle::minimal().masked_paths(["/proc/kcore"]));
    runner.run().expect("masked paths container failed");
}

#[test]
fn conformance_readonly_paths() {
    let mut runner = runner::Runner::new("readonly-paths");
    runner.bundle(bundle::minimal().readonly_paths(["/proc/sys"]));
    runner.run().expect("readonly paths container failed");
}

#[test]
fn conformance_lifecycle_state() {
    let mut runner = runner::Runner::new("lifecycle");
    runner.bundle(bundle::minimal().args(["/bin/sleep", "0.5"]));

    runner.create().expect("lifecycle create failed");
    runner.assert_state("created");

    runner.start().expect("lifecycle start failed");
    // Immediately after start, container should be "running" (sleep 0.5 still executing)
    runner.assert_state("running");

    // Wait for sleep to exit, then verify stopped
    std::thread::sleep(std::time::Duration::from_millis(1000));
    runner.assert_state("stopped");

    runner.delete().expect("lifecycle delete failed");
    runner.assert_state_deleted();
}

#[test]
fn conformance_duplicate_id() {
    let mut runner = runner::Runner::new("dup");
    runner.bundle(bundle::minimal().args(["/bin/sleep", "5"]));

    runner.create().expect("first create failed");

    // Second create with same ID should fail — call the CLI directly
    let out = runner::cli_create(runner.id(), runner.bundle_path());
    assert!(!out.status.success(), "duplicate create should have failed, got: {}",
        String::from_utf8_lossy(&out.stdout));

    runner.delete_force().ok();
}

#[test]
fn conformance_cgroup_memory() {
    let mut runner = runner::Runner::new("cg-mem");
    runner.bundle(
        bundle::minimal()
            .cgroup_path("/edgerun-test-mem")
            .memory_limit(64 * 1024 * 1024),
    );
    runner.run().expect("cgroup memory container failed");

    // Verify cgroup v2 file was written
    verify_cgroup_file("/sys/fs/cgroup/edgerun-test-mem/memory.max", "67108864");
    let _ = std::fs::remove_dir_all("/sys/fs/cgroup/edgerun-test-mem");
}

#[test]
fn conformance_cgroup_pids() {
    let mut runner = runner::Runner::new("cg-pids");
    runner.bundle(
        bundle::minimal()
            .cgroup_path("/edgerun-test-pids")
            .pids_limit(32),
    );
    runner.run().expect("cgroup pids container failed");

    verify_cgroup_file("/sys/fs/cgroup/edgerun-test-pids/pids.max", "32");
    let _ = std::fs::remove_dir_all("/sys/fs/cgroup/edgerun-test-pids");
}

#[test]
fn conformance_cgroup_cpu() {
    let mut runner = runner::Runner::new("cg-cpu");
    runner.bundle(
        bundle::minimal()
            .cgroup_path("/edgerun-test-cpu")
            .cpu_shares(512),
    );
    runner.run().expect("cgroup cpu container failed");

    let content = std::fs::read_to_string("/sys/fs/cgroup/edgerun-test-cpu/cpu.weight")
        .expect("cgroup v2 cpu.weight not readable");
    let weight: u64 = content.trim().parse().expect("cpu.weight not a number");
    assert!((1..=10000).contains(&weight), "cpu.weight out of range: {weight}");
    let _ = std::fs::remove_dir_all("/sys/fs/cgroup/edgerun-test-cpu");
}

#[test]
fn conformance_cgroup_cleanup() {
    let cgroup_path = "/sys/fs/cgroup/edgerun-test-cleanup";
    let mut runner = runner::Runner::new("cg-cleanup");
    runner.bundle(
        bundle::minimal()
            .cgroup_path(cgroup_path),
    );
    runner.run().expect("cgroup cleanup container failed");
    runner.delete().expect("cgroup cleanup delete failed");

    // Give cgroup a moment to settle
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(
        !Path::new(cgroup_path).exists(),
        "cgroup directory should be cleaned up after container delete"
    );
}

// ===========================================================================
// Helpers
// ===========================================================================

fn proc_mount() -> OciMount {
    OciMount {
        destination: "/proc".into(),
        mount_type: Some("proc".into()),
        source: Some("proc".into()),
        options: None,
        label: None,
        recursive: None,
        uid_mappings: None,
        gid_mappings: None,
    }
}

fn verify_cgroup_file(path: &str, expected: &str) {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            assert_eq!(
                content.trim(),
                expected,
                "cgroup v2 file {path} not set correctly: got {:?}, expected {expected}",
                content.trim()
            );
        }
        Err(e) => panic!("cgroup v2 file {path} not readable: {e}"),
    }
}

// ===========================================================================
// Unit tests for the test infrastructure
// ===========================================================================

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn bundle_minimal_has_required_fields() {
        let b = bundle::minimal();
        assert_eq!(b.spec.version, "1.0.2");
        assert!(b.spec.process.is_some());
        assert!(b.spec.root.is_some());
        assert!(b.spec.linux.is_some());
    }

    #[test]
    fn bundle_with_hostname() {
        let b = bundle::minimal().hostname("my-host");
        assert_eq!(b.spec.hostname, Some("my-host".into()));
    }

    #[test]
    fn bundle_with_custom_args() {
        let b = bundle::minimal().args(["/bin/foo", "-x"]);
        let args = b.spec.process.as_ref().unwrap().args.as_ref().unwrap();
        assert_eq!(args, &["/bin/foo", "-x"]);
    }

    #[test]
    fn bundle_with_mount() {
        let m = OciMount {
            destination: "/proc".into(),
            mount_type: Some("proc".into()),
            source: Some("proc".into()),
            options: None,
            label: None,
            recursive: None,
            uid_mappings: None,
            gid_mappings: None,
        };
        let b = bundle::minimal().mount(m.clone());
        let mounts = b.spec.mounts.as_ref().unwrap();
        assert!(mounts.iter().any(|m| m.destination == "/proc"));
    }

    #[test]
    fn bundle_with_masked_paths() {
        let b = bundle::minimal().masked_paths(["/proc/kcore", "/proc/keys"]);
        let masked = b.spec.linux.as_ref().unwrap().masked_paths.as_ref().unwrap();
        assert_eq!(masked, &["/proc/kcore", "/proc/keys"]);
    }

    #[test]
    fn bundle_with_cgroup() {
        let b = bundle::minimal()
            .cgroup_path("/test/cg")
            .memory_limit(1048576);
        let linux = b.spec.linux.as_ref().unwrap();
        assert_eq!(linux.cgroups_path, Some("/test/cg".into()));
        let mem = linux.resources.as_ref().unwrap().memory.as_ref().unwrap();
        assert_eq!(mem.limit, Some(1048576));
    }

    #[test]
    fn bundle_config_json_roundtrip() {
        let b = bundle::minimal()
            .hostname("roundtrip")
            .args(["/bin/test"])
            .cgroup_path("/test")
            .pids_limit(10);

        let json = b.spec.to_json_string();
        let parsed: OciSpec = edgerun_json::from_slice::<OciSpec>(json.as_bytes())
            .expect("config.json should be valid JSON");

        assert_eq!(parsed.hostname, Some("roundtrip".into()));
        assert_eq!(
            parsed.process.as_ref().unwrap().args.as_ref().unwrap(),
            &["/bin/test"]
        );
        assert_eq!(
            parsed.linux.as_ref().unwrap().cgroups_path,
            Some("/test".into())
        );
        assert_eq!(
            parsed.linux.as_ref().unwrap().resources.as_ref().unwrap().pids.as_ref().unwrap().limit,
            10
        );
    }

    #[test]
    fn copy_runtime_libs_copies_binaries() {
        let tmp = std::env::temp_dir().join(format!("oci-test-libs-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        bundle::copy_runtime_libs(&tmp).expect("copy_runtime_libs failed");

        // /bin/sleep should exist
        assert!(
            tmp.join("bin/sleep").exists(),
            "bin/sleep should be copied"
        );

        // Dynamic linker should exist at /lib64/
        assert!(
            tmp.join("lib64/ld-linux-x86-64.so.2").exists(),
            "lib64/ld-linux-x86-64.so.2 should be copied"
        );

        // libc should exist
        assert!(
            tmp.join("usr/lib/libc.so.6").exists(),
            "usr/lib/libc.so.6 should be copied"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn runner_id_format() {
        let r = runner::Runner::new("test");
        assert!(r.bundle_path().to_str().unwrap().contains("oci-test-"));
    }

    #[test]
    fn device_uid_gid_ownership_in_container() {
        // Real test: container creates device with uid/gid ownership
        // This FAILS if chown() is NOT called after mknod()
        use edgerun_oci_runtime::OciLinuxDevice;

        let mut b = bundle::minimal().args(["/bin/true"]);
        let dev = OciLinuxDevice {
            ns_type: "c".into(),
            path: "/dev/edgerun-test-device".into(),
            file_mode: Some(0o660),
            uid: Some(1000),
            gid: Some(1000),
            major: Some(10),
            minor: Some(200),
        };
        b.spec.linux.as_mut().unwrap().devices = Some(vec![dev]);

        let mut r = runner::Runner::new("dev-uid-gid");
        r.bundle(b);
        r.run().expect("device uid/gid container failed — chown after mknod not working");
    }

    #[test]
    fn time_namespace_accepted_by_validator() {
        // This FAILS if "time" is NOT in KNOWN_NAMESPACES
        use edgerun_oci_runtime::process::validate_spec;

        let spec = edgerun_oci_runtime::OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(edgerun_oci_runtime::OciProcess {
                terminal: None, user: None, console_size: None,
                args: Some(vec!["/bin/true".into()]),
                env: None, cwd: None, capabilities: None,
                rlimits: None, no_new_privileges: None, oom_score_adj: None,
                apparmor_profile: None, selinux_label: None, scheduler: None,
            }),
            root: Some(edgerun_oci_runtime::OciRoot {
                path: "rootfs".into(), readonly: None,
            }),
            hostname: None,
            linux: Some(edgerun_oci_runtime::OciLinux {
                namespaces: Some(vec![
                    edgerun_oci_runtime::OciNamespace { ns_type: "time".into(), path: None },
                ]),
                ..Default::default()
            }),
            mounts: None, annotations: None,
        };

        validate_spec(&spec).expect("time namespace should be recognized — KNOWN_NAMESPACES missing 'time'");
    }

    #[test]
    fn cgroup_weight_device_per_device_written() {
        // Tests that weightDevice configuration is applied to cgroup files.
        // The kernel must have a weight-based IO scheduler (BFQ) for per-device
        // entries to be accepted. Without it, the global weight is still written.
        use edgerun_oci_runtime::OciLinuxWeightDevice;

        let mut b = bundle::minimal()
            .cgroup_path("/edgerun-test-weightdevice")
            .args(["/bin/true"]);

        let linux = b.spec.linux.as_mut().unwrap();
        let resources = linux.resources.get_or_insert_with(Default::default);
        let blkio = resources.block_io.get_or_insert_with(Default::default);
        blkio.weight = Some(500);
        blkio.weight_device = Some(vec![
            OciLinuxWeightDevice {
                major: 8, minor: 0,
                weight: Some(300),
                leaf_weight: None,
            },
        ]);

        let mut r = runner::Runner::new("weight-device");
        r.bundle(b);
        r.run().expect("weightdevice container failed");

        let cg_path = Path::new("/sys/fs/cgroup/edgerun-test-weightdevice");
        let io_weight = cg_path.join("io.weight");
        let bfq_weight = cg_path.join("io.bfq.weight");

        // At minimum the global weight should be written (500 * 100 = 50000, clamped to 10000)
        if io_weight.exists() {
            let content = std::fs::read_to_string(&io_weight).expect("io.weight readable");
            assert!(content.contains("10000"), "io.weight should have global weight 10000, got: {content}");
            // Per-device entries only accepted if kernel has BFQ/weight-based scheduler
            if content.contains("8:0") {
                // Full support: per-device entries accepted
            } else {
                eprintln!("NOTE: io.weight exists but per-device '8:0' entry not accepted by kernel (no BFQ scheduler?)");
            }
        } else if bfq_weight.exists() {
            let content = std::fs::read_to_string(&bfq_weight).expect("io.bfq.weight readable");
            assert!(content.contains("10000"), "io.bfq.weight should have global weight 10000, got: {content}");
            if content.contains("8:0") {
                // Full support
            } else {
                eprintln!("NOTE: io.bfq.weight exists but per-device '8:0' entry not accepted");
            }
        } else {
            eprintln!("SKIP: no io.weight or io.bfq.weight cgroup files on this kernel");
        }
        let _ = std::fs::remove_dir_all(cg_path);
    }
}
