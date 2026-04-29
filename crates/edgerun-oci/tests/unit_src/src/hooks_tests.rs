use super::*;

#[test]
fn container_state_to_json_is_valid() {
    let mut annotations = BTreeMap::new();
    annotations.insert("key".into(), "value".into());

    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test-container".into(),
        status: "created".into(),
        pid: 12345,
        bundle: "/var/lib/bundles/test".into(),
        annotations,
    };

    let json = state.to_json();
    let value = edgerun_json::parse_json(&json).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.get_str("ociVersion"), Some("1.0.2"));
    assert_eq!(object.get_str("id"), Some("test-container"));
    assert_eq!(object.get_str("status"), Some("created"));
    assert_eq!(object.get_u32("pid"), Some(12345));
    assert_eq!(object.get_str("bundle"), Some("/var/lib/bundles/test"));
    assert_eq!(
        object.get_object("annotations").unwrap().get_str("key"),
        Some("value")
    );
}

#[test]
fn container_state_empty_annotations() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "running".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };
    let json = state.to_json();
    let value = edgerun_json::parse_json(&json).unwrap();
    assert!(value
        .as_object()
        .unwrap()
        .get_object("annotations")
        .unwrap()
        .is_empty());
}

#[test]
fn container_state_json_escapes_strings() {
    let mut annotations = BTreeMap::new();
    annotations.insert("quoted\"key".into(), "line\nvalue".into());

    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test\"container".into(),
        status: "created".into(),
        pid: 12345,
        bundle: "/var/lib/bundles/test".into(),
        annotations,
    };

    let value = edgerun_json::parse_json(&state.to_json()).unwrap();
    let object = value.as_object().unwrap();
    assert_eq!(object.get_str("id"), Some("test\"container"));
    assert_eq!(
        object
            .get_object("annotations")
            .unwrap()
            .get_str("quoted\"key"),
        Some("line\nvalue")
    );
}

#[test]
fn execute_hooks_empty_is_ok() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "created".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };
    assert!(execute_hooks(None, &state).is_ok());
    assert!(execute_hooks(Some(&[]), &state).is_ok());
}

#[test]
fn execute_hooks_failing_hook_returns_error() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "created".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };

    // Hook with non-existent path should fail
    let hooks = vec![crate::spec::OciHook {
        path: "/usr/bin/nonexistent-hook".into(),
        args: None,
        env: None,
        timeout: None,
    }];
    let result = execute_hooks(Some(&hooks), &state);
    assert!(result.is_err());
}

#[test]
fn execute_hooks_successful_hook() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "created".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };

    // Use /bin/true which always exits 0
    let hooks = vec![crate::spec::OciHook {
        path: "/bin/true".into(),
        args: None,
        env: None,
        timeout: None,
    }];
    let result = execute_hooks(Some(&hooks), &state);
    assert!(result.is_ok());
}

#[test]
fn execute_hooks_hook_with_nonzero_exit_fails() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "created".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };

    // /bin/false always exits 1
    let hooks = vec![crate::spec::OciHook {
        path: "/bin/false".into(),
        args: None,
        env: None,
        timeout: None,
    }];
    let result = execute_hooks(Some(&hooks), &state);
    assert!(result.is_err());
}

#[test]
fn execute_poststop_hooks_never_fails() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "stopped".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };

    // Even with failing hooks, poststop should not return an error
    let hooks = vec![
        crate::spec::OciHook {
            path: "/bin/false".into(),
            args: None,
            env: None,
            timeout: None,
        },
        crate::spec::OciHook {
            path: "/bin/true".into(),
            args: None,
            env: None,
            timeout: None,
        },
    ];
    // Should not panic — poststop logs warnings but continues
    execute_poststop_hooks(Some(&hooks), &state);
}

#[test]
fn execute_hooks_stops_on_first_failure() {
    let state = ContainerState {
        version: "1.0.2".into(),
        id: "test".into(),
        status: "created".into(),
        pid: 1,
        bundle: "/rootfs".into(),
        annotations: BTreeMap::new(),
    };

    // false should fail, true should never run
    let hooks = vec![
        crate::spec::OciHook {
            path: "/bin/false".into(),
            args: None,
            env: None,
            timeout: None,
        },
        crate::spec::OciHook {
            path: "/bin/true".into(),
            args: None,
            env: None,
            timeout: None,
        },
    ];
    let result = execute_hooks(Some(&hooks), &state);
    assert!(result.is_err());
    // The error should be about /bin/false, not /bin/true
    assert!(result.unwrap_err().hook_path.contains("false"));
}
