// SPDX-License-Identifier: Apache-2.0

pub const SCOPE_OPENID: &str = "openid";
pub const SCOPE_PROFILE: &str = "profile";
pub const SCOPE_EMAIL: &str = "email";

pub const SCOPE_PROFILE_READ: &str = "edgerun:profile.read";
pub const SCOPE_PROFILE_WRITE: &str = "edgerun:profile.write";
pub const SCOPE_NODES_READ: &str = "edgerun:nodes.read";
pub const SCOPE_NODES_BIND: &str = "edgerun:nodes.bind";
pub const SCOPE_INTENTS_SUBMIT: &str = "edgerun:intents.submit";
pub const SCOPE_INTENTS_APPROVE: &str = "edgerun:intents.approve";

pub const SCOPE_CAP_STORAGE_READ: &str = "edgerun:cap.storage.read";
pub const SCOPE_CAP_STORAGE_WRITE: &str = "edgerun:cap.storage.write";
pub const SCOPE_CAP_NETWORK_USE: &str = "edgerun:cap.network.use";
pub const SCOPE_CAP_USB_USE: &str = "edgerun:cap.usb.use";
pub const SCOPE_CAP_CAMERA_USE: &str = "edgerun:cap.camera.use";
pub const SCOPE_CAP_MICROPHONE_USE: &str = "edgerun:cap.microphone.use";
pub const SCOPE_CAP_GPU_USE: &str = "edgerun:cap.gpu.use";

pub const SCOPE_EXEC_WASM: &str = "edgerun:exec.wasm";
pub const SCOPE_EXEC_OCI: &str = "edgerun:exec.oci";
pub const SCOPE_EXEC_VM: &str = "edgerun:exec.vm";
pub const SCOPE_EXEC_LXC: &str = "edgerun:exec.lxc";

pub const SCOPE_POLICY_WRITE: &str = "edgerun:policy.write";
pub const SCOPE_CLUSTER_ADMIN: &str = "edgerun:cluster.admin";
pub const SCOPE_NODE_RECOVER: &str = "edgerun:node.recover";

pub const CANONICAL_SCOPES: &[&str] = &[
    SCOPE_OPENID,
    SCOPE_PROFILE,
    SCOPE_EMAIL,
    SCOPE_PROFILE_READ,
    SCOPE_PROFILE_WRITE,
    SCOPE_NODES_READ,
    SCOPE_NODES_BIND,
    SCOPE_INTENTS_SUBMIT,
    SCOPE_INTENTS_APPROVE,
    SCOPE_CAP_STORAGE_READ,
    SCOPE_CAP_STORAGE_WRITE,
    SCOPE_CAP_NETWORK_USE,
    SCOPE_CAP_USB_USE,
    SCOPE_CAP_CAMERA_USE,
    SCOPE_CAP_MICROPHONE_USE,
    SCOPE_CAP_GPU_USE,
    SCOPE_EXEC_WASM,
    SCOPE_EXEC_OCI,
    SCOPE_EXEC_VM,
    SCOPE_EXEC_LXC,
    SCOPE_POLICY_WRITE,
    SCOPE_CLUSTER_ADMIN,
    SCOPE_NODE_RECOVER,
];

pub const DEFAULT_LOCAL_PROFILE_SCOPES: &[&str] = &[
    SCOPE_OPENID,
    SCOPE_PROFILE,
    SCOPE_PROFILE_READ,
    SCOPE_PROFILE_WRITE,
    SCOPE_NODES_READ,
    SCOPE_NODES_BIND,
    SCOPE_INTENTS_SUBMIT,
    SCOPE_INTENTS_APPROVE,
    SCOPE_CAP_STORAGE_READ,
    SCOPE_CAP_STORAGE_WRITE,
    SCOPE_CAP_NETWORK_USE,
    SCOPE_CAP_USB_USE,
    SCOPE_CAP_CAMERA_USE,
    SCOPE_CAP_MICROPHONE_USE,
    SCOPE_CAP_GPU_USE,
    SCOPE_EXEC_WASM,
    SCOPE_EXEC_OCI,
    SCOPE_EXEC_VM,
    SCOPE_EXEC_LXC,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_scopes_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for scope in CANONICAL_SCOPES {
            assert!(seen.insert(*scope), "duplicate scope: {scope}");
        }
    }

    #[test]
    fn default_local_profile_scopes_exclude_admin() {
        assert!(!DEFAULT_LOCAL_PROFILE_SCOPES.contains(&SCOPE_POLICY_WRITE));
        assert!(!DEFAULT_LOCAL_PROFILE_SCOPES.contains(&SCOPE_CLUSTER_ADMIN));
        assert!(!DEFAULT_LOCAL_PROFILE_SCOPES.contains(&SCOPE_NODE_RECOVER));
    }
}
