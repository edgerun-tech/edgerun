use super::*;

#[test]
fn parse_minimal_oci_spec_with_edgerun_json() {
    let json = r#"{"ociVersion":"1.0.2","root":{"path":"rootfs"},"process":{"args":["/bin/sh"]}}"#;

    let spec = parse_oci_spec(json.as_bytes()).unwrap();

    assert_eq!(spec.version, "1.0.2");
    assert_eq!(spec.root.unwrap().path, "rootfs");
    assert_eq!(spec.process.unwrap().args.unwrap(), vec!["/bin/sh"]);
}

#[test]
fn parse_runtime_fields_with_edgerun_json() {
    let json = r#"{
            "ociVersion": "1.0.2",
            "hostname": "edgerun",
            "process": {
                "terminal": true,
                "args": ["/init", "--boot"],
                "env": ["PATH=/bin", "TERM=xterm"],
                "cwd": "/",
                "user": { "uid": 1000, "gid": 1000, "additionalGids": [10, 11] },
                "capabilities": { "bounding": ["CAP_CHOWN"], "effective": ["CAP_CHOWN"] },
                "rlimits": [{ "type": "RLIMIT_NOFILE", "hard": 1024, "soft": 512 }],
                "noNewPrivileges": true
            },
            "root": { "path": "/rootfs", "readonly": true },
            "mounts": [{ "destination": "/proc", "type": "proc", "source": "proc", "options": ["nosuid"] }],
            "linux": {
                "namespaces": [{ "type": "mount" }, { "type": "pid" }],
                "uidMappings": [{ "containerID": 0, "hostID": 100000, "size": 65536 }],
                "maskedPaths": ["/proc/kcore"],
                "readonlyPaths": ["/proc/sys"],
                "sysctl": { "net.ipv4.ip_forward": "1" },
                "resources": {
                    "memory": { "limit": 1048576 },
                    "cpu": { "shares": 1024, "cpus": "0" },
                    "pids": { "limit": 64 },
                    "devices": [{ "type": "c", "major": 1, "minor": 3, "access": "rwm" }]
                }
            }
        }"#;

    let spec = parse_oci_spec(json.as_bytes()).unwrap();
    let process = spec.process.unwrap();
    assert_eq!(process.terminal, Some(true));
    assert_eq!(process.user.unwrap().uid, Some(1000));
    assert_eq!(process.rlimits.unwrap()[0].soft, 512);

    let linux = spec.linux.unwrap();
    assert_eq!(linux.namespaces.unwrap()[1].ns_type, "pid");
    assert_eq!(linux.uid_mappings.unwrap()[0].host_id, 100000);
    assert_eq!(
        linux.sysctl.unwrap().get("net.ipv4.ip_forward"),
        Some(&"1".into())
    );
    let resources = linux.resources.unwrap();
    assert_eq!(resources.memory.unwrap().limit, Some(1048576));
    assert_eq!(resources.cpu.unwrap().cpus, Some("0".into()));
    assert_eq!(resources.pids.unwrap().limit, 64);
    assert_eq!(resources.devices.unwrap()[0].access, Some("rwm".into()));

    let mounts = spec.mounts.unwrap();
    assert_eq!(mounts[0].destination, "/proc");
    assert_eq!(mounts[0].options.as_ref().unwrap()[0], "nosuid");
}

#[test]
fn parse_oci_spec_with_edgerun_json_rejects_missing_version() {
    assert!(parse_oci_spec(br#"{"root":{"path":"rootfs"}}"#).is_err());
}
