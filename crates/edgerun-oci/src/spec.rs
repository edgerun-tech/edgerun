//! OCI runtime spec compatibility re-exports and host helpers.

pub use edgerun_protocols::oci::runtime_spec::*;

pub fn platform_matches_host(platform: &OciPlatform) -> bool {
    if let Some(ref os) = platform.os {
        if os != crate::validate::host_os() {
            return false;
        }
    }
    if let Some(ref arch) = platform.arch {
        if arch != crate::validate::host_arch() {
            return false;
        }
    }
    true
}

pub fn spec_to_json_string(spec: &OciSpec) -> alloc::string::String {
    spec.to_json_string()
}

pub fn spec_to_json_string_pretty(spec: &OciSpec) -> alloc::string::String {
    spec.to_json_string_pretty()
}
