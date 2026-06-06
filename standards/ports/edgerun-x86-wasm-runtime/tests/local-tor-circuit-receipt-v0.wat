(module
  (memory 1)

  ;; Canonical local Tor circuit receipt record writer.
  ;;
  ;; Input at hashes_ptr is 5 contiguous 32-byte fields:
  ;; app_id, source_event, relay_identity, cell_hash, handshake_transcript.
  ;;
  ;; Output layout:
  ;; u32 magic 'ERTR'
  ;; u32 version 0
  ;; u32 phase_id
  ;; u32 amount
  ;; u32 reserved0
  ;; u32 reserved1
  ;; bytes hashes[5][32]
  (func (export "er_local_tor_circuit_receipt_write")
    (param $out_ptr i32)
    (param $hashes_ptr i32)
    (param $phase_id i32)
    (param $amount i32)
    (result i32)
    (local $i i32)

    local.get $out_ptr
    i32.const 0x45525452
    i32.store

    local.get $out_ptr
    i32.const 4
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $phase_id
    i32.store

    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $amount
    i32.store

    local.get $out_ptr
    i32.const 16
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 20
    i32.add
    i32.const 0
    i32.store

    i32.const 0
    local.set $i
    block $done_hashes
      loop $copy_hashes
        local.get $i
        i32.const 160
        i32.eq
        br_if $done_hashes
        local.get $out_ptr
        i32.const 24
        i32.add
        local.get $i
        i32.add
        local.get $hashes_ptr
        local.get $i
        i32.add
        i32.load8_u
        i32.store8
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $copy_hashes
      end
    end

    i32.const 184)
)
