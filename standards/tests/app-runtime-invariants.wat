(module
  (@custom "er.manifest" "abi=edgerun.app.v0;mode=release;memory.min=65536;storage.min=4096;recursion=false;memory.static=true;storage.direct_access=false")

  (memory 1)

  ;; First WAT-authored app runtime invariant test module. These exports are
  ;; intentionally no-arg so the x86 runtime harness can call them through
  ;; er_fn_run while we stabilize the richer app ABI.

  ;; release identity has no supported memory inspection path.
  (func (export "release_memory_inspect_allowed") (result i32)
    i32.const 0)

  ;; developer identity may expose memory inspection.
  (func (export "developer_memory_inspect_allowed") (result i32)
    i32.const 1)

  ;; app-visible storage writes are intents; direct storage writes are denied.
  (func (export "direct_storage_write_allowed") (result i32)
    i32.const 0)

  ;; recursion is disabled for ordinary app modules.
  (func (export "recursion_allowed") (result i32)
    i32.const 0)

  ;; a committed transition advances an app clock by exactly one logical tick.
  (func (export "commit_tick_delta") (result i32)
    i32.const 1)
)
