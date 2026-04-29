//! edgerun-json bindings for OCI spec model types.

use crate::prelude::*;
use crate::spec::*;
use edgerun_json::{FromJson, JsonValue, JsonValueError, ToJson};

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

edgerun_json::impl_json_struct! {
    OciSpec {
        required { version: "ociVersion" => String }
        optional {
            platform: "platform" => OciPlatform,
            process: "process" => OciProcess,
            root: "root" => OciRoot,
            hostname: "hostname" => String,
            domainname: "domainname" => String,
            linux: "linux" => OciLinux,
            mounts: "mounts" => Vec<OciMount>,
            annotations: "annotations" => BTreeMap<String, String>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciPlatform {
        required {}
        optional {
            os: "os" => String,
            arch: "arch" => String,
            os_version: "os.version" => String,
            os_features: "os.features" => Vec<String>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciProcess {
        required {}
        optional {
            terminal: "terminal" => bool,
            user: "user" => OciUser,
            console_size: "consoleSize" => OciBox,
            args: "args" => Vec<String>,
            env: "env" => Vec<String>,
            cwd: "cwd" => String,
            capabilities: "capabilities" => OciCapabilities,
            rlimits: "rlimits" => Vec<OciRlimit>,
            no_new_privileges: "noNewPrivileges" => bool,
            oom_score_adj: "oomScoreAdj" => i64,
            apparmor_profile: "apparmorProfile" => String,
            selinux_label: "selinuxLabel" => String,
            scheduler: "scheduler" => OciScheduler,
            io_priority: "ioPriority" => OciIoPriority,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciBox {
        required {
            width: "width" => u64,
            height: "height" => u64,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciUser {
        required {}
        optional {
            uid: "uid" => u32,
            gid: "gid" => u32,
            additional_gids: "additionalGids" => Vec<u32>,
            umask: "umask" => u32,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciCapabilities {
        required {}
        optional {
            bounding: "bounding" => Vec<String>,
            effective: "effective" => Vec<String>,
            inheritable: "inheritable" => Vec<String>,
            permitted: "permitted" => Vec<String>,
            ambient: "ambient" => Vec<String>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciRlimit {
        required {
            ns_type: "type" => String,
            hard: "hard" => u64,
            soft: "soft" => u64,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciRoot {
        required { path: "path" => String }
        optional { readonly: "readonly" => bool }
    }
}

edgerun_json::impl_json_struct! {
    OciScheduler {
        required { policy: "policy" => String }
        optional {
            nice: "nice" => i32,
            priority: "priority" => i32,
            deadline: "deadline" => OciSchedDeadline,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciSchedDeadline {
        required {}
        optional {
            runtime_ns: "runtime" => u64,
            period_ns: "period" => u64,
            deadline_ns: "deadline" => u64,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciIoPriority {
        required { class: "class" => u32 }
        optional { priority: "priority" => u32 }
    }
}

edgerun_json::impl_json_struct! {
    OciLinux {
        required {}
        optional {
            uid_mappings: "uidMappings" => Vec<OciIdMapping>,
            gid_mappings: "gidMappings" => Vec<OciIdMapping>,
            resources: "resources" => OciLinuxResources,
            cgroups_path: "cgroupsPath" => String,
            namespaces: "namespaces" => Vec<OciNamespace>,
            devices: "devices" => Vec<OciLinuxDevice>,
            masked_paths: "maskedPaths" => Vec<String>,
            readonly_paths: "readonlyPaths" => Vec<String>,
            mount_label: "mountLabel" => String,
            rootfs_propagation: "rootfsPropagation" => String,
            sysctl: "sysctl" => BTreeMap<String, String>,
            hooks: "hooks" => OciHooks,
            seccomp: "seccomp" => OciLinuxSeccomp,
            intel_rdt: "intelRdt" => OciLinuxIntelRdt,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciHooks {
        required {}
        optional {
            prestart: "prestart" => Vec<OciHook>,
            create_runtime: "createRuntime" => Vec<OciHook>,
            create_container: "createContainer" => Vec<OciHook>,
            start_container: "startContainer" => Vec<OciHook>,
            poststart: "poststart" => Vec<OciHook>,
            poststop: "poststop" => Vec<OciHook>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciHook {
        required { path: "path" => String }
        optional {
            args: "args" => Vec<String>,
            env: "env" => Vec<String>,
            timeout: "timeout" => u64,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciIdMapping {
        required {
            container_id: "containerID" => u32,
            host_id: "hostID" => u32,
            size: "size" => u32,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciNamespace {
        required { ns_type: "type" => String }
        optional { path: "path" => String }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxDevice {
        required {
            ns_type: "type" => String,
            path: "path" => String,
        }
        optional {
            file_mode: "fileMode" => u32,
            uid: "uid" => u32,
            gid: "gid" => u32,
            major: "major" => i64,
            minor: "minor" => i64,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxCpu {
        required {}
        optional {
            shares: "shares" => u64,
            quota: "quota" => i64,
            period: "period" => u64,
            realtime_runtime: "realtimeRuntime" => i64,
            realtime_period: "realtimePeriod" => u64,
            cpus: "cpus" => String,
            mems: "mems" => String,
            idle: "idle" => i64,
            burst: "burst" => i64,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxBlockIO {
        required {}
        optional {
            weight: "weight" => u16,
            leaf_weight: "leafWeight" => u16,
            weight_device: "weightDevice" => Vec<OciLinuxWeightDevice>,
            leaf_weight_device: "leafWeightDevice" => Vec<OciLinuxWeightDevice>,
            throttle_read_bps_device: "throttleReadBpsDevice" => Vec<OciLinuxThrottleDevice>,
            throttle_write_bps_device: "throttleWriteBpsDevice" => Vec<OciLinuxThrottleDevice>,
            throttle_read_iops_device: "throttleReadIOPSDevice" => Vec<OciLinuxThrottleDevice>,
            throttle_write_iops_device: "throttleWriteIOPSDevice" => Vec<OciLinuxThrottleDevice>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxWeightDevice {
        required {
            major: "major" => i64,
            minor: "minor" => i64,
        }
        optional {
            weight: "weight" => u16,
            leaf_weight: "leafWeight" => u16,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxThrottleDevice {
        required {
            major: "major" => i64,
            minor: "minor" => i64,
            rate: "rate" => u64,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxHugepageLimit {
        required {
            pagesize: "pagesize" => String,
            limit: "limit" => u64,
        }
        optional { rsvd: "rsvd" => bool }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxNetwork {
        required {}
        optional {
            class_id: "classID" => u32,
            priorities: "priorities" => Vec<OciLinuxNetworkPriority>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxNetworkPriority {
        required {
            name: "name" => String,
            priority: "priority" => u32,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxIntelRdt {
        required {}
        optional {
            l3_cache_schema: "l3CacheSchema" => String,
            mem_bw_schema: "memBwSchema" => String,
            clos_id: "closID" => String,
            enable_monitoring: "enableMonitoring" => bool,
            schemata: "schemata" => String,
        }
    }
}

impl FromJson for OciSeccompAction {
    fn from_json(value: JsonValue) -> Result<Self, JsonValueError> {
        let action = String::from_json(value)?;
        Self::try_from(action).map_err(JsonValueError::WrongType)
    }
}

impl ToJson for OciSeccompAction {
    fn to_json(&self) -> JsonValue {
        let value: String = self.clone().into();
        value.to_json()
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxSeccomp {
        required {}
        optional {
            default_action: "defaultAction" => OciSeccompAction,
            default_errno_ret: "defaultErrnoRet" => u32,
            architectures: "architectures" => Vec<String>,
            listener_path: "listenerPath" => String,
            listener_metadata: "listenerMetadata" => String,
            syscalls: "syscalls" => Vec<OciSeccompSyscallEntry>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciSeccompSyscallEntry {
        required {}
        optional {
            names: "names" => Vec<String>,
            action: "action" => OciSeccompAction,
            errno_ret: "errnoRet" => u32,
            args: "args" => Vec<OciSeccompArg>,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciSeccompArg {
        required {
            index: "index" => u32,
            value: "value" => u64,
            value_two: "valueTwo" => u64,
            op: "op" => String,
        }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxResources {
        required {}
        optional {
            devices: "devices" => Vec<OciLinuxDeviceCgroup>,
            memory: "memory" => OciLinuxMemory,
            cpu: "cpu" => OciLinuxCpu,
            pids: "pids" => OciLinuxPids,
            block_io: "blockIO" => OciLinuxBlockIO,
            hugepage_limits: ["hugepageLimits", "hugepage_limits"] => Vec<OciLinuxHugepageLimit>,
            network: "network" => OciLinuxNetwork,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxDeviceCgroup {
        required {}
        optional {
            allow: "allow" => bool,
            ns_type: "type" => String,
            major: "major" => i64,
            minor: "minor" => i64,
            access: "access" => String,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxPids {
        required { limit: "limit" => i64 }
        optional {}
    }
}

edgerun_json::impl_json_struct! {
    OciLinuxMemory {
        required {}
        optional {
            limit: "limit" => i64,
            reservation: "reservation" => i64,
            swap: "swap" => i64,
            kernel: "kernel" => i64,
            kernel_tcp: "kernelTCP" => i64,
            check_before_update: "checkBeforeUpdate" => bool,
        }
    }
}

edgerun_json::impl_json_struct! {
    OciMount {
        required { destination: "destination" => String }
        optional {
            mount_type: "type" => String,
            source: "source" => String,
            options: "options" => Vec<String>,
            label: "label" => String,
            recursive: "recursive" => bool,
            uid_mappings: "uidMappings" => Vec<OciIdMapping>,
            gid_mappings: "gidMappings" => Vec<OciIdMapping>,
        }
    }
}
