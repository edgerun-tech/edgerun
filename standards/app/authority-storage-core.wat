;; Authority storage semantics plundered from edgerun-vfs and edgerun-virtual-disk.
  (func (export "authority_vfs_wire_abi_version") (result i32)
    i32.const 1)

  (func (export "authority_vfs_default_object_packet_bytes") (result i32)
    i32.const 65536)

  (func (export "authority_vfs_compression_none") (result i32)
    i32.const 0)

  (func (export "authority_vfs_compression_deflate_raw") (result i32)
    i32.const 1)

  (func (export "authority_vfs_seal_aes256_gcm") (result i32)
    i32.const 1)

  (func (export "authority_vfs_wire_record_tag") (param $kind i32) (result i32)
    ;; 0 packet, 1 file ref, 2 transform ref, 3 seal request,
    ;; 4 unseal request, 5 tree manifest.
    (if (result i32) (i32.le_u (local.get $kind) (i32.const 5))
      (then local.get $kind)
      (else i32.const -1)))

  (func (export "authority_vfs_packet_count") (param $object_len i32) (param $max_payload i32) (result i32)
    (if (i32.eqz (local.get $max_payload)) (then (return (i32.const -1))))
    (if (i32.eqz (local.get $object_len)) (then (return (i32.const 1))))
    (i32.add
      (i32.div_u (i32.sub (local.get $object_len) (i32.const 1)) (local.get $max_payload))
      (i32.const 1)))

  (func (export "authority_vfs_packet_offset") (param $packet_index i32) (param $max_payload i32) (result i32)
    (i32.mul (local.get $packet_index) (local.get $max_payload)))

  (func (export "authority_vfs_packet_shape_result")
    (param $abi i32) (param $object_len i32) (param $packet_index i32)
    (param $packet_count i32) (param $offset i32) (param $payload_len i32)
    (result i32)
    ;; 0 ok, 1 invalid shape, 2 missing packet/index, 3 object too large.
    (if (i32.or (i32.ne (local.get $abi) (i32.const 1)) (i32.eqz (local.get $packet_count)))
      (then (return (i32.const 1))))
    (if (i32.ge_u (local.get $packet_index) (local.get $packet_count))
      (then (return (i32.const 2))))
    (if (i32.lt_u (i32.add (local.get $offset) (local.get $payload_len)) (local.get $offset))
      (then (return (i32.const 3))))
    (if (i32.gt_u (i32.add (local.get $offset) (local.get $payload_len)) (local.get $object_len))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.lt_u (i32.add (local.get $packet_index) (i32.const 1)) (local.get $packet_count))
          (i32.eqz (local.get $payload_len)))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_vfs_transform_result")
    (param $abi i32) (param $compression i32) (param $seal i32) (param $hash_matches i32)
    (result i32)
    ;; validate_transform: ABI 1, AES-256-GCM seal, none/raw-deflate compression, hash match.
    (if (i32.ne (local.get $abi) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.ne (local.get $seal) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz
          (i32.or
            (i32.eq (local.get $compression) (i32.const 0))
            (i32.eq (local.get $compression) (i32.const 1))))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $hash_matches)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_vfs_file_ref_result")
    (param $abi i32) (param $path_ok i32) (param $file_hash_matches i32)
    (param $object_hash_matches i32) (param $object_len_matches i32)
    (result i32)
    (if (i32.ne (local.get $abi) (i32.const 1)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $path_ok)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $file_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $object_hash_matches)) (then (return (i32.const 3))))
    (if (i32.eqz (local.get $object_len_matches)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "authority_vfs_manifest_order") (param $left_path_cmp i32) (result i32)
    ;; files_to_manifest sorts VfsFileRef entries by path before hashing.
    (if (result i32) (i32.le_s (local.get $left_path_cmp) (i32.const 0))
      (then i32.const 0)
      (else i32.const 1)))

  (func (export "authority_vfs_seal_request_payload_result")
    (param $compression i32) (param $adapter_available i32) (result i32)
    ;; none keeps payload bytes; raw deflate requires an explicit WAT adapter and fails closed.
    (if (i32.eq (local.get $compression) (i32.const 0)) (then (return (i32.const 0))))
    (if (i32.eq (local.get $compression) (i32.const 1))
      (then
        (if (local.get $adapter_available)
          (then (return (i32.const 0)))
          (else (return (i32.const 1))))))
    (return (i32.const 2)))

  (func (export "authority_vfs_memory_write_usage") (param $old_total i32) (param $old_size i32) (param $new_size i32) (result i32)
    (i32.add (i32.sub (local.get $old_total) (local.get $old_size)) (local.get $new_size)))

  (func (export "authority_vfs_text_edit_result") (param $exists i32) (param $deleted i32) (param $is_text i32) (result i32)
    ;; 0 ok, 1 deleted, 2 not found, 3 binary.
    (if (local.get $deleted) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $exists)) (then (return (i32.const 2))))
    (if (i32.eqz (local.get $is_text)) (then (return (i32.const 3))))
    i32.const 0)

  (func (export "authority_vfs_path_result") (param $ptr i32) (param $len i32) (result i32)
    (local $start i32) (local $end i32) (local $i i32) (local $seg_start i32) (local $seg_len i32) (local $ch i32)
    (local.set $start (local.get $ptr))
    (local.set $end (i32.add (local.get $ptr) (local.get $len)))
    (loop $trim_left
      (if (i32.and (i32.lt_u (local.get $start) (local.get $end)) (i32.eq (i32.load8_u (local.get $start)) (i32.const 47)))
        (then
          (local.set $start (i32.add (local.get $start) (i32.const 1)))
          (br $trim_left))))
    (loop $trim_right
      (if (i32.and (i32.lt_u (local.get $start) (local.get $end)) (i32.eq (i32.load8_u (i32.sub (local.get $end) (i32.const 1))) (i32.const 47)))
        (then
          (local.set $end (i32.sub (local.get $end) (i32.const 1)))
          (br $trim_right))))
    (if (i32.eq (local.get $start) (local.get $end)) (then (return (i32.const 1))))
    (local.set $i (local.get $start))
    (local.set $seg_start (local.get $start))
    (block $done
      (loop $scan
        (br_if $done (i32.ge_u (local.get $i) (local.get $end)))
        (local.set $ch (i32.load8_u (local.get $i)))
        (if (i32.eq (local.get $ch) (i32.const 92)) (then (return (i32.const 1))))
        (if (i32.eq (local.get $ch) (i32.const 47))
          (then
            (local.set $seg_len (i32.sub (local.get $i) (local.get $seg_start)))
            (if (call $authority_bad_path_segment (local.get $seg_start) (local.get $seg_len))
              (then (return (i32.const 1))))
            (local.set $seg_start (i32.add (local.get $i) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $scan)))
    (local.set $seg_len (i32.sub (local.get $end) (local.get $seg_start)))
    (if (call $authority_bad_path_segment (local.get $seg_start) (local.get $seg_len))
      (then (return (i32.const 1))))
    i32.const 0)

  (func $authority_bad_path_segment (param $ptr i32) (param $len i32) (result i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 1))))
    (if (i32.and
          (i32.eq (local.get $len) (i32.const 1))
          (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 46)))
      (then (return (i32.const 1))))
    (if (i32.and
          (i32.eq (local.get $len) (i32.const 2))
          (i32.and
            (i32.eq (i32.load8_u (local.get $ptr)) (i32.const 46))
            (i32.eq (i32.load8_u (i32.add (local.get $ptr) (i32.const 1))) (i32.const 46))))
      (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_disk_format_from_path") (param $ptr i32) (param $len i32) (result i32)
    ;; raw=1, qcow2=2, vhd=3, vhdx=4; unknown/no extension defaults raw.
    (if (call $suffix4 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 118) (i32.const 104) (i32.const 100)) (then (return (i32.const 3))))
    (if (call $suffix5 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 118) (i32.const 104) (i32.const 100) (i32.const 120)) (then (return (i32.const 4))))
    (if (call $suffix5 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 113) (i32.const 99) (i32.const 111) (i32.const 119)) (then (return (i32.const 2))))
    (if (call $suffix6 (local.get $ptr) (local.get $len) (i32.const 46) (i32.const 113) (i32.const 99) (i32.const 111) (i32.const 119) (i32.const 50)) (then (return (i32.const 2))))
    i32.const 1)

  (func $suffix4 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 4)) (then (return (i32.const 0))))
    (i32.and
      (i32.and (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 4))))) (local.get $a))
               (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 3))))) (local.get $b)))
      (i32.and (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 2))))) (local.get $c))
               (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 1))))) (local.get $d)))))

  (func $suffix5 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 5)) (then (return (i32.const 0))))
    (i32.and
      (call $suffix4 (i32.add (local.get $ptr) (i32.const 1)) (i32.sub (local.get $len) (i32.const 1)) (local.get $b) (local.get $c) (local.get $d) (local.get $e))
      (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 5))))) (local.get $a))))

  (func $suffix6 (param $ptr i32) (param $len i32) (param $a i32) (param $b i32) (param $c i32) (param $d i32) (param $e i32) (param $f i32) (result i32)
    (if (i32.lt_u (local.get $len) (i32.const 6)) (then (return (i32.const 0))))
    (i32.and
      (call $suffix5 (i32.add (local.get $ptr) (i32.const 1)) (i32.sub (local.get $len) (i32.const 1)) (local.get $b) (local.get $c) (local.get $d) (local.get $e) (local.get $f))
      (i32.eq (call $m28lower (i32.load8_u (i32.add (local.get $ptr) (i32.sub (local.get $len) (i32.const 6))))) (local.get $a))))

  (import "edgerun" "to_lower" (func $m28lower (param i32) (result i32)))

  (func $authority_disk_format_requires_qemu (export "authority_disk_format_requires_qemu") (param $format i32) (result i32)
    (i32.or (i32.eq (local.get $format) (i32.const 2)) (i32.or (i32.eq (local.get $format) (i32.const 3)) (i32.eq (local.get $format) (i32.const 4)))))

  (func (export "authority_disk_validate_spec_result") (param $path_len i32) (param $size_bytes i32) (param $format i32) (param $qemu_available i32) (result i32)
    ;; 0 ok, 1 invalid argument, 2 command missing.
    (if (i32.eqz (local.get $size_bytes)) (then (return (i32.const 1))))
    (if (i32.eqz (local.get $path_len)) (then (return (i32.const 1))))
    (if (i32.and (call $authority_disk_format_requires_qemu (local.get $format)) (i32.eqz (local.get $qemu_available)))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_disk_qemu_size_unit") (param $size_bytes i32) (result i32)
    ;; 3 GiB, 2 MiB, 1 KiB, 0 raw bytes.
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1073741824))) (then (return (i32.const 3))))
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1048576))) (then (return (i32.const 2))))
    (if (i32.eqz (i32.rem_u (local.get $size_bytes) (i32.const 1024))) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "authority_block_total_size") (param $block_size i32) (param $block_count i32) (result i64)
    (i64.mul (i64.extend_i32_u (local.get $block_size)) (i64.extend_i32_u (local.get $block_count))))

  (func (export "authority_block_transfer_result")
    (param $block_size i32) (param $block_count i32) (param $readonly i32)
    (param $op i32) (param $lba i32) (param $blocks i32) (param $buffer_len i32)
    (result i32)
    ;; op: 1 read, 2 write, 3 discard, 4 write_zeroes. 0 ok, 1 bad device,
    ;; 2 readonly, 3 out of range, 4 length mismatch.
    (if (i32.or (i32.eqz (local.get $block_size)) (i32.eqz (local.get $block_count)))
      (then (return (i32.const 1))))
    (if (i32.eqz (local.get $blocks)) (then (return (i32.const 1))))
    (if (i32.and (local.get $readonly) (i32.or (i32.eq (local.get $op) (i32.const 2)) (i32.or (i32.eq (local.get $op) (i32.const 3)) (i32.eq (local.get $op) (i32.const 4)))))
      (then (return (i32.const 2))))
    (if (i32.gt_u (i32.add (local.get $lba) (local.get $blocks)) (local.get $block_count))
      (then (return (i32.const 3))))
    (if (i32.ne (local.get $buffer_len) (i32.mul (local.get $block_size) (local.get $blocks)))
      (then (return (i32.const 4))))
    i32.const 0)

  (func (export "authority_block_next_request_id") (param $current i32) (result i32)
    (if (result i32) (i32.eq (local.get $current) (i32.const -1))
      (then i32.const -1)
      (else (i32.add (local.get $current) (i32.const 1)))))

  (func (export "authority_file_backend_open_result") (param $exists i32) (param $is_file i32) (param $block_size i32) (param $file_len i32) (result i32)
    ;; 0 ok, 1 invalid argument, 2 misaligned.
    (if (i32.eqz (local.get $block_size)) (then (return (i32.const 1))))
    (if (i32.eqz (i32.and (local.get $exists) (local.get $is_file))) (then (return (i32.const 1))))
    (if (i32.ne (i32.rem_u (local.get $file_len) (local.get $block_size)) (i32.const 0))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "authority_nbd_ioctl_code") (param $which i32) (result i32)
    (if (i32.eq (local.get $which) (i32.const 1)) (then (return (i32.const 0xab00))))
    (if (i32.eq (local.get $which) (i32.const 2)) (then (return (i32.const 0xab01))))
    (if (i32.eq (local.get $which) (i32.const 3)) (then (return (i32.const 0xab03))))
    (if (i32.eq (local.get $which) (i32.const 4)) (then (return (i32.const 0xab04))))
    (if (i32.eq (local.get $which) (i32.const 5)) (then (return (i32.const 0xab05))))
    (if (i32.eq (local.get $which) (i32.const 6)) (then (return (i32.const 0xab07))))
    (if (i32.eq (local.get $which) (i32.const 7)) (then (return (i32.const 0xab08))))
    (if (i32.eq (local.get $which) (i32.const 8)) (then (return (i32.const 0xab0a))))
    i32.const 0)

  (func (export "authority_nbd_attach_result") (param $size_bytes i32) (param $block_size i32) (param $read_only i32) (param $flags i32) (result i32)
    ;; 0 ok, 1 misaligned; read_only ORs the negotiated flags with read-only bit 2.
    (drop (local.get $flags))
    (if (i32.eqz (local.get $block_size)) (then (return (i32.const 1))))
    (if (i32.ne (i32.rem_u (local.get $size_bytes) (local.get $block_size)) (i32.const 0))
      (then (return (i32.const 1))))
    (if (result i32) (local.get $read_only)
      (then i32.const 2)
      (else i32.const 0)))

  (func (export "authority_nbd_len") (param $which i32) (result i32)
    ;; 1 server handshake, 2 client flags, 3 option header, 4 export info,
    ;; 5 request header, 6 reply header, 7 option reply header.
    (if (i32.eq (local.get $which) (i32.const 1)) (then (return (i32.const 18))))
    (if (i32.eq (local.get $which) (i32.const 2)) (then (return (i32.const 4))))
    (if (i32.eq (local.get $which) (i32.const 3)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $which) (i32.const 4)) (then (return (i32.const 134))))
    (if (i32.eq (local.get $which) (i32.const 5)) (then (return (i32.const 28))))
    (if (i32.eq (local.get $which) (i32.const 6)) (then (return (i32.const 16))))
    (if (i32.eq (local.get $which) (i32.const 7)) (then (return (i32.const 20))))
    i32.const 0)
