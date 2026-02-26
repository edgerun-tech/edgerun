// SPDX-License-Identifier: Apache-2.0

/// Render an additive containerd config snippet that keeps OCI on `crun`
/// while exposing an explicit EdgeRun runtime class for WASI workloads.
pub fn render_containerd_snippet(runtime_binary: &str, snapshotter: &str) -> String {
    format!(
        "[plugins.'io.containerd.cri.v1.runtime'.containerd]\n  default_runtime_name = 'crun'\n  snapshotter = '{snapshotter}'\n\n[plugins.'io.containerd.cri.v1.runtime'.containerd.runtimes.crun]\n  runtime_type = 'io.containerd.runc.v2'\n  privileged_without_host_devices = false\n  [plugins.'io.containerd.cri.v1.runtime'.containerd.runtimes.crun.options]\n    BinaryName = '/usr/bin/crun'\n    SystemdCgroup = true\n\n[plugins.'io.containerd.cri.v1.runtime'.containerd.runtimes.edgerun]\n  runtime_type = 'io.containerd.edgerun.v1'\n  privileged_without_host_devices = false\n  [plugins.'io.containerd.cri.v1.runtime'.containerd.runtimes.edgerun.options]\n    BinaryName = '{runtime_binary}'\n    SystemdCgroup = true\n"
    )
}

#[cfg(test)]
mod tests {
    use super::render_containerd_snippet;

    #[test]
    fn snippet_contains_runtime_and_snapshotter() {
        let rendered =
            render_containerd_snippet("/usr/bin/containerd-shim-edgerun-v2", "overlayfs");
        assert!(rendered.contains("default_runtime_name = 'crun'"));
        assert!(
            rendered.contains("[plugins.'io.containerd.cri.v1.runtime'.containerd.runtimes.crun]")
        );
        assert!(rendered.contains("runtime_type = 'io.containerd.runc.v2'"));
        assert!(rendered.contains("BinaryName = '/usr/bin/crun'"));
        assert!(rendered.contains("runtime_type = 'io.containerd.edgerun.v1'"));
        assert!(rendered.contains("BinaryName = '/usr/bin/containerd-shim-edgerun-v2'"));
        assert!(rendered.contains("snapshotter = 'overlayfs'"));
        assert!(!rendered.contains("[proxy_plugins.edgerun]"));
    }
}
