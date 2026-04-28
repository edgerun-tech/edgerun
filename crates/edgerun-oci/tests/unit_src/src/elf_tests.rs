use super::*;
use crate::bare_rootfs::BareRootfs;
use crate::rootfs_access::build_launch_plan;
use crate::tar_layer::{TarEntry, TarEntryKind, TarLayerSink};

#[test]
fn inspects_launch_plan_elf_program_headers() {
    let mut rootfs = BareRootfs::new();
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: minimal_elf64().len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    let elf = minimal_elf64();
    rootfs.apply_entry(&entry, &elf).unwrap();

    let plan = build_launch_plan(&rootfs, &["/bin/app".into()], &[], "/")
        .unwrap()
        .unwrap();
    let info = inspect_launch_elf(&rootfs, &plan).unwrap();

    assert_eq!(info.path, "bin/app");
    assert_eq!(info.elf_type, OciElfType::Executable);
    assert_eq!(info.machine, OciElfMachine::X86_64);
    assert_eq!(info.entry, 0x401000);
    assert_eq!(info.program_header_offset, ELF_HEADER_LEN as u64);
    assert_eq!(info.program_header_entry_size, ELF64_PHDR_LEN as u16);
    assert_eq!(info.program_header_count, 1);
    assert_eq!(info.interpreter, None);
    assert_eq!(info.program_headers.len(), 1);
    assert!(info.program_headers[0].is_load());
    assert_eq!(
        info.program_headers[0].permissions(),
        OciElfPermissions {
            read: true,
            write: false,
            execute: true
        }
    );
    assert_eq!(info.program_headers[0].virtual_addr, 0x400000);
    assert_eq!(info.program_headers[0].memory_size, 0x2000);

    let load = build_launch_elf_load_plan(&rootfs, &plan).unwrap();
    assert_eq!(load.path, "bin/app");
    assert_eq!(load.elf_type, OciElfType::Executable);
    assert_eq!(load.entry, 0x401000);
    assert_eq!(load.machine, OciElfMachine::X86_64);
    assert_eq!(load.program_header_offset, ELF_HEADER_LEN as u64);
    assert_eq!(load.program_header_entry_size, ELF64_PHDR_LEN as u16);
    assert_eq!(load.program_header_count, 1);
    assert_eq!(load.interpreter, None);
    assert_eq!(load.segments.len(), 1);
    assert_eq!(load.segments[0].file_offset, 0);
    assert_eq!(load.segments[0].virtual_addr, 0x400000);
    assert_eq!(load.segments[0].file_size, 0x1000);
    assert_eq!(load.segments[0].memory_size, 0x2000);
    assert!(load.segments[0].is_executable());
    assert_eq!(
        load.segments[0].permissions(),
        OciElfPermissions {
            read: true,
            write: false,
            execute: true
        }
    );

    let mut buf = [0u8; 16];
    let read = read_elf_segment_chunk(&rootfs, &load, 0, 0, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);

    buf.fill(0xaa);
    let read = read_elf_segment_chunk(&rootfs, &load, 0, 0x0ff8, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 8,
            zero_fill_bytes: 8
        }
    );
    assert!(buf[8..].iter().all(|byte| *byte == 0));

    buf.fill(0xaa);
    let read = read_elf_segment_chunk(&rootfs, &load, 0, 0x1000, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 0,
            zero_fill_bytes: 16
        }
    );
    assert!(buf.iter().all(|byte| *byte == 0));
    assert!(matches!(
        read_elf_segment_chunk(&rootfs, &load, 1, 0, &mut buf),
        Err(OciElfError::InvalidSegmentIndex(1))
    ));

    let map = build_elf_memory_map(&load).unwrap();
    assert_eq!(map.page_size, 4096);
    assert_eq!(map.load_bias, 0);
    assert_eq!(map.entry, 0x401000);
    assert_eq!(map.mappings.len(), 1);
    assert_eq!(map.mappings[0].segment_index, 0);
    assert_eq!(map.mappings[0].map_start, 0x400000);
    assert_eq!(map.mappings[0].map_size, 0x2000);
    assert_eq!(map.mappings[0].segment_start, 0x400000);
    assert_eq!(map.mappings[0].segment_size, 0x2000);
    assert_eq!(
        map.mappings[0].permissions(),
        OciElfPermissions {
            read: true,
            write: false,
            execute: true
        }
    );
    assert!(matches!(
        build_elf_memory_map_with_page_size(&load, 3000),
        Err(OciElfError::InvalidPageSize(3000))
    ));
    let biased_map = build_elf_memory_map_with_load_bias(&load, 0x10000000).unwrap();
    assert_eq!(biased_map.load_bias, 0x10000000);
    assert_eq!(biased_map.entry, 0x10401000);
    assert_eq!(biased_map.mappings[0].map_start, 0x10400000);
    assert_eq!(biased_map.mappings[0].segment_start, 0x10400000);

    buf.fill(0xaa);
    let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, 0x0ff8, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 8,
            zero_fill_bytes: 8
        }
    );
    assert!(buf[8..].iter().all(|byte| *byte == 0));

    let prepared = prepare_oci_elf_program(
        &rootfs,
        &["app".into(), "--flag".into()],
        &["PATH=/bin".into()],
        "/",
    )
    .unwrap()
    .unwrap();
    assert_eq!(prepared.launch.argv, vec!["app", "--flag"]);
    assert_eq!(prepared.runtime.executable.path, "bin/app");
    assert!(prepared.runtime.interpreter.is_none());
    assert_eq!(prepared.memory.executable.entry, 0x401000);
    assert!(prepared.memory.interpreter.is_none());
    let auxv = build_elf64_auxv(&prepared.runtime, &prepared.memory).unwrap();
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHDR), Some(0x400040));
    assert_eq!(
        auxv_value(&auxv, OCI_ELF_AT_PHENT),
        Some(ELF64_PHDR_LEN as u64)
    );
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHNUM), Some(1));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PAGESZ), Some(4096));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_BASE), Some(0));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_FLAGS), Some(0));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_ENTRY), Some(0x401000));
    let mappings = build_elf_runtime_mapping_list(&prepared.runtime, &prepared.memory).unwrap();
    assert_eq!(mappings.len(), 1);
    assert_eq!(mappings[0].image, OciElfImage::Executable);
    assert_eq!(mappings[0].path, "bin/app");
    assert_eq!(mappings[0].mapping_index, 0);
    assert_eq!(mappings[0].mapping.map_start, 0x400000);
    let layout = build_elf_runtime_layout(&prepared.runtime, &prepared.memory).unwrap();
    assert_eq!(layout.map_start, 0x400000);
    assert_eq!(layout.map_end, 0x402000);
    assert_eq!(layout.mapped_bytes, 0x2000);
    assert_eq!(layout.mappings.len(), 1);

    let mut mapper = RecordingMapper::new();
    let mut scratch = [0u8; 512];
    let mut stack = [0u8; 512];
    let stack_base = 0x7000_0000u64;
    let stack_top = stack_base + stack.len() as u64;
    let launch = load_prepared_elf64_program(
        &rootfs,
        &prepared,
        &mut mapper,
        &mut scratch,
        stack_base,
        stack_top,
        &mut stack,
    )
    .unwrap();
    assert_eq!(launch.entry_point, 0x401000);
    assert_eq!(mapper.mapped.len(), 1);
    assert_eq!(mapper.mapped[0].mapping.map_start, 0x400000);
    assert_eq!(mapper.protected.len(), 1);
    assert_eq!(mapper.writes.len(), 16);
    assert_eq!(mapper.writes[0].0, 0x400000);
    assert_eq!(&mapper.writes[0].1[..4], &[0x7f, b'E', b'L', b'F']);
    assert_eq!(mapper.writes[15].0, 0x401e00);
    assert!(mapper.writes[15].1.iter().all(|byte| *byte == 0));
    assert_eq!(mapper.stack_base, Some(stack_base));
    assert_eq!(mapper.stack_bytes.as_deref(), Some(stack.as_slice()));

    let mut buf = [0u8; 16];
    let read = read_elf_runtime_segment_chunk(
        &rootfs,
        &prepared.runtime,
        OciElfImage::Executable,
        0,
        0,
        &mut buf,
    )
    .unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
    let read = read_elf_runtime_mapping_chunk(
        &rootfs,
        &prepared.runtime,
        &prepared.memory,
        OciElfImage::Executable,
        0,
        0,
        &mut buf,
    )
    .unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
    assert!(matches!(
        read_elf_runtime_segment_chunk(
            &rootfs,
            &prepared.runtime,
            OciElfImage::Interpreter,
            0,
            0,
            &mut buf
        ),
        Err(OciElfError::MissingInterpreter)
    ));

    let missing =
        prepare_oci_elf_program(&rootfs, &["missing".into()], &["PATH=/bin".into()], "/").unwrap();
    assert!(missing.is_none());
}

#[test]
fn reads_dynamic_elf_interpreter() {
    let mut rootfs = BareRootfs::new();
    let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let info = inspect_elf(&rootfs, "bin/app").unwrap();
    assert_eq!(info.interpreter, Some("/lib64/ld-linux-x86-64.so.2".into()));

    let load = build_elf_load_plan(&rootfs, "bin/app").unwrap();
    assert_eq!(load.interpreter, Some("/lib64/ld-linux-x86-64.so.2".into()));

    let interpreter = minimal_elf64();
    let interpreter_entry = TarEntry {
        path: "lib64/ld-linux-x86-64.so.2".into(),
        kind: TarEntryKind::Regular,
        size: interpreter.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs
        .apply_entry(&interpreter_entry, &interpreter)
        .unwrap();

    let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
    assert_eq!(runtime.executable.path, "bin/app");
    assert_eq!(
        runtime.executable.interpreter,
        Some("/lib64/ld-linux-x86-64.so.2".into())
    );
    let interpreter = runtime.interpreter.unwrap();
    assert_eq!(interpreter.path, "lib64/ld-linux-x86-64.so.2");
    assert_eq!(interpreter.interpreter, None);

    let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
    let maps = build_elf_runtime_memory_map(&runtime).unwrap();
    assert_eq!(maps.executable.entry, 0x401000);
    assert!(maps.interpreter.is_some());
    assert_eq!(maps.interpreter.as_ref().unwrap().entry, 0x401000);
    let maps = build_elf_runtime_memory_map_with_page_size(&runtime, 0x2000).unwrap();
    assert_eq!(maps.executable.page_size, 0x2000);
    assert_eq!(maps.interpreter.as_ref().unwrap().page_size, 0x2000);
    let biased_maps = build_elf_runtime_memory_map_with_page_size_and_load_bias(
        &runtime,
        0x2000,
        OciElfLoadBias {
            executable: 0x10000000,
            interpreter: 0x20000000,
        },
    )
    .unwrap();
    assert_eq!(biased_maps.executable.load_bias, 0x10000000);
    assert_eq!(biased_maps.executable.entry, 0x10401000);
    assert_eq!(
        biased_maps.interpreter.as_ref().unwrap().load_bias,
        0x20000000
    );
    assert_eq!(biased_maps.interpreter.as_ref().unwrap().entry, 0x20401000);
    let auxv = build_elf64_auxv(&runtime, &biased_maps).unwrap();
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHDR), Some(0x10400040));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PHNUM), Some(2));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_PAGESZ), Some(0x2000));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_BASE), Some(0x20000000));
    assert_eq!(auxv_value(&auxv, OCI_ELF_AT_ENTRY), Some(0x10401000));

    let biased_prepared = prepare_oci_elf_program_with_page_size_and_load_bias(
        &rootfs,
        &["app".into()],
        &["PATH=/bin".into()],
        "/",
        0x2000,
        OciElfLoadBias {
            executable: 0x10000000,
            interpreter: 0x20000000,
        },
    )
    .unwrap()
    .unwrap();
    let mut stack = [0u8; 512];
    let stack_base = 0x7000_0000u64;
    let stack_top = stack_base + stack.len() as u64;
    let launch =
        write_prepared_elf64_launch_state(&biased_prepared, stack_base, stack_top, &mut stack)
            .unwrap();
    assert_eq!(launch.executable_entry, 0x10401000);
    assert_eq!(launch.interpreter_entry, Some(0x20401000));
    assert_eq!(launch.entry_point, 0x20401000);
    assert_eq!(
        stack_auxv_value(&stack, stack_base, launch.stack.auxv_addr, OCI_ELF_AT_BASE),
        0x20000000
    );
    let mappings =
        build_elf_runtime_mapping_list(&biased_prepared.runtime, &biased_prepared.memory).unwrap();
    assert_eq!(mappings.len(), 2);
    assert_eq!(mappings[0].image, OciElfImage::Executable);
    assert_eq!(mappings[0].path, "bin/app");
    assert_eq!(mappings[0].mapping.map_start, 0x10400000);
    assert_eq!(mappings[1].image, OciElfImage::Interpreter);
    assert_eq!(mappings[1].path, "lib64/ld-linux-x86-64.so.2");
    assert_eq!(mappings[1].mapping.map_start, 0x20400000);
    let layout =
        build_elf_runtime_layout(&biased_prepared.runtime, &biased_prepared.memory).unwrap();
    assert_eq!(layout.map_start, 0x10400000);
    assert_eq!(layout.map_end, 0x20402000);
    assert_eq!(layout.mapped_bytes, 0x4000);
    assert_eq!(layout.mappings.len(), 2);
    let mut buf = [0u8; 16];
    let read = read_elf_runtime_mapping_list_chunk(
        &rootfs,
        &biased_prepared.runtime,
        &biased_prepared.memory,
        &mappings[1],
        0,
        &mut buf,
    )
    .unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);

    let prepared = prepare_oci_elf_program_with_page_size(
        &rootfs,
        &["app".into()],
        &["PATH=/bin".into()],
        "/",
        0x2000,
    )
    .unwrap()
    .unwrap();
    assert_eq!(prepared.runtime.executable.path, "bin/app");
    assert_eq!(
        prepared.runtime.interpreter.as_ref().unwrap().path,
        "lib64/ld-linux-x86-64.so.2"
    );
    assert_eq!(prepared.memory.executable.page_size, 0x2000);
    assert_eq!(
        prepared.memory.interpreter.as_ref().unwrap().page_size,
        0x2000
    );

    let mut stack = [0u8; 512];
    let stack_base = 0x7000_0000u64;
    let stack_top = stack_base + stack.len() as u64;
    let launch =
        write_prepared_elf64_launch_state(&prepared, stack_base, stack_top, &mut stack).unwrap();
    assert_eq!(launch.executable_entry, 0x401000);
    assert_eq!(launch.interpreter_entry, Some(0x401000));
    assert_eq!(launch.entry_point, 0x401000);
    assert_eq!(
        stack_auxv_value(&stack, stack_base, launch.stack.auxv_addr, OCI_ELF_AT_BASE),
        0
    );

    let mut buf = [0u8; 16];
    let read = read_elf_runtime_segment_chunk(
        &rootfs,
        &prepared.runtime,
        OciElfImage::Interpreter,
        0,
        0,
        &mut buf,
    )
    .unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
    let read = read_elf_runtime_mapping_chunk(
        &rootfs,
        &prepared.runtime,
        &prepared.memory,
        OciElfImage::Interpreter,
        0,
        0,
        &mut buf,
    )
    .unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 16,
            zero_fill_bytes: 0
        }
    );
    assert_eq!(&buf[..4], &[0x7f, b'E', b'L', b'F']);
}

#[test]
fn reads_page_aligned_mapping_prefix_and_tail_zeroes() {
    let mut rootfs = BareRootfs::new();
    let mut elf = minimal_elf64();
    let segment_start = 0x400123u64;
    let ph = ELF_HEADER_LEN;
    elf[ph + 16..ph + 24].copy_from_slice(&segment_start.to_le_bytes());
    elf[ph + 24..ph + 32].copy_from_slice(&segment_start.to_le_bytes());
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let load = build_elf_load_plan(&rootfs, "bin/app").unwrap();
    let map = build_elf_memory_map(&load).unwrap();
    assert_eq!(map.mappings[0].map_start, 0x400000);
    assert_eq!(map.mappings[0].segment_start, segment_start);

    let mut buf = [0xaau8; 0x130];
    let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, 0, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 0x0d,
            zero_fill_bytes: 0x123
        }
    );
    assert!(buf[..0x123].iter().all(|byte| *byte == 0));
    assert_eq!(&buf[0x123..0x127], &[0x7f, b'E', b'L', b'F']);

    let mut buf = [0xaau8; 16];
    let tail_offset = 0x123 + 0x1000 - 8;
    let read = read_elf_mapping_chunk(&rootfs, &load, &map, 0, tail_offset, &mut buf).unwrap();
    assert_eq!(
        read,
        OciElfSegmentRead {
            bytes_read: 8,
            zero_fill_bytes: 8
        }
    );
    assert!(buf[8..].iter().all(|byte| *byte == 0));
}

#[test]
fn writes_elf64_initial_stack_for_argv_env_and_auxv() {
    let stack_base = 0x7000_0000u64;
    let mut stack = [0xaau8; 512];
    let stack_top = stack_base + stack.len() as u64;
    let plan = write_elf64_initial_stack(
        stack_base,
        stack_top,
        &mut stack,
        &["app".into(), "--flag".into()],
        &["PATH=/bin".into()],
        &[
            OciElfAuxvEntry {
                key: 3,
                value: 0x400040,
            },
            OciElfAuxvEntry {
                key: 9,
                value: 0x401000,
            },
        ],
    )
    .unwrap();

    assert_eq!(plan.stack_pointer % 16, 0);
    assert_eq!(stack_u64(&stack, stack_base, plan.argc_addr), 2);
    let argv0 = stack_u64(&stack, stack_base, plan.argv_ptrs_addr);
    let argv1 = stack_u64(&stack, stack_base, plan.argv_ptrs_addr + 8);
    assert_eq!(stack_u64(&stack, stack_base, plan.argv_ptrs_addr + 16), 0);
    assert_eq!(stack_cstr(&stack, stack_base, argv0), "app");
    assert_eq!(stack_cstr(&stack, stack_base, argv1), "--flag");

    let env0 = stack_u64(&stack, stack_base, plan.env_ptrs_addr);
    assert_eq!(stack_u64(&stack, stack_base, plan.env_ptrs_addr + 8), 0);
    assert_eq!(stack_cstr(&stack, stack_base, env0), "PATH=/bin");
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr), 3);
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 8), 0x400040);
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 16), 9);
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 24), 0x401000);
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 32), 0);
    assert_eq!(stack_u64(&stack, stack_base, plan.auxv_addr + 40), 0);
}

#[test]
fn prepares_biased_program_and_writes_prepared_initial_stack() {
    let mut rootfs = BareRootfs::new();
    let elf = minimal_elf64();
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let prepared = prepare_oci_elf_program_with_load_bias(
        &rootfs,
        &["app".into(), "--flag".into()],
        &["PATH=/bin".into()],
        "/",
        OciElfLoadBias {
            executable: 0x10000000,
            interpreter: 0,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(prepared.memory.executable.entry, 0x10401000);

    let stack_base = 0x7000_0000u64;
    let mut stack = [0u8; 512];
    let stack_top = stack_base + stack.len() as u64;
    let stack_plan =
        write_prepared_elf64_initial_stack(&prepared, stack_base, stack_top, &mut stack).unwrap();
    assert_eq!(stack_u64(&stack, stack_base, stack_plan.argc_addr), 2);
    assert_eq!(
        stack_u64(&stack, stack_base, stack_plan.auxv_addr + 8),
        0x10400040
    );
    assert_eq!(
        stack_auxv_value(&stack, stack_base, stack_plan.auxv_addr, OCI_ELF_AT_ENTRY),
        0x10401000
    );

    let mut stack = [0u8; 512];
    let launch =
        write_prepared_elf64_launch_state(&prepared, stack_base, stack_top, &mut stack).unwrap();
    assert_eq!(launch.executable_entry, 0x10401000);
    assert_eq!(launch.interpreter_entry, None);
    assert_eq!(launch.entry_point, 0x10401000);
    assert_eq!(launch.stack_pointer, launch.stack.stack_pointer);
}

#[test]
fn prepares_and_loads_oci_elf_program() {
    let mut rootfs = BareRootfs::new();
    let elf = minimal_elf64();
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let stack_base = 0x7000_0000u64;
    let mut stack = [0u8; 512];
    let stack_top = stack_base + stack.len() as u64;
    let mut scratch = [0u8; 128];
    let mut mapper = RecordingMapper::new();
    let launch = prepare_and_load_oci_elf_program(
        &rootfs,
        &["app".into()],
        &["PATH=/bin".into()],
        "/",
        &mut mapper,
        &mut scratch,
        stack_base,
        stack_top,
        &mut stack,
    )
    .unwrap()
    .unwrap();

    assert_eq!(launch.entry_point, 0x401000);
    assert_eq!(mapper.mapped.len(), 1);
    assert_eq!(mapper.protected.len(), 1);
    assert_eq!(mapper.stack_base, Some(stack_base));
    assert_eq!(mapper.stack_size, Some(stack.len() as u64));
    assert!(mapper
        .stack_bytes
        .as_ref()
        .unwrap()
        .iter()
        .any(|byte| *byte != 0));
    assert_eq!(&mapper.writes[0].1[..4], &[0x7f, b'E', b'L', b'F']);
}

#[test]
fn rejects_invalid_initial_stack_inputs() {
    let stack_base = 0x7000_0000u64;
    let mut stack = [0u8; 32];
    let stack_top = stack_base + stack.len() as u64;

    assert!(matches!(
        write_elf64_initial_stack_aligned(stack_base, stack_top, &mut stack, &[], &[], &[], 24),
        Err(OciElfError::InvalidStackAlignment(24))
    ));
    assert!(matches!(
        write_elf64_initial_stack(
            stack_base,
            stack_top,
            &mut stack,
            &["bad\0arg".into()],
            &[],
            &[]
        ),
        Err(OciElfError::InvalidStackString)
    ));
    assert!(matches!(
        write_elf64_initial_stack(
            stack_base,
            stack_top + 8,
            &mut stack,
            &["app".into()],
            &[],
            &[]
        ),
        Err(OciElfError::StackTooSmall)
    ));
}

#[test]
fn dynamic_runtime_plan_requires_interpreter_file() {
    let mut rootfs = BareRootfs::new();
    let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let err = build_elf_runtime_plan(&rootfs, "bin/app").unwrap_err();
    assert!(matches!(err, OciElfError::NotFound(path) if path == "lib64/ld-linux-x86-64.so.2"));
}

#[test]
fn rejects_overlapping_runtime_image_mappings() {
    let mut rootfs = BareRootfs::new();
    let elf = elf64_with_interpreter("/lib64/ld-linux-x86-64.so.2");
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();
    let interpreter = minimal_elf64();
    let interpreter_entry = TarEntry {
        path: "lib64/ld-linux-x86-64.so.2".into(),
        kind: TarEntryKind::Regular,
        size: interpreter.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs
        .apply_entry(&interpreter_entry, &interpreter)
        .unwrap();

    let runtime = build_elf_runtime_plan(&rootfs, "bin/app").unwrap();
    let memory = build_elf_runtime_memory_map_with_load_bias(
        &runtime,
        OciElfLoadBias {
            executable: 0,
            interpreter: 0,
        },
    )
    .unwrap();
    assert!(matches!(
        build_elf_runtime_mapping_list(&runtime, &memory),
        Err(OciElfError::OverlappingLoadSegments)
    ));
    assert!(matches!(
        build_elf_runtime_layout(&runtime, &memory),
        Err(OciElfError::OverlappingLoadSegments)
    ));
}

#[test]
fn rejects_non_elf_executables() {
    let mut rootfs = BareRootfs::new();
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: 4,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, b"nope").unwrap();

    let err = inspect_elf(&rootfs, "bin/app").unwrap_err();
    assert!(matches!(err, OciElfError::ShortRead { .. }));
}

#[test]
fn rejects_invalid_load_segment_ranges() {
    let mut rootfs = BareRootfs::new();
    let mut elf = minimal_elf64();
    let ph = ELF_HEADER_LEN;
    let bad_file_size = elf.len() as u64 + 1;
    elf[ph + 32..ph + 40].copy_from_slice(&bad_file_size.to_le_bytes());
    let entry = TarEntry {
        path: "bin/app".into(),
        kind: TarEntryKind::Regular,
        size: elf.len() as u64,
        mode: 0o755,
        uid: 0,
        gid: 0,
        mtime: 0,
        dev_major: None,
        dev_minor: None,
        link_name: None,
        whiteout: None,
    };
    rootfs.apply_entry(&entry, &elf).unwrap();

    let err = build_elf_load_plan(&rootfs, "bin/app").unwrap_err();
    assert!(matches!(err, OciElfError::InvalidLoadSegment(_)));
}

#[test]
fn rejects_overlapping_memory_mappings() {
    let load = OciElfLoadPlan {
        path: "bin/app".into(),
        elf_type: OciElfType::Executable,
        entry: 0x1000,
        machine: OciElfMachine::X86_64,
        program_header_offset: ELF_HEADER_LEN as u64,
        program_header_entry_size: ELF64_PHDR_LEN as u16,
        program_header_count: 1,
        interpreter: None,
        segments: vec![
            OciElfLoadSegment {
                file_offset: 0,
                virtual_addr: 0x1000,
                file_size: 0x1000,
                memory_size: 0x1000,
                flags: PF_X,
                align: 0x1000,
            },
            OciElfLoadSegment {
                file_offset: 0x1000,
                virtual_addr: 0x1800,
                file_size: 0x1000,
                memory_size: 0x1000,
                flags: 0,
                align: 0x1000,
            },
        ],
    };

    assert!(matches!(
        build_elf_memory_map(&load),
        Err(OciElfError::OverlappingLoadSegments)
    ));
}

fn auxv_value(auxv: &[OciElfAuxvEntry], key: u64) -> Option<u64> {
    auxv.iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value)
}

fn stack_u64(stack: &[u8], stack_base: u64, addr: u64) -> u64 {
    let offset = (addr - stack_base) as usize;
    read_u64(stack, offset)
}

fn stack_cstr(stack: &[u8], stack_base: u64, addr: u64) -> &str {
    let offset = (addr - stack_base) as usize;
    let end = stack[offset..]
        .iter()
        .position(|byte| *byte == 0)
        .map(|len| offset + len)
        .unwrap();
    core::str::from_utf8(&stack[offset..end]).unwrap()
}

fn stack_auxv_value(stack: &[u8], stack_base: u64, auxv_addr: u64, key: u64) -> u64 {
    let mut addr = auxv_addr;
    loop {
        let entry_key = stack_u64(stack, stack_base, addr);
        let value = stack_u64(stack, stack_base, addr + 8);
        if entry_key == key || entry_key == OCI_ELF_AT_NULL {
            return value;
        }
        addr += 16;
    }
}

struct RecordingMapper {
    mapped: Vec<OciElfRuntimeMapping>,
    writes: Vec<(u64, Vec<u8>)>,
    protected: Vec<OciElfRuntimeMapping>,
    stack_base: Option<u64>,
    stack_size: Option<u64>,
    stack_bytes: Option<Vec<u8>>,
}

impl RecordingMapper {
    fn new() -> Self {
        Self {
            mapped: Vec::new(),
            writes: Vec::new(),
            protected: Vec::new(),
            stack_base: None,
            stack_size: None,
            stack_bytes: None,
        }
    }
}

impl OciElfMapper for RecordingMapper {
    fn map_elf_region(&mut self, mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
        self.mapped.push(mapping.clone());
        Ok(())
    }

    fn write_elf_region(
        &mut self,
        mapping: &OciElfRuntimeMapping,
        mapping_offset: u64,
        bytes: &[u8],
    ) -> Result<(), OciElfError> {
        let addr = mapping
            .mapping
            .map_start
            .checked_add(mapping_offset)
            .ok_or(OciElfError::Overflow)?;
        self.writes.push((addr, bytes.to_vec()));
        Ok(())
    }

    fn protect_elf_region(&mut self, mapping: &OciElfRuntimeMapping) -> Result<(), OciElfError> {
        self.protected.push(mapping.clone());
        Ok(())
    }

    fn map_stack_region(&mut self, stack_base: u64, stack_size: u64) -> Result<(), OciElfError> {
        self.stack_base = Some(stack_base);
        self.stack_size = Some(stack_size);
        Ok(())
    }

    fn write_stack_region(&mut self, _stack_base: u64, bytes: &[u8]) -> Result<(), OciElfError> {
        self.stack_bytes = Some(bytes.to_vec());
        Ok(())
    }
}

fn minimal_elf64() -> Vec<u8> {
    let mut out = vec![0u8; 0x1000];
    out[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    out[EI_CLASS] = ELFCLASS64;
    out[EI_DATA] = ELFDATA2LSB;
    out[EI_VERSION] = EV_CURRENT;
    out[16..18].copy_from_slice(&2u16.to_le_bytes());
    out[18..20].copy_from_slice(&62u16.to_le_bytes());
    out[20..24].copy_from_slice(&1u32.to_le_bytes());
    out[24..32].copy_from_slice(&0x401000u64.to_le_bytes());
    out[32..40].copy_from_slice(&(ELF_HEADER_LEN as u64).to_le_bytes());
    out[52..54].copy_from_slice(&(ELF_HEADER_LEN as u16).to_le_bytes());
    out[54..56].copy_from_slice(&(ELF64_PHDR_LEN as u16).to_le_bytes());
    out[56..58].copy_from_slice(&1u16.to_le_bytes());

    let ph = ELF_HEADER_LEN;
    out[ph..ph + 4].copy_from_slice(&PT_LOAD.to_le_bytes());
    out[ph + 4..ph + 8].copy_from_slice(&5u32.to_le_bytes());
    out[ph + 8..ph + 16].copy_from_slice(&0u64.to_le_bytes());
    out[ph + 16..ph + 24].copy_from_slice(&0x400000u64.to_le_bytes());
    out[ph + 24..ph + 32].copy_from_slice(&0x400000u64.to_le_bytes());
    out[ph + 32..ph + 40].copy_from_slice(&0x1000u64.to_le_bytes());
    out[ph + 40..ph + 48].copy_from_slice(&0x2000u64.to_le_bytes());
    out[ph + 48..ph + 56].copy_from_slice(&0x1000u64.to_le_bytes());
    out
}

fn elf64_with_interpreter(interpreter: &str) -> Vec<u8> {
    let mut out = minimal_elf64();
    let interp_offset = 0x200usize;
    let phoff = ELF_HEADER_LEN;
    let interp_ph = phoff + ELF64_PHDR_LEN;
    let interp_len = interpreter.len() + 1;
    out[32..40].copy_from_slice(&(phoff as u64).to_le_bytes());
    out[56..58].copy_from_slice(&2u16.to_le_bytes());

    out[interp_ph..interp_ph + 4].copy_from_slice(&PT_INTERP.to_le_bytes());
    out[interp_ph + 8..interp_ph + 16].copy_from_slice(&(interp_offset as u64).to_le_bytes());
    out[interp_ph + 32..interp_ph + 40].copy_from_slice(&(interp_len as u64).to_le_bytes());
    out[interp_ph + 40..interp_ph + 48].copy_from_slice(&(interp_len as u64).to_le_bytes());
    out[interp_offset..interp_offset + interpreter.len()].copy_from_slice(interpreter.as_bytes());
    out[interp_offset + interpreter.len()] = 0;
    out
}
