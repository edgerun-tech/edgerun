
(func (export "proto_standard_id") (result i32)
    i32.const 300044)

  ;; HPACK dynamic table entry size is name_len + value_len + 32.
  ;; Returns packed low32=status, high32=size.
  (func $hpack_table_entry_size (export "hpack_table_entry_size")
    (param $name_len i32) (param $value_len i32)
    (result i64)
    (local $sum i32)
    (local.set $sum (i32.add (local.get $name_len) (local.get $value_len)))
    (if (i32.lt_u (local.get $sum) (local.get $name_len))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (local.set $sum (i32.add (local.get $sum) (i32.const 32)))
    (if (i32.lt_u (local.get $sum) (i32.const 32))
      (then (return (call $pack (i32.const 4) (i32.const 0)))))
    (call $pack (i32.const 0) (local.get $sum)))

  ;; Insert-plan output record, little-endian:
  ;; 0:u32 new_entry_size
  ;; 4:u32 retained_size
  ;; 8:u32 evict_existing_count
  ;; 12:u32 inserted
  ;;
  ;; oldest_sizes_ptr points to u32 entry sizes in eviction order: oldest first.
  ;; This mirrors Rust VecDeque push_front(new) + pop_back(oldest) behavior,
  ;; while keeping header storage/search in the HTTP runtime.
  (func $hpack_table_insert_plan (export "hpack_table_insert_plan")
    (param $max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $oldest_count i32)
    (param $name_len i32)
    (param $value_len i32)
    (param $out_ptr i32)
    (result i32)
    (local $packed i64)
    (local $entry_size i32)
    (local $retained_size i32)
    (local $evict_count i32)
    (local $old_size i32)
    (local.set $packed
      (call $hpack_table_entry_size (local.get $name_len) (local.get $value_len)))
    (if (i32.ne (i32.wrap_i64 (local.get $packed)) (i32.const 0))
      (then (return (i32.wrap_i64 (local.get $packed)))))
    (local.set $entry_size (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (i32.store (local.get $out_ptr) (local.get $entry_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (i32.const 0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (i32.const 0))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 0))
    (if (i32.gt_u (local.get $entry_size) (local.get $max_size))
      (then
        (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $oldest_count))
        (return (i32.const 0))))
    (local.set $retained_size (i32.add (local.get $current_size) (local.get $entry_size)))
    (if (i32.lt_u (local.get $retained_size) (local.get $current_size))
      (then (return (i32.const 4))))
    (block $done
      (loop $again
        (if (i32.le_u (local.get $retained_size) (local.get $max_size))
          (then (br $done)))
        (if (i32.ge_u (local.get $evict_count) (local.get $oldest_count))
          (then (return (i32.const 3))))
        (local.set $old_size
          (i32.load
            (i32.add
              (local.get $oldest_sizes_ptr)
              (i32.mul (local.get $evict_count) (i32.const 4)))))
        (if (i32.gt_u (local.get $old_size) (local.get $retained_size))
          (then (return (i32.const 3))))
        (local.set $retained_size (i32.sub (local.get $retained_size) (local.get $old_size)))
        (local.set $evict_count (i32.add (local.get $evict_count) (i32.const 1)))
        (br $again)))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $retained_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 8)) (local.get $evict_count))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 12)) (i32.const 1))
    (i32.const 0))

  ;; Resize-plan output record, little-endian:
  ;; 0:u32 retained_size
  ;; 4:u32 evict_existing_count
  ;;
  ;; oldest_sizes_ptr points to u32 entry sizes in eviction order: oldest first.
  (func $hpack_table_resize_plan (export "hpack_table_resize_plan")
    (param $new_max_size i32)
    (param $current_size i32)
    (param $oldest_sizes_ptr i32)
    (param $oldest_count i32)
    (param $out_ptr i32)
    (result i32)
    (local $retained_size i32)
    (local $evict_count i32)
    (local $old_size i32)
    (local.set $retained_size (local.get $current_size))
    (block $done
      (loop $again
        (if (i32.le_u (local.get $retained_size) (local.get $new_max_size))
          (then (br $done)))
        (if (i32.ge_u (local.get $evict_count) (local.get $oldest_count))
          (then (return (i32.const 3))))
        (local.set $old_size
          (i32.load
            (i32.add
              (local.get $oldest_sizes_ptr)
              (i32.mul (local.get $evict_count) (i32.const 4)))))
        (if (i32.gt_u (local.get $old_size) (local.get $retained_size))
          (then (return (i32.const 3))))
        (local.set $retained_size (i32.sub (local.get $retained_size) (local.get $old_size)))
        (local.set $evict_count (i32.add (local.get $evict_count) (i32.const 1)))
        (br $again)))
    (i32.store (local.get $out_ptr) (local.get $retained_size))
    (i32.store (i32.add (local.get $out_ptr) (i32.const 4)) (local.get $evict_count))
    (i32.const 0))
