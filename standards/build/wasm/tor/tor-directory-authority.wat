;; EdgeRun/Tor directory-authority core.
  ;; This module models the local DA from tor-spec/00-os-mapping.md while
  ;; retaining Tor directory semantics: descriptor acceptance, flags, consensus
  ;; route entries, protocol requirements, and descriptor upload paths.
  (global $m7STANDARD_ID i32 (i32.const 300218))
  (global $m7ABI_VERSION i32 (i32.const 2))
  (global $m7OK i32 (i32.const 0))
  (global $m7NOT_FOUND i32 (i32.const 1))
  (global $m7ERR_INVALID i32 (i32.const -1))
  (global $m7ERR_BOUNDS i32 (i32.const -2))
  (global $ERR_DUPLICATE i32 (i32.const -3))
  (global $ERR_REJECT i32 (i32.const -4))

  (global $MAX_DESC i32 (i32.const 128))
  (global $DESC_SIZE i32 (i32.const 128))
  (global $DESC_BASE i32 (i32.const 8192))
  (global $CONS_BASE i32 (i32.const 32768))
  (global $CONS_SIZE i32 (i32.const 96))
  (global $IDENTITY_LEN i32 (i32.const 32))
  (global $NICK_MAX i32 (i32.const 19))
  (global $DESC_MAX_BYTES i32 (i32.const 20000))

  (global $FLAG_VALID i32 (i32.const 1))
  (global $FLAG_RUNNING i32 (i32.const 2))
  (global $FLAG_STABLE i32 (i32.const 4))
  (global $FLAG_FAST i32 (i32.const 8))
  (global $FLAG_GUARD i32 (i32.const 16))
  (global $FLAG_EXIT i32 (i32.const 32))
  (global $FLAG_HSDIR i32 (i32.const 64))
  (global $FLAG_V2DIR i32 (i32.const 128))
  (global $FLAG_AUTHORITY i32 (i32.const 256))
  (global $FLAG_STALE_DESC i32 (i32.const 512))
  (global $FLAG_SYBIL i32 (i32.const 1024))
  (global $FLAG_MIDDLE_ONLY i32 (i32.const 2048))

  (data (i32.const 1024) "/tor/\00")
  (data (i32.const 1040) "/tor/server/\00")
  (data (i32.const 1072) "/tor/status-vote/current/consensus\00")
  (data (i32.const 1120) "/tor/status-vote/next/authority\00")
  (data (i32.const 1168) "/tor/status-vote/current/consensus-microdesc\00")

  ;; Descriptor layout:
  ;; 0 active, 4 flags, 8 bandwidth_kib, 12 uptime_hours, 16 published_minute,
  ;; 20 or_port, 24 dir_port, 28 proto_bits, 32 next_hop,
  ;; 36 identity32, 68 nickname_len, 72 nickname20, 96 desc_len,
  ;; 100 vote_weight, 104 service_bits, 108 reserved.

  (func $desc_ptr (param $idx i32) (result i32)
    (i32.add (global.get $DESC_BASE) (i32.mul (local.get $idx) (global.get $DESC_SIZE))))

  (func $cons_ptr (param $idx i32) (result i32)
    (i32.add (global.get $CONS_BASE) (i32.mul (local.get $idx) (global.get $CONS_SIZE))))

  (func $m7copy (param $dst i32) (param $src i32) (param $len i32)
    (local $i i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (i32.store8 (i32.add (local.get $dst) (local.get $i)) (i32.load8_u (i32.add (local.get $src) (local.get $i))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop))))

  (func $m7mem_eq (param $a i32) (param $b i32) (param $len i32) (result i32)
    (local $i i32) (local $acc i32)
    (local.set $i (i32.const 0))
    (local.set $acc (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (local.set $acc
          (i32.or (local.get $acc)
            (i32.xor
              (i32.load8_u (i32.add (local.get $a) (local.get $i)))
              (i32.load8_u (i32.add (local.get $b) (local.get $i))))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.eqz (local.get $acc)))

  (func $is_nick_char (param $c i32) (result i32)
    (i32.or
      (i32.or
        (i32.and (i32.ge_u (local.get $c) (i32.const 65)) (i32.le_u (local.get $c) (i32.const 90)))
        (i32.and (i32.ge_u (local.get $c) (i32.const 97)) (i32.le_u (local.get $c) (i32.const 122))))
      (i32.and (i32.ge_u (local.get $c) (i32.const 48)) (i32.le_u (local.get $c) (i32.const 57)))))

  (func $valid_nickname (param $ptr i32) (param $len i32) (result i32)
    (local $i i32)
    (if (i32.or (i32.eqz (local.get $len)) (i32.gt_u (local.get $len) (global.get $NICK_MAX))) (then (return (i32.const 0))))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
        (if (i32.eqz (call $is_nick_char (i32.load8_u (i32.add (local.get $ptr) (local.get $i)))))
          (then (return (i32.const 0))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 1))

  (func $find_identity (param $identity32 i32) (result i32)
    (local $i i32) (local $p i32)
    (local.set $i (i32.const 0))
    (block $nf
      (loop $loop
        (br_if $nf (i32.ge_u (local.get $i) (global.get $MAX_DESC)))
        (local.set $p (call $desc_ptr (local.get $i)))
        (if (i32.and (i32.load (local.get $p)) (call $m7mem_eq (i32.add (local.get $p) (i32.const 36)) (local.get $identity32) (global.get $IDENTITY_LEN)))
          (then (return (local.get $p))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 0))

  (func $alloc_desc (result i32)
    (local $i i32) (local $p i32)
    (local.set $i (i32.const 0))
    (block $nf
      (loop $loop
        (br_if $nf (i32.ge_u (local.get $i) (global.get $MAX_DESC)))
        (local.set $p (call $desc_ptr (local.get $i)))
        (if (i32.eqz (i32.load (local.get $p))) (then (return (local.get $p))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (i32.const 0))

  (func $count_addr (param $next_hop i32) (result i32)
    (local $i i32) (local $p i32) (local $n i32)
    (local.set $i (i32.const 0))
    (local.set $n (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (global.get $MAX_DESC)))
        (local.set $p (call $desc_ptr (local.get $i)))
        (if (i32.and (i32.load (local.get $p)) (i32.eq (i32.load offset=32 (local.get $p)) (local.get $next_hop)))
          (then (local.set $n (i32.add (local.get $n) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $n))

  (func $assign_flags
    (param $bandwidth_kib i32) (param $uptime_hours i32) (param $or_port i32) (param $dir_port i32)
    (param $proto_bits i32) (param $next_hop i32) (param $is_authority i32) (result i32)
    (local $flags i32)
    (local.set $flags (global.get $FLAG_VALID))
    (if (local.get $or_port) (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_RUNNING)))))
    (if (i32.ge_u (local.get $uptime_hours) (i32.const 168)) (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_STABLE)))))
    (if (i32.ge_u (local.get $bandwidth_kib) (i32.const 100)) (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_FAST)))))
    (if (i32.and (i32.ne (local.get $or_port) (i32.const 0)) (i32.and (i32.ge_u (local.get $bandwidth_kib) (i32.const 2048)) (i32.ge_u (local.get $uptime_hours) (i32.const 168))))
      (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_GUARD)))))
    (if (i32.and (i32.ne (local.get $dir_port) (i32.const 0)) (i32.ne (i32.and (local.get $proto_bits) (i32.const 0x1)) (i32.const 0)))
      (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_V2DIR)))))
    (if (i32.and (i32.ne (i32.and (local.get $flags) (global.get $FLAG_STABLE)) (i32.const 0)) (i32.ne (i32.and (local.get $flags) (global.get $FLAG_FAST)) (i32.const 0)))
      (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_HSDIR)))))
    (if (i32.and (local.get $proto_bits) (i32.const 0x20)) (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_EXIT)))))
    (if (local.get $is_authority) (then (local.set $flags (i32.or (local.get $flags) (global.get $FLAG_AUTHORITY)))))
    (if (i32.ge_u (call $count_addr (local.get $next_hop)) (i32.const 2))
      (then (local.set $flags (i32.or (i32.and (local.get $flags) (i32.xor (i32.const -1) (i32.or (global.get $FLAG_RUNNING) (global.get $FLAG_VALID)))) (global.get $FLAG_SYBIL)))))
    (local.get $flags))

  (func (export "proto_standard_id") (result i32) (global.get $m7STANDARD_ID))
  (func (export "proto_abi_version") (result i32) (global.get $m7ABI_VERSION))
  (func (export "simd_capabilities") (result i32) (i32.const 1))
  (func (export "tor_da_max_descriptors") (result i32) (global.get $MAX_DESC))
  (func (export "tor_da_descriptor_size") (result i32) (global.get $DESC_SIZE))
  (func (export "tor_da_consensus_entry_size") (result i32) (global.get $CONS_SIZE))

  (func (export "tor_da_init") (result i32)
    (memory.fill (global.get $DESC_BASE) (i32.const 0) (i32.mul (global.get $MAX_DESC) (global.get $DESC_SIZE)))
    (memory.fill (global.get $CONS_BASE) (i32.const 0) (i32.mul (global.get $MAX_DESC) (global.get $CONS_SIZE)))
    (global.get $m7OK))

  (func (export "tor_da_descriptor_upload_path") (param $out i32) (result i32)
    (call $m7copy (local.get $out) (i32.const 1040) (i32.const 12))
    (i32.const 12))

  (func (export "tor_da_consensus_path") (param $out i32) (param $microdesc i32) (result i32)
    (if (local.get $microdesc)
      (then
        (call $m7copy (local.get $out) (i32.const 1168) (i32.const 44))
        (return (i32.const 44))))
    (call $m7copy (local.get $out) (i32.const 1072) (i32.const 34))
    (i32.const 34))

  (func (export "tor_da_vote_path") (param $out i32) (result i32)
    (call $m7copy (local.get $out) (i32.const 1120) (i32.const 31))
    (i32.const 31))

  (func (export "tor_da_accept_descriptor")
    (param $identity32 i32) (param $nickname i32) (param $nickname_len i32)
    (param $bandwidth_kib i32) (param $uptime_hours i32) (param $published_minute i32)
    (param $or_port i32) (param $dir_port i32) (param $proto_bits i32)
    (param $next_hop i32) (param $desc_len i32) (param $is_authority i32) (result i32)
    (local $p i32) (local $old_pub i32) (local $flags i32)
    (if (i32.gt_u (local.get $desc_len) (global.get $DESC_MAX_BYTES)) (then (return (global.get $m7ERR_BOUNDS))))
    (if (i32.eqz (call $valid_nickname (local.get $nickname) (local.get $nickname_len))) (then (return (global.get $m7ERR_INVALID))))
    (if (i32.eqz (i32.or (local.get $or_port) (local.get $dir_port))) (then (return (global.get $ERR_REJECT))))
    (local.set $p (call $find_identity (local.get $identity32)))
    (if (local.get $p)
      (then
        (local.set $old_pub (i32.load offset=16 (local.get $p)))
        (if (i32.lt_u (local.get $published_minute) (local.get $old_pub)) (then (return (global.get $ERR_DUPLICATE)))))
      (else
        (local.set $p (call $alloc_desc))
        (if (i32.eqz (local.get $p)) (then (return (global.get $m7ERR_BOUNDS))))))
    (local.set $flags (call $assign_flags (local.get $bandwidth_kib) (local.get $uptime_hours) (local.get $or_port) (local.get $dir_port) (local.get $proto_bits) (local.get $next_hop) (local.get $is_authority)))
    (i32.store (local.get $p) (i32.const 1))
    (i32.store offset=4 (local.get $p) (local.get $flags))
    (i32.store offset=8 (local.get $p) (local.get $bandwidth_kib))
    (i32.store offset=12 (local.get $p) (local.get $uptime_hours))
    (i32.store offset=16 (local.get $p) (local.get $published_minute))
    (i32.store offset=20 (local.get $p) (local.get $or_port))
    (i32.store offset=24 (local.get $p) (local.get $dir_port))
    (i32.store offset=28 (local.get $p) (local.get $proto_bits))
    (i32.store offset=32 (local.get $p) (local.get $next_hop))
    (call $m7copy (i32.add (local.get $p) (i32.const 36)) (local.get $identity32) (global.get $IDENTITY_LEN))
    (i32.store offset=68 (local.get $p) (local.get $nickname_len))
    (memory.fill (i32.add (local.get $p) (i32.const 72)) (i32.const 0) (i32.const 20))
    (call $m7copy (i32.add (local.get $p) (i32.const 72)) (local.get $nickname) (local.get $nickname_len))
    (i32.store offset=96 (local.get $p) (local.get $desc_len))
    (i32.store offset=100 (local.get $p) (select (local.get $bandwidth_kib) (i32.const 1) (i32.gt_u (local.get $bandwidth_kib) (i32.const 1))))
    (i32.store offset=104 (local.get $p) (i32.and (local.get $proto_bits) (i32.const 0xff)))
    (global.get $m7OK))

  (func (export "tor_da_descriptor_ptr") (param $identity32 i32) (result i32)
    (call $find_identity (local.get $identity32)))

  (func (export "tor_da_descriptor_count") (result i32)
    (local $i i32) (local $p i32) (local $n i32)
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (global.get $MAX_DESC)))
        (local.set $p (call $desc_ptr (local.get $i)))
        (if (i32.load (local.get $p)) (then (local.set $n (i32.add (local.get $n) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $n))

  (func (export "tor_da_required_protocols_ok") (param $proto_bits i32) (param $required_bits i32) (result i32)
    (i32.eq (i32.and (local.get $proto_bits) (local.get $required_bits)) (local.get $required_bits)))

  (func (export "tor_da_build_consensus") (param $valid_after_minute i32) (param $fresh_until_minute i32) (param $valid_until_minute i32) (result i32)
    (local $i i32) (local $p i32) (local $c i32) (local $n i32)
    (memory.fill (global.get $CONS_BASE) (i32.const 0) (i32.mul (global.get $MAX_DESC) (global.get $CONS_SIZE)))
    (local.set $i (i32.const 0))
    (block $done
      (loop $loop
        (br_if $done (i32.ge_u (local.get $i) (global.get $MAX_DESC)))
        (local.set $p (call $desc_ptr (local.get $i)))
        (if (i32.and (i32.load (local.get $p)) (i32.and (i32.load offset=4 (local.get $p)) (global.get $FLAG_VALID)))
          (then
            (local.set $c (call $cons_ptr (local.get $n)))
            (call $m7copy (local.get $c) (i32.add (local.get $p) (i32.const 36)) (i32.const 32))
            (i32.store offset=32 (local.get $c) (i32.load offset=32 (local.get $p)))
            (i32.store offset=36 (local.get $c) (i32.load offset=4 (local.get $p)))
            (i32.store offset=40 (local.get $c) (i32.load offset=100 (local.get $p)))
            (i32.store offset=44 (local.get $c) (local.get $valid_after_minute))
            (i32.store offset=48 (local.get $c) (local.get $fresh_until_minute))
            (i32.store offset=52 (local.get $c) (local.get $valid_until_minute))
            (i32.store offset=56 (local.get $c) (i32.load offset=28 (local.get $p)))
            (i32.store offset=60 (local.get $c) (i32.load offset=20 (local.get $p)))
            (i32.store offset=64 (local.get $c) (i32.load offset=24 (local.get $p)))
            (call $m7copy (i32.add (local.get $c) (i32.const 68)) (i32.add (local.get $p) (i32.const 72)) (i32.const 20))
            (local.set $n (i32.add (local.get $n) (i32.const 1)))))
        (local.set $i (i32.add (local.get $i) (i32.const 1)))
        (br $loop)))
    (local.get $n))

  (func (export "tor_da_consensus_entry_ptr") (param $index i32) (result i32)
    (if (i32.ge_u (local.get $index) (global.get $MAX_DESC)) (then (return (i32.const 0))))
    (call $cons_ptr (local.get $index)))

  (func (export "tor_da_route_lookup") (param $identity32 i32) (param $out_next_hop i32) (result i32)
    (local $p i32)
    (local.set $p (call $find_identity (local.get $identity32)))
    (if (i32.eqz (local.get $p)) (then (return (global.get $m7NOT_FOUND))))
    (if (i32.eqz (i32.and (i32.load offset=4 (local.get $p)) (global.get $FLAG_VALID))) (then (return (global.get $ERR_REJECT))))
    (i32.store (local.get $out_next_hop) (i32.load offset=32 (local.get $p)))
    (global.get $m7OK))