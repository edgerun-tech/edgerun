(module
  (memory 1)

  ;; Canonical local Tor cell receipt record writer.
  ;;
  ;; Input at hashes_ptr is 8 contiguous 32-byte fields:
  ;; app_id, source_event, payload_hash, onion_key_identity,
  ;; client_ephemeral, server_ephemeral, handshake_transcript, relay_body.
  ;;
  ;; Output layout:
  ;; u32 magic 'ERCR'
  ;; u32 version 0
  ;; u32 circuit_id
  ;; u32 sequence
  ;; u32 command
  ;; u32 relay_command
  ;; u32 stream_id
  ;; u32 relay_length
  ;; u32 handshake_type
  ;; u32 plaintext_private_bytes
  ;; u32 private_key_export_count
  ;; u32 reserved
  ;; bytes hashes[8][32]
  (func (export "er_local_tor_cell_record_write")
    (param $out_ptr i32)
    (param $hashes_ptr i32)
    (param $circuit_id i32)
    (param $sequence i32)
    (param $command i32)
    (param $relay_command i32)
    (param $stream_id i32)
    (param $relay_length i32)
    (param $handshake_type i32)
    (result i32)
    (local $i i32)

    local.get $out_ptr
    i32.const 0x45524352
    i32.store

    local.get $out_ptr
    i32.const 4
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 8
    i32.add
    local.get $circuit_id
    i32.store

    local.get $out_ptr
    i32.const 12
    i32.add
    local.get $sequence
    i32.store

    local.get $out_ptr
    i32.const 16
    i32.add
    local.get $command
    i32.store

    local.get $out_ptr
    i32.const 20
    i32.add
    local.get $relay_command
    i32.store

    local.get $out_ptr
    i32.const 24
    i32.add
    local.get $stream_id
    i32.store

    local.get $out_ptr
    i32.const 28
    i32.add
    local.get $relay_length
    i32.store

    local.get $out_ptr
    i32.const 32
    i32.add
    local.get $handshake_type
    i32.store

    local.get $out_ptr
    i32.const 36
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 40
    i32.add
    i32.const 0
    i32.store

    local.get $out_ptr
    i32.const 44
    i32.add
    i32.const 0
    i32.store

    i32.const 0
    local.set $i
    block $done_hashes
      loop $copy_hashes
        local.get $i
        i32.const 256
        i32.eq
        br_if $done_hashes
        local.get $out_ptr
        i32.const 48
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

    i32.const 304)
)
