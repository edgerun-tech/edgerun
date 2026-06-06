;; Utility compatibility semantics plundered from crates/utility.

  (func (export "utility_compat_abi_version") (result i32) i32.const 1)

  (func (export "arrayvec_push_result") (param $len i32) (param $cap i32) (result i32)
    ;; 0 ok, 1 capacity exceeded.
    (if (i32.ge_u (local.get $len) (local.get $cap)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "arrayvec_len_after_push") (param $len i32) (param $cap i32) (result i32)
    (if (i32.ge_u (local.get $len) (local.get $cap)) (then (return (local.get $len))))
    (i32.add (local.get $len) (i32.const 1)))

  (func (export "arrayvec_len_after_pop") (param $len i32) (result i32)
    (if (i32.eqz (local.get $len)) (then (return (i32.const 0))))
    (i32.sub (local.get $len) (i32.const 1)))

  (func (export "arrayvec_len_after_truncate") (param $len i32) (param $new_len i32) (result i32)
    (if (i32.ge_u (local.get $new_len) (local.get $len)) (then (return (local.get $len))))
    local.get $new_len)

  (func (export "arrayvec_remaining_capacity") (param $len i32) (param $cap i32) (result i32)
    (if (i32.gt_u (local.get $len) (local.get $cap)) (then (return (i32.const 0))))
    (i32.sub (local.get $cap) (local.get $len)))

  (func (export "bytes_slice_result") (param $len i32) (param $start i32) (param $end i32) (result i32)
    ;; 0 ok, 1 invalid bounds. Rust slice panics if start/end are invalid.
    (if (i32.gt_u (local.get $start) (local.get $end)) (then (return (i32.const 1))))
    (if (i32.gt_u (local.get $end) (local.get $len)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "bytes_split_to_result") (param $len i32) (param $at i32) (result i32)
    ;; 0 ok, 1 out of bounds.
    (if (i32.gt_u (local.get $at) (local.get $len)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "bytes_len_after_advance") (param $len i32) (param $cnt i32) (result i32)
    ;; advance keeps bytes[cnt..]; invalid cnt is a panic surface, represented as 0 remaining here.
    (if (i32.gt_u (local.get $cnt) (local.get $len)) (then (return (i32.const 0))))
    (i32.sub (local.get $len) (local.get $cnt)))

  (func (export "log_level_name_code") (param $level i32) (result i32)
    ;; trace/debug/info/warn/error are 0..4.
    (if (i32.le_u (local.get $level) (i32.const 4)) (then (return (i32.add (local.get $level) (i32.const 1)))))
    i32.const 5)

  (func (export "log_enabled") (param $message_level i32) (param $configured_level i32) (result i32)
    ;; enabled when message severity ordinal is >= configured threshold.
    (if (i32.ge_u (local.get $message_level) (local.get $configured_level)) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "log_dispatch_path") (param $enabled i32) (param $has_format_logger i32) (param $has_plain_logger i32) (result i32)
    ;; 0 dropped, 1 format logger, 2 fixed-buffer plain logger.
    (if (i32.eqz (local.get $enabled)) (then (return (i32.const 0))))
    (if (local.get $has_format_logger) (then (return (i32.const 1))))
    (if (local.get $has_plain_logger) (then (return (i32.const 2))))
    i32.const 0)

  (func (export "try_lock_result") (param $was_locked i32) (result i32)
    ;; swap(true): succeeds only from unlocked state.
    (if (local.get $was_locked) (then (return (i32.const 1))))
    i32.const 0)

  (func (export "try_lock_ordering_result") (param $lock_order i32) (param $unlock_order i32) (result i32)
    ;; accepted lock order: Acquire=2, Release=3, AcqRel=4, SeqCst=5 in Rust Ordering order;
    ;; accepted unlock order: Release or SeqCst.
    (if (i32.eqz
        (i32.or (i32.eq (local.get $lock_order) (i32.const 2))
          (i32.or (i32.eq (local.get $lock_order) (i32.const 4)) (i32.eq (local.get $lock_order) (i32.const 5)))))
      (then (return (i32.const 1))))
    (if (i32.eqz (i32.or (i32.eq (local.get $unlock_order) (i32.const 3)) (i32.eq (local.get $unlock_order) (i32.const 5))))
      (then (return (i32.const 2))))
    i32.const 0)

  (func (export "async_channel_try_recv_result") (param $queue_len i32) (param $disconnected i32) (result i32)
    ;; 0 value, 1 empty, 2 disconnected.
    (if (i32.gt_u (local.get $queue_len) (i32.const 0)) (then (return (i32.const 0))))
    (if (local.get $disconnected) (then (return (i32.const 2))))
    i32.const 1)

  (func (export "async_channel_bounded_capacity") (param $cap i32) (result i32)
    local.get $cap)

  (func (export "async_channel_unbounded_capacity_code") (result i32)
    ;; This local wrapper maps unbounded to mpsc::channel(0).
    i32.const 0)

  (func (export "error_repr_display_code") (param $repr i32) (result i32)
    ;; 1 message, 2 source, 3 context:source, 0 unknown.
    (if (i32.and (i32.ge_u (local.get $repr) (i32.const 1)) (i32.le_u (local.get $repr) (i32.const 3))) (then (return (local.get $repr))))
    i32.const 0)

  (func (export "error_chain_next_result") (param $has_current i32) (param $current_has_source i32) (result i32)
    ;; 0 no item, 1 yielded and chain continues, 2 yielded final item.
    (if (i32.eqz (local.get $has_current)) (then (return (i32.const 0))))
    (if (local.get $current_has_source) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "ensure_result") (param $condition i32) (result i32)
    ;; ensure! bails when condition is false.
    (if (local.get $condition) (then (return (i32.const 0))))
    i32.const 1)

  (func (export "lazy_static_state_result") (param $initialized i32) (param $initializer_panicked i32) (result i32)
    ;; 0 already initialized/usable, 1 initialize now, 2 poisoned by panic.
    (if (local.get $initialized) (then (return (i32.const 0))))
    (if (local.get $initializer_panicked) (then (return (i32.const 2))))
    i32.const 1)

  (func (export "time_target_code") (param $is_wasm_unknown i32) (result i32)
    ;; 1 browser Performance/Date-backed time, 2 std::time re-export.
    (if (local.get $is_wasm_unknown) (then (return (i32.const 1))))
    i32.const 2)

  (func (export "derive_macro_kind_code") (param $kind i32) (result i32)
    ;; futures/select/join/error/strum/async_trait/cfg_if/unit/wasm_bindgen/tokio are 1..10.
    (if (i32.and (i32.ge_u (local.get $kind) (i32.const 1)) (i32.le_u (local.get $kind) (i32.const 10))) (then (return (local.get $kind))))
    i32.const 0))
