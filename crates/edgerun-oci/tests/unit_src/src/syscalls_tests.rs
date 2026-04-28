use super::*;

#[test]
fn mount_attr_constants_are_correct() {
    // Verify mount attr constants match kernel values
    assert_eq!(mount_attr::RDONLY, 0x00000001);
    assert_eq!(mount_attr::NOSUID, 0x00000002);
    assert_eq!(mount_attr::NODEV, 0x00000004);
    assert_eq!(mount_attr::NOEXEC, 0x00000008);
    assert_eq!(mount_attr::REC, 0x00001000);
    assert_eq!(mount_attr::IDMAP, 0x00100000);
}

#[test]
fn open_tree_constants_are_correct() {
    assert_eq!(open_tree::CLOEXEC, 0x001);
    assert_eq!(open_tree::CLONE, 0x002);
}

#[test]
fn move_mount_constants_are_correct() {
    assert_eq!(move_mount::F_EMPTY_PATH, 0x0100);
    assert_eq!(move_mount::T_EMPTY_PATH, 0x0200);
}

#[test]
fn mount_attr_struct_size() {
    // MountAttr must be exactly 32 bytes (4 × u64)
    assert_eq!(std::mem::size_of::<MountAttr>(), 32);
}

#[test]
fn new_mount_syscall_numbers_match() {
    // open_tree, move_mount, mount_setattr have the same numbers on both archs
    assert_eq!(SYS_OPEN_TREE, 428);
    assert_eq!(SYS_MOVE_MOUNT, 429);
    assert_eq!(SYS_MOUNT_SETATTR, 442);
}
