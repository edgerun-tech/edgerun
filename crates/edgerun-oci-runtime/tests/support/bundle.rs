//! OCI bundle creation for conformance tests.

use edgerun_oci_runtime::{self, json::*};

use std::fs;
use std::io;
use std::path::Path;

/// A builder for OCI test bundles.
#[derive(Debug)]
pub struct Bundle {
    pub spec: OciSpec,
}

/// Create a minimal bundle for conformance testing.
/// Uses /bin/sleep 5 as the default workload so the container stays running long
/// enough to test state transitions, kill, pause, etc.
pub fn minimal() -> Bundle {
    let mut spec = edgerun_oci_runtime::create_bundle(
        "rootfs",
        vec!["/bin/sleep".into(), "5".into()],
        None,
        None,
    );
    // Minimal namespaces: just mount (pid namespace adds complexity with PID 1 init)
    spec.linux = Some(OciLinux {
        namespaces: Some(vec![OciNamespace {
            ns_type: "mount".into(),
            path: None,
        }]),
        masked_paths: Some(vec![]),
        readonly_paths: Some(vec![]),
        ..Default::default()
    });

    Bundle { spec }
}

impl Bundle {
    /// Set the container hostname.
    pub fn hostname(mut self, name: &str) -> Self {
        self.spec.hostname = Some(name.into());
        self
    }

    /// Set the process arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = &'static str>) -> Self {
        if let Some(ref mut proc) = self.spec.process {
            proc.args = Some(args.into_iter().map(|s| s.to_string()).collect());
        }
        self
    }

    /// Add a mount to the bundle.
    pub fn mount(mut self, mount: OciMount) -> Self {
        let mounts = self.spec.mounts.get_or_insert_with(Vec::new);
        mounts.push(mount);
        self
    }

    /// Set the cgroup path for resource controls.
    pub fn cgroup_path(mut self, path: &str) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        linux.cgroups_path = Some(path.into());
        self
    }

    /// Set memory limit in bytes.
    pub fn memory_limit(mut self, bytes: i64) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        let resources = linux.resources.get_or_insert_with(Default::default);
        let memory = resources.memory.get_or_insert_with(Default::default);
        memory.limit = Some(bytes);
        self
    }

    /// Set CPU shares.
    pub fn cpu_shares(mut self, shares: u64) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        let resources = linux.resources.get_or_insert_with(Default::default);
        let cpu = resources.cpu.get_or_insert_with(Default::default);
        cpu.shares = Some(shares);
        self
    }

    /// Set PIDs limit.
    pub fn pids_limit(mut self, limit: i64) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        let resources = linux.resources.get_or_insert_with(Default::default);
        resources.pids = Some(OciLinuxPids { limit });
        self
    }

    /// Set masked paths.
    pub fn masked_paths(mut self, paths: impl IntoIterator<Item = &'static str>) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        linux.masked_paths = Some(paths.into_iter().map(|s| s.to_string()).collect());
        self
    }

    /// Set readonly paths.
    pub fn readonly_paths(mut self, paths: impl IntoIterator<Item = &'static str>) -> Self {
        let linux = self.spec.linux.get_or_insert_with(Default::default);
        linux.readonly_paths = Some(paths.into_iter().map(|s| s.to_string()).collect());
        self
    }

    /// Write the bundle to the given directory.
    pub fn write_to(&self, dir: &Path) -> io::Result<()> {
        let rootfs = dir.join("rootfs");
        copy_runtime_libs(&rootfs)?;

        // Create required empty directories
        for d in &["proc", "sys", "tmp", "etc", "dev"] {
            let _ = fs::create_dir_all(rootfs.join(d));
        }

        fs::write(dir.join("config.json"), self.spec.to_json_string())
    }
}

/// Copy host binaries and their dynamic libraries into the rootfs.
///
/// On this system:
/// - `/bin/true` and `/bin/sleep` are dynamically linked via `/lib64/ld-linux-x86-64.so.2`
/// - `libc` is at `/usr/lib/libc.so.6`
/// - The actual linker binary is at `/usr/lib64/ld-linux-x86-64.so.2` (symlinked from /lib64)
pub fn copy_runtime_libs(rootfs: &Path) -> io::Result<()> {
    // Known file copies: (host_path, rootfs_relative_path)
    let copies: &[(&str, &str)] = &[
        ("/bin/true", "bin/true"),
        ("/bin/sleep", "bin/sleep"),
        ("/bin/false", "bin/false"),
        ("/lib64/ld-linux-x86-64.so.2", "lib64/ld-linux-x86-64.so.2"),
        ("/usr/lib/libc.so.6", "usr/lib/libc.so.6"),
    ];

    for (src, dst_rel) in copies {
        if Path::new(src).exists() {
            let dst = rootfs.join(dst_rel);
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, &dst);
        }
    }

    // Alternative libc locations
    for (src, dst_rel) in &[
        ("/lib/x86_64-linux-gnu/libc.so.6", "lib/x86_64-linux-gnu/libc.so.6"),
        (
            "/usr/lib/x86_64-linux-gnu/libc.so.6",
            "usr/lib/x86_64-linux-gnu/libc.so.6",
        ),
    ] {
        if Path::new(src).exists() && !rootfs.join(dst_rel).exists() {
            let dst = rootfs.join(dst_rel);
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, &dst);
        }
    }

    Ok(())
}
