use super::*;

#[test]
fn is_valid_capability_known() {
    assert!(is_valid_capability("CAP_CHOWN"));
    assert!(is_valid_capability("CAP_NET_BIND_SERVICE"));
    assert!(is_valid_capability("CAP_SYS_ADMIN"));
    assert!(is_valid_capability("CAP_NET_RAW"));
    assert!(is_valid_capability("CAP_KILL"));
}

#[test]
fn is_valid_capability_unknown() {
    assert!(!is_valid_capability("CAP_NONEXISTENT"));
    assert!(!is_valid_capability(""));
    assert!(!is_valid_capability("NOT_A_CAP"));
}

#[test]
fn is_valid_capability_case_sensitive() {
    assert!(!is_valid_capability("cap_chown"));
    assert!(!is_valid_capability("Cap_Chown"));
}

#[test]
fn caps_to_bitmask_single() {
    let caps = vec!["CAP_CHOWN".to_string()];
    let mask = caps_to_bitmask(&caps).unwrap();
    assert!(mask != 0);
}

#[test]
fn caps_to_bitmask_multiple() {
    let caps = vec!["CAP_CHOWN".to_string(), "CAP_KILL".to_string()];
    let mask = caps_to_bitmask(&caps).unwrap();
    assert!(mask != 0);
    // Each should set a different bit
    let chown = caps_to_bitmask(&["CAP_CHOWN".to_string()]).unwrap();
    let kill = caps_to_bitmask(&["CAP_KILL".to_string()]).unwrap();
    assert_eq!(mask, chown | kill);
}

#[test]
fn caps_to_bitmask_empty() {
    let caps: Vec<String> = vec![];
    let mask = caps_to_bitmask(&caps).unwrap();
    assert_eq!(mask, 0);
}

#[test]
fn caps_to_bitmask_invalid_fails() {
    let caps = vec!["CAP_CHOWN".to_string(), "CAP_FAKE".to_string()];
    let result = caps_to_bitmask(&caps);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("CAP_FAKE"));
}

#[test]
fn drop_capabilities_validation_invalid_cap() {
    // Invalid capability name in keep list should fail
    let result = drop_capabilities(Some(&["CAP_FAKE".to_string()]));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("CAP_FAKE"));
}

#[test]
fn drop_capabilities_validation_empty_is_ok() {
    // Empty keep list is valid (drop everything)
    let result = drop_capabilities(Some(&[]));
    assert!(result.is_ok());
}

#[test]
fn drop_capabilities_none_is_ok() {
    // None keep list is valid
    let result = drop_capabilities(None);
    assert!(result.is_ok());
}

#[test]
fn set_supplementary_gids_empty_is_noop() {
    // Empty slice should be a no-op
    set_supplementary_gids(&[]);
    // No panic = test passes
}
