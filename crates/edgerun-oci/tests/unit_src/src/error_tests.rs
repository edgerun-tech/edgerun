use super::*;

#[test]
fn all_errors_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<LifecycleError>();
    assert_send_sync::<RootfsError>();
    assert_send_sync::<SeccompError>();
    assert_send_sync::<CgroupError>();
    assert_send_sync::<CapabilityError>();
    assert_send_sync::<NamespaceError>();
    assert_send_sync::<ConfigError>();
    assert_send_sync::<FifoError>();
}
