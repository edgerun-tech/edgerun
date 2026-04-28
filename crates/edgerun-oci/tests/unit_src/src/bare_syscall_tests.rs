use super::*;

#[test]
fn dispatches_write_in_chunks() {
    let memory = OciSliceSyscallMemory {
        base: 0x1000,
        bytes: b"hello world",
    };
    let mut sink = OciBufferSyscallSink::default();
    let mut scratch = [0u8; 5];
    let action = dispatch_linux_syscall(
        &memory,
        &mut sink,
        &mut scratch,
        OCI_LINUX_SYS_WRITE,
        [1, 0x1000, 11, 0, 0, 0],
    )
    .unwrap();

    assert_eq!(action, OciSyscallAction::Return(11));
    assert_eq!(sink.writes.len(), 3);
    assert_eq!(sink.writes[0], (1, b"hello".to_vec()));
    assert_eq!(sink.writes[1], (1, b" worl".to_vec()));
    assert_eq!(sink.writes[2], (1, b"d".to_vec()));
}

#[test]
fn dispatches_exit() {
    let memory = OciSliceSyscallMemory {
        base: 0,
        bytes: &[],
    };
    let mut sink = OciBufferSyscallSink::default();
    let mut scratch = [0u8; 1];
    assert_eq!(
        dispatch_linux_syscall(
            &memory,
            &mut sink,
            &mut scratch,
            OCI_LINUX_SYS_EXIT,
            [42, 0, 0, 0, 0, 0],
        )
        .unwrap(),
        OciSyscallAction::Exit(42)
    );
}

#[test]
fn dispatches_x86_64_frame_and_updates_rax() {
    let memory = OciSliceSyscallMemory {
        base: 0x1000,
        bytes: b"hello",
    };
    let mut sink = OciBufferSyscallSink::default();
    let mut scratch = [0u8; 8];
    let mut frame = OciX86_64SyscallFrame {
        rax: OCI_LINUX_SYS_WRITE,
        rdi: 2,
        rsi: 0x1000,
        rdx: 5,
        ..OciX86_64SyscallFrame::default()
    };

    let action =
        dispatch_x86_64_linux_syscall_frame(&memory, &mut sink, &mut scratch, &mut frame).unwrap();
    assert_eq!(action, OciSyscallAction::Return(5));
    assert_eq!(frame.rax, 5);
    assert_eq!(sink.writes, vec![(2, b"hello".to_vec())]);
}
